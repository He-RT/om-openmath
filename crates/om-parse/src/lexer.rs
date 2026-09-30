//! Lossless token spellings and UTF-8 byte positions for both dialects.

use crate::{Diagnostic, Dialect, Severity, Span, TokenClass};

#[path = "lexer_chars.rs"]
pub(crate) mod named;
#[path = "lexer_numbers.rs"]
mod numbers;

/// A token's grammatical role. Its spelling is retained in the source span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Unsigned numeric literal, including exponent or precision suffix.
    Number,
    /// Symbol or named-character identifier.
    Identifier,
    /// Quoted string, including escapes and delimiters.
    String,
    /// Line or nested block comment.
    Comment,
    /// LF, CR, CRLF, or a Unicode line separator.
    Newline,
    /// Consecutive Unicode superscript digits or minus signs.
    Superscript,
    /// Modern declaration keyword.
    Let,
    /// Modern replacement keyword.
    Where,
    /// Addition or unary positive sign.
    Plus,
    /// Subtraction or unary negative sign.
    Minus,
    /// Explicit multiplication.
    Star,
    /// Division.
    Slash,
    /// Exponentiation, spelled ^ or **.
    Power,
    /// Single =; interpretation depends on dialect and statement context.
    Assign,
    /// Equality.
    Equal,
    /// Structural equality ===.
    SameQ,
    /// Inequality != or ≠.
    Unequal,
    /// Strict less-than.
    Less,
    /// Non-strict less-than.
    LessEqual,
    /// Strict greater-than.
    Greater,
    /// Non-strict greater-than.
    GreaterEqual,
    /// Conjunction.
    And,
    /// Disjunction.
    Or,
    /// Word or Unicode negation.
    Not,
    /// Prefix negation or postfix factorial.
    Bang,
    /// Immediate rule.
    Rule,
    /// Delayed rule.
    RuleDelayed,
    /// ReplaceAll /..
    ReplaceAll,
    /// ReplaceRepeated //..
    ReplaceRepeated,
    /// Delayed assignment :=.
    SetDelayed,
    /// Unset =..
    Unset,
    /// Pattern condition /;.
    Condition,
    /// Prefix function application @.
    PrefixApply,
    /// Postfix function application //.
    PostfixApply,
    /// Head application @@.
    Apply,
    /// Map /@.
    Map,
    /// Derivative apostrophe.
    Prime,
    /// Pure-function terminator &.
    Function,
    /// Wolfram # or #n slot.
    Slot,
    /// Previous output %.
    Out,
    /// Absolute-value delimiter |.
    Bar,
    /// Prefix square root √.
    Sqrt,
    /// Set membership ∈ or named Element.
    Element,
    /// Keyword-argument or pattern separator :.
    Colon,
    /// Argument separator.
    Comma,
    /// Statement separator and output suppression.
    Semicolon,
    /// Opening parenthesis.
    LParen,
    /// Closing parenthesis.
    RParen,
    /// Opening square bracket; never coalesced with an adjacent bracket.
    LBracket,
    /// Closing square bracket.
    RBracket,
    /// Opening list brace.
    LBrace,
    /// Closing list brace.
    RBrace,
    /// Wolfram _.
    Blank,
    /// Wolfram __.
    BlankSequence,
    /// Wolfram ___.
    BlankNullSequence,
    /// Malformed token; a diagnostic supplies details.
    Error,
}

/// A token and its source-highlighting category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    /// Grammatical role.
    pub kind: TokenKind,
    /// Half-open source byte offsets.
    pub span: Span,
    /// Highlighting category, refined for built-ins by the parser.
    pub class: TokenClass,
}

/// Tokenization result, retaining recoverable lexical failures.
#[derive(Clone, Debug)]
pub struct LexOutput {
    /// Tokens in source order, including comments and newlines.
    pub tokens: Vec<Token>,
    /// Lexical errors in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Tokenize input without numerical conversion or semantic normalization.
/// Auto defaults to Modern; automatic detection belongs to the parser.
pub fn lex(src: &str, dialect: Dialect) -> LexOutput {
    if u32::try_from(src.len()).is_err() {
        return LexOutput {
            tokens: vec![],
            diagnostics: vec![Diagnostic {
                span: Span { start: 0, end: 0 },
                severity: Severity::Error,
                code: "E007",
                message: "输入超出 UTF-8 位置范围".into(),
                fix: None,
            }],
        };
    }
    let mut scanner = Scanner {
        src,
        pos: 0,
        dialect,
        output: LexOutput {
            tokens: vec![],
            diagnostics: vec![],
        },
    };
    while let Some(ch) = scanner.peek() {
        let start = scanner.pos;
        if is_newline(ch) {
            scanner.bump();
            if ch == '\r' && scanner.peek() == Some('\n') {
                scanner.bump();
            }
            scanner.emit(start, TokenKind::Newline);
        } else if ch.is_whitespace() {
            scanner.bump();
        } else {
            let kind = scanner.scan(start, ch);
            scanner.emit(start, kind);
        }
    }
    scanner.output
}

struct Scanner<'a> {
    src: &'a str,
    pos: usize,
    dialect: Dialect,
    output: LexOutput,
}

