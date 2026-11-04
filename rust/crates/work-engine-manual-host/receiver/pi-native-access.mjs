// Minimal CredentialStore interoperability only. Rust owns source admission,
// projection, deadlines and private staging. This module opens no files.
export function createNativeAccessStore(grant, now=()=>Date.now()) {
  if(Object.keys(grant).sort().join(',')!=='access,account_id,expires_ms'
      ||typeof grant.access!=='string'||grant.access.length===0
      ||typeof grant.account_id!=='string'||grant.account_id.length===0
      ||!Number.isSafeInteger(grant.expires_ms))throw Error('invalid access-only projection');
  const check=()=>{if(grant.expires_ms<=now()+300000)throw Error('native access validity insufficient; refresh prohibited');};
  check();
  const reject=async()=>{throw Error('native credential mutation and refresh prohibited');};
  return {
    async read(providerId){if(providerId!=='openai-codex')return undefined;check();return {type:'oauth',access:grant.access,refresh:'',expires:grant.expires_ms,accountId:grant.account_id};},
    async list(){check();return [{providerId:'openai-codex',type:'oauth'}];},
    modify:reject,delete:reject,
  };
}
// Preserve presence/types from the raw terminal event; SDK-normalized defaults
// cannot attest backend completion or measured zero tokens.
export function rawTerminalEvidence(data){
 if(data?.type!=='response.completed')return undefined;
 const response=data.response;
 return {eventType:data.type,status:response?.status,usage:{input_tokens:response?.usage?.input_tokens,output_tokens:response?.usage?.output_tokens,total_tokens:response?.usage?.total_tokens}};
}
