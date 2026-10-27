#!/usr/bin/env python3
"""Real production SDK JSONL pipes, synthetic host only. No platform services."""
import json, os, pathlib, select, subprocess, sys, tempfile, time, uuid
ROOT=pathlib.Path(__file__).resolve().parents[1]
QA=ROOT/'.qa'
QA.mkdir(exist_ok=True)
class Pipe:
 def __init__(self,binary,profile=None):
  self.dir=pathlib.Path(tempfile.mkdtemp(prefix='pipe-',dir=QA))
  self.profile=pathlib.Path(profile) if profile else self.dir/'data'
  self.profile.mkdir(exist_ok=True)
  self.tmp=self.dir/'tmp';self.tmp.mkdir()
  env=dict(os.environ,HOME=str(self.dir),TMPDIR=str(self.tmp),USERPROFILE=str(self.dir))
  self.p=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,bufsize=0)
  self.buf=b'';self.n=0;self.queue=[];self.methods=[]
  hello=self.recv();assert hello['type']=='hello' and hello['id']=='com.tezbar.native.clipboard-sequential-paste' and hello['sdkVersion']=='0.1.1',hello
  i=self.req('initialize',dict(protocol=1,extensionId=hello['id'],platform='macOS',packageDir=str(self.dir),dataDir=str(self.profile),tempDir=str(self.tmp),grantedCapabilities=['host.clipboard.read','host.clipboard.pasteText','host.window.visibility'],reason='qa',migrationPaths={}))
  assert self.response(i)['result']['ready'] is True
 def send(self,v):self.p.stdin.write((json.dumps(v,ensure_ascii=False)+'\n').encode());self.p.stdin.flush()
 def req(self,method,params):
  self.n+=1;i='h:'+str(uuid.uuid4());self.send(dict(type='request',id=i,method=method,params=params));return i
 def recv(self,timeout=15):
  deadline=time.monotonic()+timeout
  while b'\n' not in self.buf:
   left=deadline-time.monotonic();assert left>0,'pipe timeout'
   ready,_,_=select.select([self.p.stdout],[],[],left);assert ready,'pipe timeout'
   b=os.read(self.p.stdout.fileno(),65536);assert b,'unexpected EOF';self.buf+=b
  line,self.buf=self.buf.split(b'\n',1);return json.loads(line)
 def response(self,i):
  for n,v in enumerate(self.queue):
   if v.get('type')=='response' and v.get('id')==i:return self.queue.pop(n)
  while True:
   v=self.recv()
   if v.get('type')=='response' and v.get('id')==i:return v
   assert v.get('type')!='request',('unexpected reverse RPC',v)
   self.queue.append(v)
 def reverse(self,method):
  while True:
   v=self.recv()
   if v.get('type')=='request':
    assert v['method']==method,v;self.methods.append(method);return v
   self.queue.append(v)
 def reply(self,v,result=None,error=None):
  self.send(dict(type='response',id=v['id'],**({'error':dict(code='qa',message=error)} if error else {'result':result})))
 def open(self,s):return self.req('session.open',dict(sessionId=s,commandName='paste-next-item',preferences={},argumentValues={}))
 def action(self,s,id):return self.req('session.call',dict(sessionId=s,operation='extension:invoke-action',payload={'actionId':id}))
 def dispose(self,s):assert 'result' in self.response(self.req('session.dispose',{'sessionId':s}))
 def checkpoint(self):return json.loads((self.profile/'clipboard-sequential-paste-v2/queue.json').read_text())
 def paste(self,s,clipboard,expected):
  i=self.open(s);v=self.reverse('host.clipboard.read');assert v['params']=={};self.reply(v,clipboard)
  v=self.reverse('host.clipboard.pasteText');assert v['params']=={'text':expected},v
  q=self.checkpoint();assert q['pending']['text']==expected and q['active']
  self.reply(v,{'ok':True,'message':'Pasted text'});r=self.response(i)['result'];assert r['mode']=='no-view',r
  assert self.checkpoint()['pending'] is None
  self.dispose(s)
 def close(self):
  if self.p.poll() is None:
   self.p.stdin.close();self.p.wait(timeout=5)
  assert self.p.stderr.read()==b'', 'production stderr must not contain clipboard data'
  assert self.p.returncode==0,self.p.returncode

