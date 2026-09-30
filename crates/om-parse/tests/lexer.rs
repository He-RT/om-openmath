//! Shared-lexer language vectors, recovery and UTF-8 span checks.

use om_parse::{
    Dialect, Span, TokenClass,
    lexer::{TokenKind as K, lex},
};

fn tokens(src: &str, dialect: Dialect) -> Vec<(K, &str)> {
    let out = lex(src, dialect);
    assert!(out.diagnostics.is_empty(), "{src:?}: {:?}", out.diagnostics);
    out.tokens
        .iter()
        .map(|t| (t.kind, &src[t.span.start as usize..t.span.end as usize]))
        .collect()
}

#[test]
fn modern_numeric_spellings_and_adjacency() {
    let src = "42 3.14 1e-3 1.5e10 0x1F 1_000_000 .5 1. 2x 2e 2exp(x)";
    assert_eq!(
        tokens(src, Dialect::Modern),
        vec![
            (K::Number, "42"),
            (K::Number, "3.14"),
            (K::Number, "1e-3"),
            (K::Number, "1.5e10"),
            (K::Number, "0x1F"),
            (K::Number, "1_000_000"),
            (K::Number, ".5"),
            (K::Number, "1."),
            (K::Number, "2"),
            (K::Identifier, "x"),
            (K::Number, "2"),
            (K::Identifier, "e"),
            (K::Number, "2"),
            (K::Identifier, "exp"),
            (K::LParen, "("),
            (K::Identifier, "x"),
            (K::RParen, ")")
        ]
    );
}

#[test]
fn wolfram_precision_exponents_and_base_literals() {
    for spelling in [
        "1.5",
        "1.5*^-3",
        "1.5`30",
        "1.5`30*^-3",
        "2^^1011",
        "16^^ff",
        "1.`",
    ] {
        assert_eq!(
            tokens(spelling, Dialect::Wolfram),
            vec![(K::Number, spelling)]
        );
    }
}

#[test]
fn unicode_identifiers_have_byte_spans_and_stop_before_superscripts() {
    let src = "α β θ π 变量₂ x² x⁻¹ _value max_iterations xy $Failed ∞";
    let out = lex(src, Dialect::Modern);
    assert!(out.diagnostics.is_empty());
    assert_eq!(out.tokens[0].span, Span { start: 0, end: 2 });
    assert_eq!(
        tokens(src, Dialect::Modern),
        vec![
            (K::Identifier, "α"),
            (K::Identifier, "β"),
            (K::Identifier, "θ"),
            (K::Identifier, "π"),
            (K::Identifier, "变量₂"),
            (K::Identifier, "x"),
            (K::Superscript, "²"),
            (K::Identifier, "x"),
            (K::Superscript, "⁻¹"),
            (K::Identifier, "_value"),
            (K::Identifier, "max_iterations"),
            (K::Identifier, "xy"),
            (K::Identifier, "$Failed"),
            (K::Identifier, "∞")
        ]
    );
}

#[test]
fn modern_keywords_are_whole_words_and_wolfram_names_are_symbols() {
    assert_eq!(
        tokens(
            "let where and or not letter somewhere android origin nothing",
            Dialect::Modern
        )
        .iter()
        .map(|(k, _)| *k)
        .collect::<Vec<_>>(),
        vec![
            K::Let,
            K::Where,
            K::And,
            K::Or,
            K::Not,
            K::Identifier,
            K::Identifier,
            K::Identifier,
            K::Identifier,
            K::Identifier
        ]
    );
    assert!(
        tokens("let where and or not", Dialect::Wolfram)
            .iter()
            .all(|(k, _)| *k == K::Identifier)
    );
}

