const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
const Ajv=require(root+'/.qa/schema/node_modules/ajv/dist/2020');
const formats=require(root+'/.qa/schema/node_modules/ajv-formats');
const ajv=new Ajv({strict:false,allErrors:true});formats(ajv);
const validate=ajv.compile(JSON.parse(fs.readFileSync(root+'/fixtures/host/schemas/extension-v3.schema.json')));
const m=JSON.parse(fs.readFileSync(root+'/extension.json'));assert(validate(m),JSON.stringify(validate.errors));
const negatives=[x=>x.schemaVersion=2,x=>x.runtime='javascript',x=>x.execution='embedded',x=>x.protocolVersion=2,x=>x.id='com.tezbar.clipboard-sequential-paste',x=>delete x.commands,x=>x.capabilities.push('host.clipboard.compareAndSwap'),x=>x.commands[0].refreshMs=0,x=>x.commands[0].timeoutMs=300001,x=>x.commands[0].concurrency='parallel',x=>x.repository='invalid url',x=>x.artifacts.invalid={},x=>x.files={'../secret':'0'.repeat(64)},x=>x.artifacts={'aarch64-apple-darwin':{url:'https://example.com/test.zip',sha256:'fake',executable:'../bad',executableSha256:'0'.repeat(64)}}];
for(const mutate of negatives){const x=structuredClone(m);mutate(x);assert(!validate(x),'invalid manifest accepted');}
assert.deepEqual(m.replaces,['com.tezbar.clipboard-sequential-paste','raycast.clipboard-sequential-paste']);assert.deepEqual(m.capabilities,['host.clipboard.read','host.clipboard.pasteText','host.window.visibility']);assert.equal(m.commands.length,1);assert.equal(m.commands[0].refreshMs,null);assert.deepEqual(m.artifacts,{});assert(m.verification.every(v=>v.state==='not-run'));
const types=fs.readFileSync(root+'/fixtures/host/src/shared/extensionRuntime.ts','utf8');assert(types.includes("kind?: 'action'"));
assert(fs.readFileSync(root+'/src/lib.rs','utf8').includes('"kind":"action"'));
console.log(`Actual pinned draft2020 schema: source manifest valid; ${negatives.length} negative mutations rejected; exact command/capability/action contract passed`);