def run(binary):
 count=0
 p=Pipe(binary)
 p.paste('one','a\r\n\r\nb\n','a');time.sleep(.66)
 p.paste('two','a','');time.sleep(.66)
 p.paste('three','','b');time.sleep(.66)
 p.paste('four','b','');count+=4
 # SDK no-view lifecycle never fills its 128 session slots.
 for n in range(140):
  i=p.open('debounce-'+str(n));v=p.reverse('host.window.visibility');assert v['params']=={'visible':False};p.reply(v,None);r=p.response(i)['result'];assert r['mode']=='no-view' and 'Debounced' in r['message'];p.dispose('debounce-'+str(n))
 profile=p.profile;p.close()
 time.sleep(.66)
 p=Pipe(binary,profile)
 i=p.open('restart');v=p.reverse('host.clipboard.read');p.reply(v,'');r=p.response(i)['result'];assert r['mode']=='view'
 # Explicit continue after restart starts source global index at zero.
 i=p.action('restart',r['actions'][0]['id']);v=p.reverse('host.clipboard.read');p.reply(v,'')
 v=p.reverse('host.clipboard.pasteText');assert v['params']['text']=='a';p.reply(v,{'ok':True});assert p.response(i)['result']['mode']=='no-view';p.dispose('restart');p.close();count+=2
 # Two opens arrive while the first effect is blocked; time in queue is not a hotkey retry.
 p=Pipe(binary);i=p.open('first');read=p.reverse('host.clipboard.read');j=p.open('queued');time.sleep(.75);p.reply(read,'a\nb')
 v=p.reverse('host.clipboard.pasteText');p.reply(v,{'ok':True});assert p.response(i)['result']['mode']=='no-view';v=p.reverse('host.window.visibility');p.reply(v,None);assert 'Debounced' in p.response(j)['result']['message'];p.dispose('first');p.dispose('queued');p.close();count+=1
 # Cancellation AFTER dispatch. Late ACK cannot advance checkpoint. No automatic replay.
 p=Pipe(binary);i=p.open('cancel');v=p.reverse('host.clipboard.read');p.reply(v,'α\r\nb');effect=p.reverse('host.clipboard.pasteText')
 assert p.checkpoint()['pending']['text']=='α'
 p.send(dict(type='cancel',requestId=i,reason='qa cancel'));assert 'error' in p.response(i)
 p.reply(effect,{'ok':True});i=p.open('recover');r=p.response(i)['result'];assert r['mode']=='view'
 retry=r['actions'][0]['id'];advance=r['actions'][1]['id']
 i=p.action('recover',retry);v=p.reverse('host.clipboard.pasteText');assert v['params']['text']=='α'
 assert p.checkpoint()['pending']['text']=='α'
 p.reply(v,{'ok':True});assert p.response(i)['result']['mode']=='no-view'
 stale=p.action('recover',advance);assert p.response(stale)['result']['mode']=='view';assert p.checkpoint()['next']==1
 p.dispose('recover');p.close();count+=1
 # Reverse timeout is unknown; keep checkpoint and show explicit recovery.
 p=Pipe(binary);i=p.open('timeout');v=p.reverse('host.clipboard.read');p.reply(v,'a\nb');v=p.reverse('host.clipboard.pasteText')
 r=p.response(i)['result'];assert r['mode']=='view' and p.checkpoint()['pending']['text']=='a'
 p.reply(v,{'ok':True});p.dispose('timeout');profile=p.profile;p.close()
 p=Pipe(binary,profile);i=p.open('pending-restart');r=p.response(i)['result'];assert r['mode']=='view'
 i=p.action('pending-restart',r['actions'][1]['id']);v=p.reverse('host.window.visibility');p.reply(v,None);assert p.response(i)['result']['mode']=='no-view';assert p.checkpoint()['next']==1;p.dispose('pending-restart');p.close();count+=2
 # Write fails before effect dispatch: preserve bounded admission record, no paste.
 p=Pipe(binary);i=p.open('write-before');v=p.reverse('host.clipboard.read')
 qpath=p.profile/'clipboard-sequential-paste-v2/queue.json';backup=qpath.read_bytes();qpath.unlink();qpath.mkdir();p.reply(v,'a\nb')
 r=p.response(i)['result'];assert r['mode']=='view';assert p.methods==['host.clipboard.read'];qpath.rmdir();qpath.write_bytes(backup);p.dispose('write-before');p.close();count+=1
 # ACK then failed completion persistence: pending remains authoritative.
 p=Pipe(binary);i=p.open('write-after');v=p.reverse('host.clipboard.read');p.reply(v,'a\nb');v=p.reverse('host.clipboard.pasteText')
 qpath=p.profile/'clipboard-sequential-paste-v2/queue.json';backup=qpath.read_bytes();qpath.unlink();qpath.mkdir();p.reply(v,{'ok':True})
 r=p.response(i)['result'];assert r['mode']=='view' and 'completion' in r['root']['props']['markdown'];qpath.rmdir();qpath.write_bytes(backup);p.dispose('write-after')
 i=p.open('reopen');assert p.response(i)['result']['mode']=='view';assert p.checkpoint()['pending']['text']=='a';p.dispose('reopen');p.close();count+=1
 # EOF during paste then restart has no effect replay.
 p=Pipe(binary);i=p.open('eof');v=p.reverse('host.clipboard.read');p.reply(v,'a\nb');v=p.reverse('host.clipboard.pasteText');profile=p.profile;p.close()
 p=Pipe(binary,profile);i=p.open('eof-restart');r=p.response(i)['result'];assert r['mode']=='view';assert p.methods==[];p.dispose('eof-restart');p.close();count+=1
 # Isolated new profile has no old queue. Malformed ACKs always retain pending.
 for bad in [None,True,{'ok':True,'message':None},{'ok':True,'error':'contradiction'},{'ok':False},{'ok':True,'extra':1}]:
  p=Pipe(binary);assert not (p.profile/'clipboard-sequential-paste-v2').exists();i=p.open('bad');v=p.reverse('host.clipboard.read');p.reply(v,'a\nb');v=p.reverse('host.clipboard.pasteText');p.reply(v,bad)
  r=p.response(i)['result'];assert r['mode']=='view' and p.checkpoint()['pending']['text']=='a';p.dispose('bad');p.close();count+=1
 # Clipboard read error and corrupt store produce actionable view without wiping.
 p=Pipe(binary);i=p.open('read-error');v=p.reverse('host.clipboard.read');p.reply(v,error='synthetic failure');assert p.response(i)['result']['mode']=='view';p.dispose('read-error')
 qpath=p.profile/'clipboard-sequential-paste-v2/queue.json';qpath.write_bytes(b'{corrupt');i=p.open('corrupt');r=p.response(i)['result'];assert r['mode']=='view' and 'repair offline' in r['root']['props']['markdown'];assert qpath.read_bytes()==b'{corrupt';p.dispose('corrupt');p.close();count+=2
 print(f'production JSONL pipes: {count} scenario checks + 140 no-view dispose/reopens passed ({binary.name})')
if __name__=='__main__':run(pathlib.Path(sys.argv[1]).resolve())
