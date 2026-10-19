import type {CellOutput} from '../kernel/generated/CellOutput';
export type AgentProductFact=
  |{type:'expr';input_form:string;truncated:boolean}
  |{type:'solutions';solution_kind:'finite'|'all'|'none'|'region';solution_count:number;verified:boolean;conditional:boolean}
  |{type:'plot';geometry_exists:boolean;curves:number}
  |{type:'scene3_d';geometry_exists:boolean;triangles:number};
/** Frozen at a successful inspection, before the requested data is paginated or chunked. */
export function productFacts(output:CellOutput):AgentProductFact[]{return output.items.flatMap<AgentProductFact>(item=>{
  if(item.type==='expr'){const prefix=item.input_form.slice(0,160),safe=prefix.charCodeAt(prefix.length-1)>=0xD800&&prefix.charCodeAt(prefix.length-1)<=0xDBFF?prefix.slice(0,-1):prefix;return [{type:item.type,input_form:safe,truncated:safe.length<item.input_form.length}];}
  if(item.type==='solutions')return [{type:item.type,solution_kind:item.view.kind,solution_count:item.view.solutions.length,verified:item.view.solutions.length>0&&item.view.solutions.every(row=>row.verified==='Exact'),conditional:!!item.view.region_latex||item.view.solutions.some(row=>!!row.condition_latex)}];
  if(item.type==='plot')return [{type:item.type,geometry_exists:!!item.data,curves:item.data?.curves.length??0}];
  if(item.type==='scene3_d')return [{type:item.type,geometry_exists:!!item.data,triangles:item.data?.meshes.reduce((count,mesh)=>count+mesh.triangles.length,0)??0}];
  return [];
});}
