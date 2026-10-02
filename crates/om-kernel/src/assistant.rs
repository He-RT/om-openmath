//! Production host source verification; model output never becomes executable host code.
use om_llm::{Suggestion, SuggestionValidator};
use om_parse::{Dialect, Severity};
use serde::Deserialize;

pub(crate) const LIMIT: usize = 1_048_576;
/// Real Wolfram parser/formatter adapter required by structured LLM jobs.
#[derive(Clone, Copy, Debug)]
pub struct SuggestionParser;
#[derive(Deserialize)]
struct Proposal {
    wolfram: String,
    explanation: String,
}

impl SuggestionValidator for SuggestionParser {
    fn validate(&self, response: &str) -> Result<Suggestion, String> {
        if response.len() > LIMIT {
            return Err("LLM response exceeds source limit".into());
        }
        let from = response
            .find('{')
            .ok_or("Expected JSON wolfram/explanation object")?;
        let to = response.rfind('}').ok_or("Expected complete JSON object")?;
        if from > to {
            return Err("Expected complete JSON object".into());
        }
        let proposal: Proposal = serde_json::from_str(&response[from..=to])
            .map_err(|_| "Expected unique string wolfram/explanation fields in valid JSON")?;
        let parsed = om_parse::parse(&proposal.wolfram, Dialect::Wolfram);
        let diagnostics: Vec<_> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| {
                format!(
                    "{} at {}..{}: {}",
                    d.code, d.span.start, d.span.end, d.message
                )
            })
            .collect();
        if !diagnostics.is_empty() {
            return Err(diagnostics.join("; "));
        }
        if parsed.statements.len() != 1 || parsed.statements[0].suppress_output {
            return Err("Expected ONE nonempty unsuppressed Wolfram expression".into());
        }
        let expr = &parsed.statements[0].expr;
        Ok(Suggestion {
            wolfram: om_format::input_form(expr),
            modern: om_format::modern_form(expr),
            latex: om_format::latex(expr),
            explanation: proposal.explanation,
        })
    }
}

/// First-line insertion filter using the actual lexer; incomplete syntax remains useful.
pub fn filter_completion(
    prefix: &str,
    suffix: &str,
    suggestion: &str,
    dialect: Dialect,
    local_first: Option<&str>,
) -> Option<String> {
    if prefix
        .len()
        .saturating_add(suffix.len())
        .saturating_add(suggestion.len())
        > LIMIT
    {
        return None;
    }
    // Strip boundary line breaks without dropping legitimate indentation/insertion spaces.
    let start = suggestion.find(|c: char| !c.is_whitespace());
    let end = suggestion.rfind(|c: char| !c.is_whitespace());
    let (start, end) = (start?, end?);
    let leading = &suggestion[..start];
    let trailing = &suggestion[end + suggestion[end..].chars().next()?.len_utf8()..];
    let line_break = |c| matches!(c, '\r' | '\n' | '\u{2028}' | '\u{2029}');
    let from = leading.rfind(line_break).map_or(0, |i| {
        i + suggestion[i..].chars().next().map_or(0, char::len_utf8)
    });
    let to = trailing
        .find(line_break)
        .map_or(suggestion.len(), |i| suggestion.len() - trailing.len() + i);
    let text = &suggestion[from..to];
    let text = text.split(line_break).next()?;
    if text.is_empty() || local_first == Some(text) {
        return None;
    }
    let source = format!("{prefix}{text}{suffix}");
    let parsed = om_parse::parse(&source, dialect);
    if parsed
        .diagnostics
        .iter()
        .any(|d| matches!(d.code, "E001" | "E005" | "E006" | "E007"))
    {
        return None;
    }
    Some(text.into())
}

pub(crate) fn context_comment(source: &str, dialect: Dialect) -> String {
    if dialect == Dialect::Wolfram {
        format!(
            "(*\n{}\n*)\n",
            source.replace("(*", "( *").replace("*)", "* )")
        )
    } else {
        let source = source.replace(['\r', '\u{2028}', '\u{2029}'], "\n");
        source
            .split('\n')
            .map(|line| format!("# {line}\n"))
            .collect()
    }
}
