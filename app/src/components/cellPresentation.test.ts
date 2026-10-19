import { expect, test } from "vitest";
import { deriveCellPresentation, cellPresentation, outputFacts, type CellPresentationFacts } from "./cellPresentation";
import { messages } from "../i18n";
import type { UiCell } from "../state/notebookStore";
test.each<[CellPresentationFacts,string]>([
  [{},"pending"], [{running:true},"running"], [{stopped:true},"stopped"],
  [{hasResult:true},"unconfirmed"], [{hasResult:true,hasMetadata:true,nature:"exact"},"exact"],
  [{hasResult:true,hasMetadata:true,nature:"machine"},"numeric"], [{hasResult:true,hasMetadata:true,nature:"high_precision"},"numeric"],
  [{hasResult:true,hasMetadata:true,outputKind:"plot"},"geometry"], [{hasResult:true,hasMetadata:true,outputKind:"scene3_d"},"geometry"],
  [{hasResult:true,hasMetadata:true,unevaluated:true,nature:"symbolic"},"unevaluated"],
  [{hasResult:true,hasMetadata:true,verified:true},"verified"],
  [{hasResult:true,hasMetadata:true,verified:true,conditional:true},"conditional"],
  [{hasResult:true,hasMetadata:true,conditional:true,error:true},"error"],
  [{hasResult:true,sourceChanged:true,error:true},"stale"],
  [{hasResult:true,staleReason:"a changed",verified:true,running:true},"running"],
  [{hasResult:true,staleReason:"a changed",verified:true,stopped:true},"stopped"],
])("matches native presentation precedence for %o",(facts,status)=>{expect(deriveCellPresentation(facts).status).toBe(status);});
test("source/definition freshness and solver evidence are independent of representation",()=>{
  const value:UiCell={id:"c",source:"2+2",kind:"Math",dialect:"Modern",revision:1,status:"Done",defines:[],uses:[],output:{items:[{type:"expr",out_index:1,latex:"4",input_form:"4",modern_form:"4",nature:"exact",unchanged:false}],messages:[],timing_ms:2},resultSource:{source:"2+2",dialect:"Modern"}};
  expect(cellPresentation(value,messages("en")).status).toBe("exact");
  expect(cellPresentation({...value,source:"3+3",status:"Stale"},messages("en"))).toEqual({status:"stale",byAgent:false,reason:"Source changed"});
  expect(cellPresentation({...value,status:"Stale"},messages("en"))).toEqual({status:"stale",byAgent:false,reason:"Source or dependency definitions changed"});
  expect(deriveCellPresentation({hasResult:true,hasMetadata:true,nature:"symbolic",byAgent:true})).toEqual({status:"exact",byAgent:true});
  const legacy=outputFacts({items:[{type:"expr",out_index:1,latex:"4",input_form:"4",modern_form:"4"}],messages:[],timing_ms:1});
  expect(deriveCellPresentation(legacy).status).toBe("unconfirmed");
});

test('structured records never claim exactness from mixed fields or arbitrary certificate labels',()=>{
  const output={items:[{type:'expr' as const,out_index:1,latex:'record',input_form:'Record[]',modern_form:'Record[]',nature:'symbolic' as const,unchanged:false,presentation:{view_id:'1',path:[],kind:'record' as const,row_count:0,column_count:0,offset:0,column_offset:0,columns:[],rows:[],source:null,origin:null}}],messages:[],timing_ms:1};
  expect(deriveCellPresentation(outputFacts(output)).status).toBe('structured');expect(outputFacts(output).verified).toBe(false);expect(deriveCellPresentation({...outputFacts(output),error:true}).status).toBe('error');
});
