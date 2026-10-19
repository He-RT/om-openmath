/** Synthetic provider replies exercise the real agent tools; no model API is called. */
export function s1Response(body:{model?:string;messages:{role:string;content?:string}[]}){
  const user=body.messages.findLastIndex(message=>message.role==='user'),current=body.messages.slice(user),previous=current.filter(message=>message.role==='tool').map(message=>JSON.parse(message.content!) as {data:Record<string,unknown>}),phase=previous.length;
  let name='',args:Record<string,unknown>={},content='';
  const cells=previous[1]?.data?.cells as {cell_id:string;source:string;source_hash:string}[]|undefined;
  if(!String(current[0]?.content).includes('change a'))content='Previous notebook work remains in this conversation. $a+1=6$.\n\n```openmath\n2 + 2\n```';
  else if(phase===0){name='read_notebook';args={view:'outline'};}
  else if(phase===1){name='read_notebook';args={view:'cells',cell_ids:(previous[0]!.data.cells as {cell_id:string}[]).map(cell=>cell.cell_id),include_outputs:true};}
  else if(phase===2){name='preview_source';args={};}
  else if(phase===3){const a=cells!.find(cell=>cell.source.replaceAll(' ','')==='leta=2')!;name='preview_source';args={snapshot_ref:previous[1]!.data.snapshot_ref,input:{kind:'patch',operations:[{type:'replace_text',target:{cell_id:a.cell_id},expected_source_hash:a.source_hash,match:'2',replacement:'5'}]}};}
  else if(phase===4){name='apply_notebook_patch';args={preview_ref:previous[3]!.data.preview_ref};}
  else if(phase===5){name='run_cells';args={snapshot_ref:previous[4]!.data.snapshot_ref,cell_ids:[cells!.find(cell=>cell.source.replaceAll(' ','')==='leta=2')!.cell_id],scope:'affected'};}
  else if(phase===6){const outputs=previous[5]!.data.cells as {cell_id:string;result_ref:string}[];name='inspect_result';args={result_ref:outputs.find(cell=>cell.cell_id===cells!.find(cell=>cell.source.replaceAll(' ','')==='a+1')!.cell_id)!.result_ref};}
  else content='已修改 a = 5，运行两个单元格，并核对 a + 1 = 6。尚未保存到文件。';
  const delta=name?{tool_calls:[{index:0,id:'loopback-call-'+body.messages.length,type:'function',function:{name,arguments:JSON.stringify(args)}}]}:{content};
  return `data: ${JSON.stringify({id:'loopback-response-'+body.messages.length,model:body.model??'loopback',choices:[{index:0,delta,finish_reason:name?'tool_calls':'stop'}],usage:{prompt_tokens:120,completion_tokens:20,total_tokens:140}})}\n\ndata: [DONE]\n\n`;
}
