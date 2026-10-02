import { useMemo, type ReactNode } from "react";
import { Marked, type Token, type Tokens } from "marked";
import { Katex } from "./output/Katex";
const markdown = new Marked({
  gfm: true,
  extensions: [
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
        const match = /^\$([^$\n]+?)\$/.exec(source);
        return match
          ? { type: "mathInline", raw: match[0], text: match[1] ?? "" }
          : undefined;
      },
    },
  ],
});
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
): ReactNode[] {
  if (depth > 24) return [];
  return tokens.slice(0, 5000).map((token, index) => {
    const child = (tokens: Token[]) =>
      renderTokens(tokens, depth + 1, references);
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
          <pre key={index}>
            <code>{(token as Tokens.Code).text}</code>
          </pre>
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
            {renderTokens(value.tokens, depth + 1)}
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
}: {
  source: string;
  stepReferences?: StepReferences;
}) {
  const tokens = useMemo(() => markdown.lexer(source), [source]);
  return (
    <div className="markdown">{renderTokens(tokens, 0, stepReferences)}</div>
  );
}
