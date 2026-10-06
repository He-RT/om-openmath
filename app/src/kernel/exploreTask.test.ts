import {expect,test} from 'vitest';
import {ExploreTask,type ExploreTaskPort} from './exploreTask';
import type {Response} from './client';
class Port implements ExploreTaskPort {
  onmessage:ExploreTaskPort['onmessage']=null;onerror:ExploreTaskPort['onerror']=null;terminated=false;sent:unknown[]=[];
  postMessage(v:unknown){this.sent.push(v);}terminate(){this.terminated=true;}
  reply(response:Response){this.onmessage?.({data:{response}} as MessageEvent<{response:Response}>);}
}
test('superseded exploration terminates its dedicated worker and cannot write a late reply',async()=>{
  const ports:Port[]=[];const task=new ExploreTask(()=>{const p=new Port();ports.push(p);return p;});
  const first=task.request({type:'run_explore_context',context:'private pure state',values:{a:1},revision:1});
  const aborted=expect(first).rejects.toThrow('cancelled');
  const next=task.request({type:'run_explore_context',context:'private pure state',values:{a:3},revision:3});
  await aborted;expect(ports[0]!.terminated).toBe(true);
  ports[0]!.reply({type:'ok'});
  ports[1]!.reply({type:'error',message:'actual failure'});
  expect(await next).toEqual({type:'error',message:'actual failure'});expect(ports[1]!.terminated).toBe(true);
  task.dispose();await expect(task.request({type:'get_config'})).rejects.toThrow('disposed');
});
