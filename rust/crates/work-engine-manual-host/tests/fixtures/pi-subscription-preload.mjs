// TEST REALIZATION ONLY. Never imported by the production receiver or binary.
// The synthetic SDK tree adds one import of this module to its entry point.
import {readFileSync} from 'node:fs';
export function fixedSyntheticFetch(command) {
 let request=0;
 return async(url,init)=>{
  if(String(url)!=='https://chatgpt.com/backend-api/codex/responses'||init?.method!=='POST'||init.redirect!=='error')throw Error('unexpected synthetic provider request path');
  const headers=new Headers(init.headers);
  if(headers.get('authorization')!=='Bearer '+syntheticToken())throw Error('only synthetic access admitted');
  const body=JSON.parse(init.body);
  if(body.model!=='gpt-6-luna'||body.stream!==true||++request>2)throw Error('unexpected synthetic provider request');
  const item=request===1?{type:'function_call',id:'fc_synthetic',call_id:'synthetic-tool-1',name:'workspace_exec',arguments:JSON.stringify({command}),status:'completed'}:
   {type:'message',id:'msg_synthetic',role:'assistant',phase:'final_answer',status:'completed',content:[{type:'output_text',text:'Synthetic subscription SSE fixture completed.',annotations:[]}]};
  const response={id:'resp_synthetic_'+request,model:'gpt-6-luna',status:'completed',output:[item],usage:{input_tokens:10,output_tokens:5,total_tokens:15,input_tokens_details:{cached_tokens:0},output_tokens_details:{reasoning_tokens:0}}};
  const events=[{type:'response.created',response:{id:response.id,model:response.model}},
   {type:'response.output_item.added',output_index:0,item:request===1?{...item,arguments:''}:{...item,content:[]}},
   {type:'response.output_item.done',output_index:0,item},
   {type:'response.completed',response}];
  return new Response(events.map(e=>'data: '+JSON.stringify(e)+'\n\n').join(''),{status:200,headers:{'content-type':'text/event-stream','x-synthetic-fixture':'subscription-sse-v1'}});
 };
}
export function syntheticToken(){
 const encode=v=>Buffer.from(JSON.stringify(v)).toString('base64url');
 return encode({alg:'RS256',typ:'JWT'})+'.'+encode({exp:4102444800,'https://api.openai.com/auth':{chatgpt_account_id:'synthetic-account'}})+'.'+Buffer.alloc(32).toString('base64url');
}
globalThis.fetch=fixedSyntheticFetch(readFileSync(new URL('./pi-subscription-command.txt',import.meta.url),'utf8'));
