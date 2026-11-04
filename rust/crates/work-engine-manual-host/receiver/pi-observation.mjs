// Pinned Pi 1.5.0 SDK interop only. Rust owns admission, transport and custody.
// Fixed absolute import; no extension discovery or caller-selected module.
import { createAgentSession, DefaultResourceLoader, ModelRuntime, SessionManager, SettingsManager }
  from '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js';
import { createHash } from 'node:crypto';
import { openSync, writeFileSync, fsyncSync, closeSync, existsSync } from 'node:fs';
import { createInterface } from 'node:readline';
const schema = 'work-engine.pi-observation-sdk.v1';
const emit = value => process.stdout.write(JSON.stringify({schema, ...value})+'\n');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
let session;
const deadline = setTimeout(() => { process.exitCode=2; process.stdin.destroy(); }, 30000);
try {
  const settingsManager = SettingsManager.inMemory({compaction:{enabled:false},retry:{enabled:false},cacheWarming:'off'});
  const resourceLoader = new DefaultResourceLoader({cwd:'/workspace',agentDir:'/tmp/pi-empty',settingsManager,
    noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true});
  await resourceLoader.reload();
  const modelRuntime = await ModelRuntime.create({authPath:'/tmp/pi-no-credentials.json',modelsPath:null,
    allowModelNetwork:false,refreshOnCreate:false});
  ({session} = await createAgentSession({cwd:'/workspace',agentDir:'/tmp/pi-empty',settingsManager,resourceLoader,
    modelRuntime,sessionManager:SessionManager.create('/workspace','/export/sessions'),noTools:'all',tools:[],customTools:[]}));
  session.setAutoCompactionEnabled(false);session.setAutoRetryEnabled(false);
  // Even an accidental future run request must not reach a provider.
  session.agent.streamFunction = () => { throw new Error('provider execution disabled by observation profile'); };
  const idle = () => !session.isStreaming && !session.isCompacting && session.pendingMessageCount===0;
  const state = () => ({sessionId:session.sessionId,isStreaming:session.isStreaming,isCompacting:session.isCompacting,
    pendingMessageCount:session.pendingMessageCount,autoCompactionEnabled:session.autoCompactionEnabled,
    autoRetryEnabled:session.autoRetryEnabled});
  session.subscribe(event => {
    if (event.type==='message_start' || event.type==='message_end') emit({type:'native_event',sessionId:session.sessionId,event});
    else { emit({type:'forbidden_native_event',event});process.exitCode=2; }
  });
  emit({type:'ready',state:state()});
  let consumed=false;
  for await (const line of createInterface({input:process.stdin,crlfDelay:Infinity})) {
    if (consumed || Buffer.byteLength(line)>131072) throw new Error('one bounded release only');
    consumed=true;
    const command=JSON.parse(line);
    if (Object.keys(command).sort().join(',')!=='envelopeSha256,nativeSessionId,rawEnvelope,type' || command.type!=='observe'
        || command.nativeSessionId!==session.sessionId || typeof command.rawEnvelope!=='string'
        || hash(command.rawEnvelope)!==command.envelopeSha256 || !idle()) throw new Error('release binding or idle admission failed');
    const envelope=JSON.parse(command.rawEnvelope);
    if (envelope.schema!=='work-engine.delivery-envelope/v1' || envelope.authority!=='observation_only'
        || envelope.recipient.session!==session.sessionId) throw new Error('native envelope binding mismatch');
    const details={rawEnvelope:command.rawEnvelope,envelopeSha256:command.envelopeSha256};
    emit({type:'admitted',sessionId:session.sessionId,envelopeSha256:command.envelopeSha256});
    // Return is not an ACK. Host requires independent native event + message/history evidence.
    await session.sendCustomMessage({customType:'work-engine.delivery-observation.v1',content:[{type:'text',text:command.rawEnvelope}],display:false,details},{triggerTurn:false});
    if (!idle()) throw new Error('target became busy');
    emit({type:'get_messages',sessionId:session.sessionId,messages:session.messages,state:state()});
    const prospective=session.sessionManager.getSessionFile();
    const history={schema,sessionId:session.sessionId,header:session.sessionManager.getHeader(),
      entries:session.sessionManager.getEntries(),messages:session.messages,
      nativeSessionFile:prospective && existsSync(prospective) ? prospective : null};
    const bytes=JSON.stringify(history)+'\n';
    const fd=openSync('/export/receiver-native-history.json','wx',0o600);
    try {writeFileSync(fd,bytes);fsyncSync(fd);} finally {closeSync(fd);}
    emit({type:'history_export',sessionId:session.sessionId,path:'receiver-native-history.json',sha256:hash(bytes)});
  }
  if (!consumed) throw new Error('no release');
} catch (error) { emit({type:'error',message:String(error)});process.exitCode=2; }
finally {clearTimeout(deadline);session?.dispose();}
