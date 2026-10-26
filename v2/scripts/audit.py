#!/usr/bin/env python3
"""Byte audit, immutable fixture hashes, and whole Git-visible file inventory."""
import hashlib,json,pathlib,subprocess,sys
root=pathlib.Path(__file__).resolve().parents[2];v2=root/'v2'
def git(*a):return subprocess.check_output(['git',*a],cwd=root)
base='dd58c5e7b4c1213abd645ed69f30c0479b6f56f0'
assert git('rev-parse','HEAD').decode().strip()==base
assert git('branch','--show-current').decode().strip()=='feat/native-v2-rust'
assert git('diff','--cached')==b''
for p in git('ls-tree','-r','--name-only',base).decode().splitlines():
 expected=git('show',base+':'+p);actual=(root/p).read_bytes()
 if p=='README.md':assert actual==expected+b'\n[Standalone Rust V2 (development)](v2/README.md).\n'
 else:assert actual==expected,('V1 modified',p)
 assert (v2/'fixtures/own'/p).read_bytes()==expected
icon=v2/'resources/extension-icon.png'
assert len(icon.read_bytes())==24883
assert hashlib.sha256(icon.read_bytes()).hexdigest()=='fcffd51e4ec206a3684a2f6bcabc7b4082964942a14fa97983b1f9c293360368'
# Fixtures are frozen source evidence, never generated build outputs.
fixtures={str(p.relative_to(v2)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((v2/'fixtures').rglob('*')) if p.is_file()}
fhash=v2/'FIXTURES.json'
if '--write' in sys.argv:fhash.write_text(json.dumps(fixtures,indent=2)+'\n')
else:assert json.loads(fhash.read_text())==fixtures
paths=git('ls-files','--cached','--others','--exclude-standard','-z').decode().split('\0')
paths=sorted(set(p for p in paths if p and p!='v2/INDEX.sha256'))
assert all(not p.startswith('v2/.qa/') and not p.startswith('v2/target/') for p in paths)
assert all(p.startswith('v2/') or p in git('ls-tree','-r','--name-only',base).decode().splitlines() for p in paths)
index=''.join(hashlib.sha256((root/p).read_bytes()).hexdigest()+'  '+p+'\n' for p in paths)
if '--write' in sys.argv:(v2/'INDEX.sha256').write_text(index)
else:assert (v2/'INDEX.sha256').read_text()==index
print(f'V1 byte audit passed (README discovery link only); HEAD unchanged; index empty; {len(fixtures)} immutable fixture hashes; {len(paths)} Git-visible inventory files (INDEX excludes itself)')
