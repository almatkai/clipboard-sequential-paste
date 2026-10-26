// Executes the actual pinned renderer functions using locked TypeScript/React.
// React element-tree callbacks and keyboard handlers are driven directly; no DOM
// or desktop GUI is claimed. Hooks get deterministic injected state/lifecycle.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),ts=require(root+'/.qa/v1/node_modules/typescript'),React=require(root+'/.qa/v1/node_modules/react');
const jsx=require(root+'/.qa/v1/node_modules/react/jsx-runtime');
let listeners=[];
React.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE.H={
 useState:v=>[typeof v==='function'?v():v===false?true:v,()=>{}],useMemo:f=>f(),useEffect:f=>f(),useRef:v=>({current:v}),useCallback:f=>f(),useContext:()=>({actions:[]})
};
function load(file,imports={}) {
 const code=ts.transpileModule(fs.readFileSync(file,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX}}).outputText;
 const context=vm.createContext({exports:{},require(id){if(id==='react')return React;if(id==='react/jsx-runtime')return jsx;if(Object.hasOwn(imports,id))return imports[id];throw Error('unmocked renderer import '+id);},window:{addEventListener:(t,f)=>listeners.push([t,f]),removeEventListener:()=>{}},setTimeout(){throw Error('unexpected timer');},clearTimeout(){}});
 vm.runInContext(code,context,{timeout:1000});return context.exports;
}
function flatten(n){if(!n||typeof n!=='object')return [];return [n,...[n.props?.children].flat(Infinity).flatMap(flatten)];}
const rel='/src/renderer/src/raycast-api/';
const baseline=root+'/fixtures/host',patched=fs.readFileSync(root+'/.qa/patched-host-path','utf8');
for(const [base,fixed] of [[baseline,false],[patched,true]]) {
 const detail=load(base+rel+'detail-runtime.tsx',{'../../colorConverter':{},'../../ui/Markdown':{Markdown:()=>null},'../../ui/primitives':{cx:()=>''}});
 const overlay=load(base+rel+'action-runtime-overlay.tsx');
 const action=load(base+rel+'action-runtime.tsx',{'./action-runtime-overlay':overlay});
 const registry=load(base+rel+'action-runtime-registry.tsx');
 const surface=load(base+rel+'index.tsx',{'./action-runtime':action,'./action-runtime-registry':registry,'./detail-runtime':detail,'./form-runtime':{},'./grid-runtime':{},'./list-runtime':{},'./hooks':{NavigationContext:React.createContext({}),setRuntimeContext(){}}});
 const actions=[{id:'saved-retry',title:'Retry exact saved item (may duplicate)',kind:'action'},{id:'saved-advance',title:'Mark handled (may skip)',kind:'action'}];
 const calls=[];listeners=[];
 const tree=surface.ExtensionRuntimeSurface({sessionId:'synthetic',title:'Recovery',extensionId:'com.tezbar.native.clipboard-sequential-paste',commandName:'paste-next-item',root:{type:'Detail',props:{markdown:'# Paste result is uncertain'}},actions,onBack(){},onSearchTextChanged(){throw Error('unused search');},onLoadMore(){throw Error('unused load');},onInvokeAction:(...args)=>calls.push(args)});
 const detailNode=flatten(tree).find(n=>n.type===detail.DetailRuntime);assert(detailNode);
 const rendered=detail.DetailRuntime(detailNode.props);
 const enter=flatten(rendered).find(n=>n.type==='button'&&n.props.children==='Enter');assert(enter);
 const event={type:'click',preventDefault(){}};enter.props.onClick(event);
 assert.equal(calls[0][0],fixed?'saved-retry':event);
 if(fixed)assert.equal(typeof calls[0][0],'string');
 const key=listeners.find(([t])=>t==='keydown')[1];key({key:'Enter',preventDefault(){}});assert.equal(calls[1][0],'saved-retry');
 const overlayNode=flatten(tree).find(n=>n.type===overlay.ActionPanelOverlay);assert(overlayNode);
 const buttons=flatten(overlay.ActionPanelOverlay(overlayNode.props)).filter(n=>n.type==='button');assert.equal(buttons.length,2);
 buttons[1].props.onClick(event);assert.equal(calls[2][0],'saved-advance');
}
console.log('Pinned renderer: original primary click bug reproduced; patched click, keyboard Enter, secondary action button IDs passed (React element callbacks, no DOM/GUI)');
// Execute the actual parent's onInvokeAction closure with its surrounding state
// injected. This covers recovery completion, instead of merely matching strings.
async function lifecycle(base,fixed) {
 const file=base+'/src/renderer/ExtensionRuntimeView.tsx',text=fs.readFileSync(file,'utf8');
 const ast=ts.createSourceFile(file,text,ts.ScriptTarget.ES2022,true,ts.ScriptKind.TSX);let handler;
 function visit(node){if(ts.isJsxAttribute(node)&&node.name.getText(ast)==='onInvokeAction')handler=node.initializer.expression;ts.forEachChild(node,visit);}visit(ast);assert(handler);
 let back=0,updates=0;
 const owner={};
 const context=vm.createContext({window:{tezbar:{extensionInvokeAction:async()=>({ok:true,mode:'no-view',message:'completed'})}},state:{actions:[{id:'saved-retry',title:'Retry',kind:'action'}],sessionId:'s'},currentSession:{current:owner},sessionOwner:owner,isFormSurface:false,setError(){},showActionNotice(){},showActionFailure(){},effectNotice(){return null;},fallbackActionNotice(){return null;},onBack(){back++},setState(){updates++}});
 const code=ts.transpileModule('globalThis.handler = '+handler.getText(ast),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
 vm.runInContext(code,context,{timeout:1000});await context.handler('saved-retry',{});
 assert.equal(back,fixed?1:0);assert.equal(updates,fixed?0:1);
}
(async()=>{await lifecycle(baseline,false);await lifecycle(patched,true);console.log('Pinned action result handler: original retained stale view; patched no-view invokes onBack once');})();
