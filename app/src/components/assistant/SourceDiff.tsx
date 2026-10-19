import {useMemo,useState} from 'react';
/** Linear prefix/suffix diff: bounded work even for long notebook sources. */
export function sourceDiff(before:string|undefined,after:string|undefined){
  const old=before===undefined?[]:before.split('\n'),next=after===undefined?[]:after.split('\n');let prefix=0,suffix=0;
  while(prefix<old.length&&prefix<next.length&&old[prefix]===next[prefix])prefix++;
  while(suffix<old.length-prefix&&suffix<next.length-prefix&&old[old.length-1-suffix]===next[next.length-1-suffix])suffix++;
  return [...old.slice(Math.max(0,prefix-2),prefix).map(line=>({kind:'context',line})),...old.slice(prefix,old.length-suffix).map(line=>({kind:'remove',line})),...next.slice(prefix,next.length-suffix).map(line=>({kind:'add',line})),...next.slice(next.length-suffix,next.length-suffix+2).map(line=>({kind:'context',line}))];
}
export function SourceDiff({before,after,more}:{before:string|undefined;after:string|undefined;more:string}){
  const lines=useMemo(()=>sourceDiff(before,after),[before,after]),[all,setAll]=useState(false);
  return <><pre className="source-diff">{(all?lines:lines.slice(0,300)).map(({kind,line},index)=><span className={`diff-${kind}`} key={index}>{kind==='remove'?'−':kind==='add'?'+':' '} {line}{'\n'}</span>)}</pre>{!all&&lines.length>300&&<button onClick={()=>setAll(true)}>{more} ({lines.length-300})</button>}</>;
}
