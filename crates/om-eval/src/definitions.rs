//! Ordered user definitions; replacement keeps the first insertion position.

use crate::Attributes;
use om_core::{Expr, Symbol};
use std::collections::BTreeMap;

/// An immediate or delayed downvalue with its original left-hand side.
#[derive(Clone, Debug)]
pub struct Rule {
    /// The definition target, including any pattern syntax.
    pub lhs: Expr,
    /// Stored right-hand side.
    pub rhs: Expr,
    /// Whether the right-hand side is evaluated only when the rule is used.
    pub delayed: bool,
}
/// Session-local ownvalues, downvalues and user attributes.
#[derive(Clone, Default)]
pub struct Definitions {
    pub(crate) own: BTreeMap<Symbol, Expr>,
    pub(crate) down: BTreeMap<Symbol, Vec<Rule>>,
    pub(crate) attrs: BTreeMap<Symbol, Attributes>,
}
impl Definitions {
    pub(crate) fn set_down(&mut self, head: Symbol, rule: Rule) {
        let rules = self.down.entry(head).or_default();
        if let Some(old) = rules.iter_mut().find(|old| old.lhs == rule.lhs) {
            *old = rule;
        } else {
            rules.push(rule);
        }
    }
    pub(crate) fn clear(&mut self, symbol: Symbol) {
        self.own.remove(&symbol);
        self.down.remove(&symbol);
    }
}
