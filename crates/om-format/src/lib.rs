//! Mathematical expression formatting.
#![forbid(unsafe_code)]

mod source;
mod text;

use om_core::Expr;

/// Preferences for mathematical display, independent of canonical tree order.
pub struct FormatOptions {
    /// Display terms in descending polynomial degree. Defaults to true.
    pub display_order: bool,
}
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            display_order: true,
        }
    }
}

/// Structural FullForm using the same spelling as the core's Debug contract.
pub fn full_form(e: &Expr) -> String {
    format!("{e:?}")
}

/// Wolfram source for a canonical expression; normalize after reparsing.
pub fn input_form(e: &Expr) -> String {
    input_form_with(e, &FormatOptions::default())
}

/// Modern source for a canonical expression; normalize after reparsing.
/// Explicit precision markers preserve Machine/Big values and bit precision.
pub fn modern_form(e: &Expr) -> String {
    modern_form_with(e, &FormatOptions::default())
}

/// Wolfram source using explicit display preferences.
pub fn input_form_with(e: &Expr, options: &FormatOptions) -> String {
    source::format(e, false, options)
}
/// Modern source using explicit display preferences.
pub fn modern_form_with(e: &Expr, options: &FormatOptions) -> String {
    source::format(e, true, options)
}