#[test]
fn all_operators_use_longest_match() {
    for (text, kind) in [
        ("+", K::Plus),
        ("-", K::Minus),
        ("*", K::Star),
        ("/", K::Slash),
        ("^", K::Power),
        ("**", K::Power),
        ("=", K::Assign),
        ("==", K::Equal),
        ("===", K::SameQ),
        ("!=", K::Unequal),
        ("≠", K::Unequal),
        ("<", K::Less),
        ("<=", K::LessEqual),
        ("≤", K::LessEqual),
        (">", K::Greater),
        (">=", K::GreaterEqual),
        ("≥", K::GreaterEqual),
        ("&&", K::And),
        ("∧", K::And),
        ("||", K::Or),
        ("∨", K::Or),
        ("!", K::Bang),
        ("¬", K::Not),
        ("->", K::Rule),
        ("→", K::Rule),
        (":>", K::RuleDelayed),
        ("/.", K::ReplaceAll),
        ("//.", K::ReplaceRepeated),
        (":=", K::SetDelayed),
        ("=.", K::Unset),
        ("/;", K::Condition),
        ("@", K::PrefixApply),
        ("//", K::PostfixApply),
        ("@@", K::Apply),
        ("/@", K::Map),
        ("'", K::Prime),
        ("&", K::Function),
        ("%", K::Out),
        ("|", K::Bar),
        ("√", K::Sqrt),
        ("∈", K::Element),
        (":", K::Colon),
        (",", K::Comma),
        (";", K::Semicolon),
        ("(", K::LParen),
        (")", K::RParen),
        ("[", K::LBracket),
        ("]", K::RBracket),
        ("{", K::LBrace),
        ("}", K::RBrace),
        ("_", K::Blank),
        ("__", K::BlankSequence),
        ("___", K::BlankNullSequence),
    ] {
        assert_eq!(tokens(text, Dialect::Wolfram), vec![(kind, text)]);
    }
    assert_eq!(
        tokens("//././@//@@===:=:>=.**___", Dialect::Wolfram)
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>(),
        vec![
            K::ReplaceRepeated,
            K::ReplaceAll,
            K::Map,
            K::PostfixApply,
            K::Apply,
            K::SameQ,
            K::SetDelayed,
            K::RuleDelayed,
            K::Unset,
            K::Power,
            K::BlankNullSequence
        ]
    );
}

#[test]
fn hash_and_underscore_follow_the_selected_dialect() {
    assert_eq!(
        tokens("#1 x_\n# comment", Dialect::Modern),
        vec![
            (K::Comment, "#1 x_"),
            (K::Newline, "\n"),
            (K::Comment, "# comment")
        ]
    );
    assert_eq!(
        tokens("# #1 #12 x_ x_Integer x__ x___", Dialect::Wolfram),
        vec![
            (K::Slot, "#"),
            (K::Slot, "#1"),
            (K::Slot, "#12"),
            (K::Identifier, "x"),
            (K::Blank, "_"),
            (K::Identifier, "x"),
            (K::Blank, "_"),
            (K::Identifier, "Integer"),
            (K::Identifier, "x"),
            (K::BlankSequence, "__"),
            (K::Identifier, "x"),
            (K::BlankNullSequence, "___")
        ]
    );
    assert_eq!(
        tokens("# auto\n", Dialect::Auto),
        tokens("# auto\n", Dialect::Modern)
    );
}

#[test]
fn strings_preserve_spelling_and_protect_comment_delimiters() {
    for text in [
        r#""a\"b\\c\n\t\r""#,
        "\"中文 α\"",
        r##""# (* *) \u03b1""##,
        "\"line\nnext\"",
    ] {
        assert_eq!(tokens(text, Dialect::Modern), vec![(K::String, text)]);
    }
}

#[test]
fn comments_nest_and_leave_statement_newlines() {
    let src = "x (* outer (* inner *) outer *) # end\r\ny\rz\n";
    assert_eq!(
        tokens(src, Dialect::Modern),
        vec![
            (K::Identifier, "x"),
            (K::Comment, "(* outer (* inner *) outer *)"),
            (K::Comment, "# end"),
            (K::Newline, "\r\n"),
            (K::Identifier, "y"),
            (K::Newline, "\r"),
            (K::Identifier, "z"),
            (K::Newline, "\n")
        ]
    );
    let nested = format!("{}{}", "(*".repeat(10_000), "*)".repeat(10_000));
    assert_eq!(
        tokens(&nested, Dialect::Wolfram),
        vec![(K::Comment, nested.as_str())]
    );
}

#[test]
fn named_characters_cover_required_greek_constants_and_operators() {
    for name in [
        "Pi", "Infinity", "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta",
        "Iota", "Kappa", "Lambda", "Mu", "Nu", "Xi", "Omicron", "Rho", "Sigma", "Tau", "Upsilon",
        "Phi", "Chi", "Psi", "Omega", "alpha", "omega",
    ] {
        let text = format!("\\[{name}]");
        assert_eq!(
            tokens(&text, Dialect::Wolfram),
            vec![(K::Identifier, text.as_str())]
        );
    }
    for (name, kind) in [
        ("Element", K::Element),
        ("Equal", K::Equal),
        ("LessEqual", K::LessEqual),
        ("GreaterEqual", K::GreaterEqual),
        ("NotEqual", K::Unequal),
        ("Rule", K::Rule),
    ] {
        let text = format!("\\[{name}]");
        assert_eq!(tokens(&text, Dialect::Wolfram), vec![(kind, text.as_str())]);
    }
}

