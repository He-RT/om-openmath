import { useLayoutEffect, useMemo, useRef, type ReactNode } from "react";
import { Marked, type Token, type Tokens } from "marked";
import { Katex } from "./output/Katex";
import { CodeBlock } from "./CodeBlock";
import type { Dialect } from "../kernel/generated/Dialect";
import { locale, messages, type Messages } from "../i18n";
interface MarkdownActions {t:Messages;onInsertCode?:((source:string,dialect:Dialect)=>void)|undefined}
function texDelimiter(source:string,display:boolean){
  const open=display?'\\[':'\\(',close=display?'\\]':'\\)';if(!source.startsWith(open))return;
  const limit=Math.min(source.length,16386);let end=2;
  for(;end<limit;end++){
    if(!display&&/[\r\n]/.test(source[end]!))break;
    if(!source.startsWith(close,end))continue;
    let slashes=0;for(let index=end-1;index>=0&&source[index]==='\\';index--)slashes++;
    if(slashes%2===0){const suffix=display?/^[\t ]*(?:\n|$)/.exec(source.slice(end+2)):null;if(display&&!suffix)return;
      return {type:display?'mathBlock':'mathInline',raw:source.slice(0,end+2+(suffix?.[0].length??0)),text:source.slice(2,end)};
    }
  }
  if(!display)return {type:'text',raw:source.slice(0,end),text:source.slice(0,end)};
}
const markdown = new Marked({
  gfm: true,
  extensions: [
    {name:'texMathBlock',level:'block',start:source=>source.indexOf('\\['),tokenizer:source=>texDelimiter(source,true)},
    {name:'texMathInline',level:'inline',start:source=>{const a=source.indexOf('\\('),b=source.indexOf('\\[');return a<0?b:b<0?a:Math.min(a,b);},tokenizer:source=>{
      const token=texDelimiter(source,false);if(token)return token;
      if(source.startsWith('\\[')){const end=source.indexOf('\\]',2),raw=source.slice(0,end>=0?end+2:source.indexOf('\n')>=0?source.indexOf('\n'):source.length);return {type:'text',raw,text:raw};}
    }},
    {
      name: "mathBlock",
      level: "block",
      start: (source) => source.indexOf("$$"),
      tokenizer: (source) => {
        const match = /^\$\$\s*\n?([\s\S]*?)\$\$(?:\n|$)/.exec(source);
        return match
          ? { type: "mathBlock", raw: match[0], text: match[1] ?? "" }
          : undefined;
      },
    },
    {
      name: "mathInline",
      level: "inline",
      start: (source) => source.indexOf("$"),
      tokenizer: (source) => {
        const match = /^\$([^\s$](?:[^$\n]*?[^\s$])?)\$(?!\d)/.exec(source);
        return match
          ? { type: "mathInline", raw: match[0], text: match[1] ?? "" }
          : undefined;
      },
    },
  ],
});
export function markdownBlocks(source:string){return markdown.lexer(source);}
function safeUrl(href: string) {
  if (href.startsWith("#")) return href;
  try {
    const url = new URL(href, location.href);
    return ["https:", "http:", "mailto:"].includes(url.protocol)
      ? url.href
      : undefined;
  } catch {
    return undefined;
  }
}
export interface StepReferences {
  knownIds: Set<string>;
  onReference: (id: string) => void;
  label?: string;
}
function referenceText(text: string, references?: StepReferences): ReactNode {
  if (!references) return text;
  return text.split(/(\[S\d+(?:\.\d+)*\])/).map((part, i) => {
    const id = /^\[(S\d+(?:\.\d+)*)\]$/.exec(part)?.[1];
    return id && references.knownIds.has(id) ? (
      <button
        className="step-reference"
        key={i}
        aria-label={`${references.label ?? "Go to"} ${id}`}
        onClick={() => references.onReference(id)}
      >
        {part}
      </button>
    ) : (
      part
    );
  });
}
function renderTokens(
  tokens: Token[],
  depth = 0,
  references?: StepReferences,
  actions?: MarkdownActions,
): ReactNode[] {
  if (depth > 24) return [];
  return tokens.slice(0, 5000).map((token, index) => {
    const child = (tokens: Token[]) =>
      renderTokens(tokens, depth + 1, references, actions);
    switch (token.type) {
      case "space":
        return null;
      case "mathInline":
      case "mathBlock":
        return (
          <Katex
            key={index}
            latex={String(token.text ?? "")}
            display={token.type === "mathBlock"}
          />
        );
      case "heading": {
        const value = token as Tokens.Heading;
        const content = child(value.tokens);
        return value.depth === 1 ? (
          <h2 key={index}>{content}</h2>
        ) : value.depth === 2 ? (
          <h3 key={index}>{content}</h3>
        ) : (
          <h4 key={index}>{content}</h4>
        );
      }
      case "paragraph":
        return <p key={index}>{child((token as Tokens.Paragraph).tokens)}</p>;
      case "text": {
        const value = token as Tokens.Text;
        return (
          <span key={index}>
            {value.tokens
              ? child(value.tokens)
              : referenceText(value.text, references)}
          </span>
        );
      }
      case "strong":
        return (
          <strong key={index}>{child((token as Tokens.Strong).tokens)}</strong>
        );
      case "em":
        return <em key={index}>{child((token as Tokens.Em).tokens)}</em>;
      case "del":
        return <del key={index}>{child((token as Tokens.Del).tokens)}</del>;
      case "codespan":
        return <code key={index}>{(token as Tokens.Codespan).text}</code>;
      case "code":
        return (
          <CodeBlock key={index} code={(token as Tokens.Code).text} language={(token as Tokens.Code).lang ?? ""} t={actions?.t ?? messages(locale())} onInsert={actions?.onInsertCode}/>
        );
      case "blockquote":
        return (
          <blockquote key={index}>
            {child((token as Tokens.Blockquote).tokens)}
          </blockquote>
        );
      case "list": {
        const value = token as Tokens.List;
        const content = value.items.map((item, i) => (
          <li key={i}>
            {item.task && (
              <input type="checkbox" checked={item.checked} readOnly />
            )}
            {child(item.tokens)}
          </li>
        ));
        return value.ordered ? (
          <ol
            key={index}
            start={typeof value.start === "number" ? value.start : 1}
          >
            {content}
          </ol>
        ) : (
          <ul key={index}>{content}</ul>
        );
      }
      case "table": {
        const value = token as Tokens.Table;
        return (
          <div className="markdown-table" key={index}>
            <table>
              <thead>
                <tr>
                  {value.header.map((cell, i) => (
                    <th key={i}>{child(cell.tokens)}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {value.rows.map((row, i) => (
                  <tr key={i}>
                    {row.map((cell, j) => (
                      <td key={j}>{child(cell.tokens)}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      }
      case "link": {
        const value = token as Tokens.Link;
        return (
          <a
            key={index}
            href={safeUrl(value.href)}
            rel="noopener noreferrer"
            target={value.href.startsWith("#") ? undefined : "_blank"}
          >
            {renderTokens(value.tokens, depth + 1,undefined,actions)}
          </a>
        );
      }
      case "image": {
        const value = token as Tokens.Image;
        return (
          <a
            key={index}
            href={safeUrl(value.href)}
            rel="noopener noreferrer"
            target="_blank"
          >
            {value.text || value.href} ↗
          </a>
        );
      }
      case "br":
        return <br key={index} />;
      case "hr":
        return <hr key={index} />;
      case "html":
        return <span key={index}>{token.raw}</span>;
      case "escape":
        return <span key={index}>{String(token.text ?? "")}</span>;
      default:
        return <span key={index}>{token.raw}</span>;
    }
  });
}
export function Markdown({
  source,
  stepReferences,
  t=messages(locale()),
  onInsertCode,
}: {
  source: string;
  stepReferences?: StepReferences;
  t?:Messages;
  onInsertCode?:(source:string,dialect:Dialect)=>void;
}) {
  const tokens = useMemo(() => markdown.lexer(source), [source]);
  return (
    <div className="markdown">{renderTokens(tokens, 0, stepReferences,{t,onInsertCode})}</div>
  );
}
export function TextCell({source,index,editing,onModeChange,onChange,onFocus,t,onInsertCode}:{source:string;index:number;editing:boolean;onModeChange:(editing:boolean)=>void;onChange:(source:string)=>void;onFocus:()=>void;t:Messages;onInsertCode:(source:string,dialect:Dialect)=>void}) {
  const editor=useRef<HTMLTextAreaElement>(null),reading=useRef<HTMLDivElement>(null),selection=useRef({start:0,end:0}),composing=useRef(false),wasEditing=useRef(false);
  useLayoutEffect(()=>{
    if(editing){wasEditing.current=true;const element=editor.current;if(element){element.focus({preventScroll:true});element.setSelectionRange(Math.min(selection.current.start,source.length),Math.min(selection.current.end,source.length));}}
    else if(wasEditing.current){wasEditing.current=false;reading.current?.focus({preventScroll:true});}
    // Source changes during typing must not move the caret.
  },[editing]);
  useLayoutEffect(()=>{
    const size=()=>{const element=editor.current;if(!element)return;const scroll=element.scrollTop;element.style.height="auto";element.style.height=`${Math.max(120,Math.min(element.scrollHeight,innerHeight*.6))}px`;element.scrollTop=scroll;};
    if(!editing)return;size();window.addEventListener("resize",size);return()=>window.removeEventListener("resize",size);
  },[source,editing]);
  const finish=()=>{if(composing.current)return;if(editor.current)selection.current={start:editor.current.selectionStart,end:editor.current.selectionEnd};onModeChange(false);};
  return <div className={`text-cell ${editing ? "text-cell-editing":"text-cell-reading"}`}>
    {editing ? <>
      <textarea ref={editor} className="prose-editor" aria-label={`${t.textInput} ${index+1}`} placeholder={t.text} value={source} onFocus={onFocus} onChange={event=>onChange(event.target.value)} onCompositionStart={()=>{composing.current=true;}} onCompositionEnd={()=>{composing.current=false;}} onKeyDown={event=>{if(event.nativeEvent.isComposing || composing.current || event.keyCode===229)return;if(event.key==="Escape" || event.key==="Enter" && (event.ctrlKey || event.metaKey)){event.preventDefault();event.stopPropagation();finish();}}}/>
      <div className="text-cell-actions"><span>{t.readShortcut}</span><button onClick={finish}>{t.read}</button></div>
    </> : <>
      <div ref={reading} tabIndex={0} className="text-reading-content" aria-label={`${t.text} ${index+1}`} onDoubleClick={event=>{if(!(event.target as HTMLElement).closest(".code-block,button,a,pre,code,input,select,textarea"))onModeChange(true);}} onKeyDown={event=>{if(event.target===event.currentTarget && event.key==="Enter"){event.preventDefault();onModeChange(true);}}}>
        {source ? <Markdown source={source} t={t} onInsertCode={onInsertCode}/>:<p className="text-empty">{t.emptyTextCell}</p>}
      </div>
      <div className="text-cell-actions"><button aria-label={`${t.edit} ${t.text} ${index+1}`} onClick={()=>onModeChange(true)}>{t.edit}</button></div>
    </>}
  </div>;
}