impl Scanner<'_> {
    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
    fn span(&self, start: usize) -> Span {
        // lex rejects sources larger than u32, and every cursor stays on a boundary.
        Span {
            start: start as u32,
            end: self.pos as u32,
        }
    }
    fn error(&mut self, start: usize, code: &'static str, message: &str) -> TokenKind {
        self.output.diagnostics.push(Diagnostic {
            span: self.span(start),
            severity: Severity::Error,
            code,
            message: message.into(),
            fix: None,
        });
        TokenKind::Error
    }
    fn emit(&mut self, start: usize, kind: TokenKind) {
        use TokenKind as K;
        let class = match kind {
            K::Number | K::Superscript => TokenClass::Number,
            K::Identifier => TokenClass::Identifier,
            K::String => TokenClass::String,
            K::Comment => TokenClass::Comment,
            K::Let | K::Where => TokenClass::Keyword,
            K::And | K::Or | K::Not
                if self.src[start..self.pos].chars().all(char::is_alphabetic) =>
            {
                TokenClass::Keyword
            }
            K::LParen | K::RParen | K::LBracket | K::RBracket | K::LBrace | K::RBrace => {
                TokenClass::Bracket
            }
            K::Error => TokenClass::Error,
            _ => TokenClass::Operator,
        };
        self.output.tokens.push(Token {
            kind,
            span: self.span(start),
            class,
        });
    }
    fn scan(&mut self, start: usize, ch: char) -> TokenKind {
        use TokenKind as K;
        if self.rest().starts_with("(*") {
            return self.comment(start);
        }
        if ch == '#' {
            self.bump();
            if self.dialect == Dialect::Wolfram {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.bump();
                }
                return K::Slot;
            }
            while self.peek().is_some_and(|c| !is_newline(c)) {
                self.bump();
            }
            return K::Comment;
        }
        if ch == '"' {
            return self.string(start);
        }
        if self.rest().starts_with("\\[") {
            let kind = self.named_character(start);
            if kind == K::Identifier {
                self.identifier_tail();
            }
            return kind;
        }
        if ch.is_ascii_digit()
            || (ch == '.'
                && self
                    .rest()
                    .as_bytes()
                    .get(1)
                    .is_some_and(u8::is_ascii_digit))
        {
            return self.number(start);
        }
        if superscript(ch) {
            while self.peek().is_some_and(superscript) {
                self.bump();
            }
            return K::Superscript;
        }
        if ch.is_alphabetic()
            || ch == '$'
            || ch == '∞'
            || (ch == '_' && self.dialect != Dialect::Wolfram)
        {
            self.bump();
            self.identifier_tail();
            return match &self.src[start..self.pos] {
                "let" if self.dialect != Dialect::Wolfram => K::Let,
                "where" if self.dialect != Dialect::Wolfram => K::Where,
                "and" if self.dialect != Dialect::Wolfram => K::And,
                "or" if self.dialect != Dialect::Wolfram => K::Or,
                "not" if self.dialect != Dialect::Wolfram => K::Not,
                _ => K::Identifier,
            };
        }
        for &(spelling, kind) in OPERATORS {
            if self.rest().starts_with(spelling) {
                self.pos += spelling.len();
                return kind;
            }
        }
        self.bump();
        self.error(start, "E001", "非法字符")
    }
    fn identifier_tail(&mut self) {
        loop {
            if self.peek().is_some_and(|c| {
                !superscript(c)
                    && (c.is_alphanumeric()
                        || c == '$'
                        || (c == '_' && self.dialect != Dialect::Wolfram))
            }) {
                self.bump();
            } else if let Some(rest) = self.rest().strip_prefix("\\[") {
                let end = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
                if rest[end..].starts_with(']')
                    && named::kind(&rest[..end]) == Some(TokenKind::Identifier)
                {
                    self.pos += end + 3;
                    continue;
                }
                break;
            } else {
                break;
            }
        }
    }
    fn comment(&mut self, start: usize) -> TokenKind {
        self.pos += 2;
        let mut depth = 1;
        while self.peek().is_some() {
            if self.rest().starts_with("(*") {
                self.pos += 2;
                depth += 1;
            } else if self.rest().starts_with("*)") {
                self.pos += 2;
                depth -= 1;
                if depth == 0 {
                    return TokenKind::Comment;
                }
            } else {
                self.bump();
            }
        }
        self.error(start, "E003", "注释缺少结束符 *)")
    }
    fn string(&mut self, start: usize) -> TokenKind {
        self.bump();
        let mut invalid_escape = false;
        while let Some(ch) = self.bump() {
            if ch == '"' {
                return if invalid_escape {
                    self.error(start, "E006", "非法字符串转义")
                } else {
                    TokenKind::String
                };
            }
            if ch == '\\' {
                match self.bump() {
                    Some('"' | '\\' | 'n' | 'r' | 't' | 'b' | 'f' | '/') => {}
                    Some('u') => {
                        for _ in 0..4 {
                            if self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                                self.bump();
                            } else {
                                invalid_escape = true;
                                break;
                            }
                        }
                    }
                    Some('[') if self.dialect == Dialect::Wolfram => {
                        let name_start = self.pos;
                        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                            self.bump();
                        }
                        let valid = named::value(&self.src[name_start..self.pos]).is_some();
                        if self.peek() == Some(']') {
                            self.bump();
                        } else {
                            invalid_escape = true;
                        }
                        invalid_escape |= !valid;
                    }
                    Some(_) => invalid_escape = true,
                    None => break,
                }
            }
        }
        self.error(start, "E002", "字符串缺少结束引号")
    }
    fn named_character(&mut self, start: usize) -> TokenKind {
        self.pos += 2;
        let name_start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.bump();
        }
        let name = &self.src[name_start..self.pos];
        if self.peek() == Some(']') {
            self.bump();
            if let Some(kind) = named::kind(name) {
                return kind;
            }
        } else {
            while self
                .peek()
                .is_some_and(|c| !is_newline(c) && !c.is_whitespace() && c != ']')
            {
                self.bump();
            }
            if self.peek() == Some(']') {
                self.bump();
            }
        }
        self.error(start, "E005", "未知或不完整的具名字符")
    }
}

