// Executes the immutable own TypeScript command via its EXACT locked compiler.
// Only mocked Raycast calls and a controlled Date.now are available in the VM.
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const ts = require(path.join(root, '.qa/v1/node_modules/typescript'));
const source = fs.readFileSync(path.join(root, 'fixtures/own/src/paste-next-item.ts'), 'utf8');
const js = ts.transpileModule(source, {compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
const scenarios = [
 {name:'crlf-empty-trailing-wrap',steps:[{at:10000,copy:'a\r\n\r\nb\n'},{at:10650},{at:11300},{at:11950},{at:12600},{at:13250}]},
 {name:'debounce-boundary',steps:[{at:10000,copy:'a\nb\nc'},{at:10649},{at:10650},{at:10651},{at:11300}]},
 {name:'new-list-global-index',steps:[{at:10000,copy:'a\nb\nc'},{at:10650},{at:11300,copy:'x\ny\nz'},{at:11950},{at:12600,copy:'short'}]},
 {name:'restart',steps:[{at:10000,copy:'a\nb\nc'},{at:10650,restart:true},{at:11300},{at:11950,restart:true}]},
 {name:'empty-and-lone-cr',steps:[{at:10000,copy:''},{at:10650,copy:'a\rb\n\r'},{at:11300}]},
 {name:'failed-paste',steps:[{at:10000,copy:'a\nb',fail:true},{at:10650}]},
 {name:'clock-rollback-source',steps:[{at:10000,copy:'a\nb'},{at:9000},{at:10650}]},
 {name:'host-writes-source-breakage',writes:true,steps:[{at:10000,copy:'a\nb\nc'},{at:10650},{at:11300}]},
];
async function trace(s) {
 let store = {}, clipboard = '', at, calls, fail;
 function fresh() {
  const context = vm.createContext({exports:{},require(name){assert.equal(name,'@raycast/api');return {
   Clipboard:{async readText(){calls.push(['read']);return clipboard;},async paste(text){calls.push(['paste',text===undefined?{undefined:true}:text]);if(fail)throw Error('synthetic paste failure');if(s.writes)clipboard=String(text);}},
   LocalStorage:{async getItem(k){return store[k];},async setItem(k,v){store[k]=v;},async removeItem(k){delete store[k];}},
   async closeMainWindow(){calls.push(['close']);},async showToast(v){calls.push(['toast',v.title]);},Toast:{Style:{Success:'success'}}
  };},Date:{now:()=>at}}, {codeGeneration:{strings:false,wasm:false}});
  vm.runInContext(js+'\nglobalThis.snapshot = () => ({list:[...list],index});',context,{timeout:1000});
  // No unmocked Node/browser/clipboard entry points.
  assert.equal(vm.runInContext('typeof process+":"+typeof fetch+":"+typeof window',context),'undefined:undefined:undefined');
  return context;
 }
 let c=fresh(),out=[];
 for(const step of s.steps){at=step.at;calls=[];fail=step.fail;if('copy'in step)clipboard=step.copy;if(step.restart)c=fresh();let error=null;try{await c.exports.default();}catch(e){error=e.message;}
  out.push({step,calls,store:structuredClone(store),globals:JSON.parse(JSON.stringify(c.snapshot())),error});
 }
 return {name:s.name,writes:!!s.writes,trace:out};
}
(async()=>{
 const traces=[];for(const s of scenarios)traces.push(await trace(s));
 const bytes=JSON.stringify(traces,null,2)+'\n';
 const dest=path.join(root,'fixtures/source-traces.json');
 if(process.argv.includes('--check'))assert.equal(fs.readFileSync(dest,'utf8'),bytes);else fs.writeFileSync(dest,bytes);
 console.log(`V1 oracle: ${traces.length} flows, ${traces.reduce((n,s)=>n+s.trace.length,0)} steps; sandbox mocks only`);
})();
