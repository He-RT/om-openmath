import {AgentBridge} from './bridge.ts';
import {FrameDecoder} from './protocol.ts';

// One private stdin/stdout activation. stdout is reserved for strict framed protocol data.
if(process.argv.length!==2)process.exit(64);
const decoder=new FrameDecoder();let stopping=false;
const bridge=new AgentBridge(bytes=>new Promise<void>((resolve,reject)=>{
 process.stdout.write(bytes,error=>error?reject(new Error('IPC_WRITE_FAILED')):resolve());
}),()=>{stopping=true;process.stdin.destroy();});
try{
 for await(const chunk of process.stdin){
  if(stopping)break;
  for(const {raw,frame} of decoder.pushTransportChunk(chunk))await bridge.accept(frame,raw);
 }
 if(!stopping)decoder.finish();
 await bridge.close();
}catch{
 // Peer-controlled bytes and internal exceptions never enter logs or diagnostics.
 await bridge.close();process.exitCode=stopping?0:70;
}