fn is_newline(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
fn superscript(c: char) -> bool {
    matches!(
        c,
        '⁰' | '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '⁻'
    )
}

use TokenKind as K;
const OPERATORS: &[(&str, TokenKind)] = &[
    ("\u{f431}", K::Equal),
    ("\u{f522}", K::Rule),
    ("//.", K::ReplaceRepeated),
    ("===", K::SameQ),
    ("___", K::BlankNullSequence),
    ("__", K::BlankSequence),
    ("**", K::Power),
    ("==", K::Equal),
    ("!=", K::Unequal),
    ("<=", K::LessEqual),
    (">=", K::GreaterEqual),
    ("&&", K::And),
    ("||", K::Or),
    ("->", K::Rule),
    (":>", K::RuleDelayed),
    ("/.", K::ReplaceAll),
    (":=", K::SetDelayed),
    ("=.", K::Unset),
    ("/;", K::Condition),
    ("//", K::PostfixApply),
    ("@@", K::Apply),
    ("/@", K::Map),
    ("+", K::Plus),
    ("-", K::Minus),
    ("*", K::Star),
    ("/", K::Slash),
    ("^", K::Power),
    ("=", K::Assign),
    ("<", K::Less),
    (">", K::Greater),
    ("!", K::Bang),
    ("@", K::PrefixApply),
    ("'", K::Prime),
    ("&", K::Function),
    ("%", K::Out),
    ("|", K::Bar),
    ("_", K::Blank),
    ("√", K::Sqrt),
    ("∈", K::Element),
    ("≠", K::Unequal),
    ("≤", K::LessEqual),
    ("≥", K::GreaterEqual),
    ("∧", K::And),
    ("∨", K::Or),
    ("¬", K::Not),
    ("→", K::Rule),
    (":", K::Colon),
    (",", K::Comma),
    (";", K::Semicolon),
    ("(", K::LParen),
    (")", K::RParen),
    ("[", K::LBracket),
    ("]", K::RBracket),
    ("{", K::LBrace),
    ("}", K::RBrace),
];