#[test]
fn named_letters_can_be_part_of_a_single_identifier() {
    let src = r"a\[Alpha]b \[Alpha]x \[Alpha]\[Beta] x\[Equal]y";
    assert_eq!(
        tokens(src, Dialect::Wolfram),
        vec![
            (K::Identifier, r"a\[Alpha]b"),
            (K::Identifier, r"\[Alpha]x"),
            (K::Identifier, r"\[Alpha]\[Beta]"),
            (K::Identifier, "x"),
            (K::Equal, r"\[Equal]"),
            (K::Identifier, "y")
        ]
    );
}

#[test]
fn brackets_stay_individual_and_adjacency_is_recoverable() {
    assert_eq!(
        tokens("[[1]]", Dialect::Modern)
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>(),
        vec![
            K::LBracket,
            K::LBracket,
            K::Number,
            K::RBracket,
            K::RBracket
        ]
    );
    let out = lex("f(x) f (x) v[1]", Dialect::Modern);
    assert_eq!(out.tokens[0].span.end, out.tokens[1].span.start);
    assert!(out.tokens[4].span.end < out.tokens[5].span.start);
}

#[test]
fn malformed_literals_report_whole_byte_spans_and_recover() {
    for (src, code, end) in [
        ("α ☃ + 2", "E001", 6),
        ("\"oops", "E002", 5),
        ("(* outer (* nested *)", "E003", 21),
        ("0x + 2", "E004", 2),
        ("1__2 + 3", "E004", 4),
        ("1e- + 2", "E004", 3),
        ("1.5*^ + 2", "E004", 5),
        ("2^^102 + 3", "E004", 6),
        ("\\[Unknown] + 2", "E005", 10),
        ("\"bad\\q\" + 2", "E006", 7),
    ] {
        let out = lex(src, Dialect::Modern);
        assert_eq!(out.diagnostics.len(), 1, "{src}: {:?}", out.diagnostics);
        assert_eq!(out.diagnostics[0].code, code, "{src}");
        assert_eq!(out.diagnostics[0].span.end, end, "{src}");
        assert!(out.tokens.iter().any(|t| t.kind == K::Error), "{src}");
        if src.ends_with("+ 2") {
            assert_eq!(out.tokens.last().unwrap().kind, K::Number);
        }
    }
}

#[test]
fn highlights_follow_token_categories() {
    let out = lex("let α = [1, \"x\"] # note\n☃", Dialect::Modern);
    assert_eq!(
        out.tokens.iter().map(|t| t.class).collect::<Vec<_>>(),
        vec![
            TokenClass::Keyword,
            TokenClass::Identifier,
            TokenClass::Operator,
            TokenClass::Bracket,
            TokenClass::Number,
            TokenClass::Operator,
            TokenClass::String,
            TokenClass::Bracket,
            TokenClass::Comment,
            TokenClass::Operator,
            TokenClass::Error
        ]
    );
}

#[test]
fn arbitrary_small_inputs_terminate_and_only_emit_valid_ordered_utf8_spans() {
    let alphabet = [
        'α', '☃', '"', '\\', '[', ']', '(', ')', '*', '^', '`', '_', '#', '\r', '\n', '1', 'e',
        '+', '⁻',
    ];
    for a in alphabet {
        for b in alphabet {
            for c in alphabet {
                let src: String = [a, b, c].into_iter().collect();
                for dialect in [Dialect::Modern, Dialect::Wolfram] {
                    let out = lex(&src, dialect);
                    let mut end = 0;
                    for token in &out.tokens {
                        assert!(token.span.start >= end && token.span.start < token.span.end);
                        assert!(
                            src.get(token.span.start as usize..token.span.end as usize)
                                .is_some()
                        );
                        end = token.span.end;
                    }
                    for d in &out.diagnostics {
                        assert!(
                            src.get(d.span.start as usize..d.span.end as usize)
                                .is_some()
                        );
                    }
                }
            }
        }
    }
}
