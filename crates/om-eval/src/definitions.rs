//! Ordered user definitions; replacement keeps the first insertion position.

use crate::Attributes;
use om_core::{Expr, Symbol};
use std::collections::{BTreeMap, BTreeSet};

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
    pub(crate) changed: BTreeSet<Symbol>,
}
impl Definitions {
    /// Actual stored global ownvalue, including delayed syntax, without evaluation.
    pub fn own_value(&self, symbol: Symbol) -> Option<&Expr> {
        self.own.get(&symbol)
    }
    /// Actual global downvalues in definition order, without evaluating patterns or RHSs.
    pub fn down_values(&self, symbol: Symbol) -> &[Rule] {
        self.down.get(&symbol).map_or(&[], Vec::as_slice)
    }
    /// Symbols with actual ownvalues or nonempty downvalues, without evaluating them.
    pub fn defined_symbols(&self) -> BTreeSet<Symbol> {
        self.own
            .keys()
            .copied()
            .chain(
                self.down
                    .iter()
                    .filter(|(_, rules)| !rules.is_empty())
                    .map(|(&s, _)| s),
            )
            .collect()
    }
    /// Take global definition mutations since the previous observation.
    /// Lexical evaluator bindings are excluded; identical writes are included.
    pub fn take_changed_symbols(&mut self) -> BTreeSet<Symbol> {
        std::mem::take(&mut self.changed)
    }
    /// Current function heads and literal callable aliases, without evaluating definitions.
    /// Ownvalues shadow downvalues; alias cycles and non-callable values are excluded.
    pub fn known_functions(&self) -> BTreeSet<Symbol> {
        let heads: BTreeSet<_> = self.down.keys().chain(self.own.keys()).copied().collect();
        let mut known = BTreeSet::new();
        let mut memo = BTreeMap::new();
        for head in heads {
            let mut cursor = head;
            let mut path = BTreeSet::new();
            let callable = loop {
                if let Some(&callable) = memo.get(&cursor) {
                    break callable;
                }
                if !path.insert(cursor) {
                    break false;
                }
                if let Some(value) = self.own.get(&cursor) {
                    if value.is_head(om_core::BUILTIN::FUNCTION)
                        || value.is_head(om_core::BUILTIN::INTERPOLATION_DATA)
                    {
                        break true;
                    }
                    if let Some(alias) = value.as_symbol() {
                        cursor = alias;
                    } else {
                        break false;
                    }
                } else {
                    break self
                        .down
                        .get(&cursor)
                        .is_some_and(|rules| !rules.is_empty())
                        || crate::builtins::table().get(cursor).is_some();
                }
            };
            for symbol in path {
                memo.insert(symbol, callable);
            }
            if callable {
                known.insert(head);
            }
        }
        known
    }
    pub(crate) fn set_down(&mut self, head: Symbol, rule: Rule) {
        self.changed.insert(head);
        let rules = self.down.entry(head).or_default();
        if let Some(old) = rules.iter_mut().find(|old| old.lhs == rule.lhs) {
            *old = rule;
        } else {
            rules.push(rule);
        }
    }
    /// Remove ownvalues and downvalues without evaluation, messages or history.
    pub fn clear(&mut self, symbol: Symbol) {
        self.changed.insert(symbol);
        self.own.remove(&symbol);
        self.down.remove(&symbol);
    }
}
