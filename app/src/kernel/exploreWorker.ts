/// <reference lib="webworker" />
import init, { Kernel } from './wasm/om_wasm.js';
import type { Request } from './client';
self.onmessage=(event:MessageEvent<{request:Request}>)=>{
  void (async()=>{
    let kernel: Kernel | undefined;
    try {
      await init();
      kernel=new Kernel(JSON.stringify({general:{auto_plot:false,reactive:false,auto_run_dependents:false},llm:{enabled:false}}));
      kernel.request(JSON.stringify({id:2,body:{type:'set_host_platform',platform:'web'}}));
      const packet=JSON.parse(kernel.request(JSON.stringify({id:1,body:event.data.request}))) as {response:{body:unknown}};
      self.postMessage({response:packet.response.body});
    }catch {self.postMessage({response:{type:'error',message:'Exploration task failed'}});}
    finally{kernel?.free();}
  })();
};
