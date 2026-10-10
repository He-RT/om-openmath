//! Persistent evaluator data. Checkpoint acceptance/build provenance are caller responsibilities.
mod wire;
use crate::{Attributes, Evaluator, Rule};
use om_core::{
    Expr, Interrupt, Symbol,
    checkpoint::{ExprCodecError, ExprLimits, decode_expressions, encode_expressions},
};
use serde::{Deserialize, Serialize};
use wire::*;

/// Complete evaluator packet and metadata budgets, independent of the mathematical work budget.
#[derive(Clone, Copy, Debug)]
pub struct EvalStateLimits {
    /// Packet bytes, including metadata and graph.
    pub max_bytes: usize,
    /// Bounded closed JSON metadata bytes, parsed before the expression graph.
    pub max_metadata_bytes: usize,
    /// Entries per global definition/attribute/changed-symbol table.
    pub max_bindings: usize,
    /// Total ordered downvalue rules.
    pub max_rules: usize,
    /// Successful input/output history pairs.
    pub max_history: usize,
    /// Shared raw expression graph bounds.
    pub expressions: ExprLimits,
}
impl Default for EvalStateLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_metadata_bytes: 4 * 1024 * 1024,
            max_bindings: 8192,
            max_rules: 32768,
            max_history: 10000,
            expressions: ExprLimits::default(),
        }
    }
}
/// Decoder cannot return a writable state whose identity, shape or mathematical data is unproven.
#[derive(Debug, thiserror::Error)]
pub enum EvalStateError {
    /// Wrong packet/version/build, invalid metadata/indices/attributes or duplicate identities.
    #[error("invalid evaluator checkpoint")]
    Invalid,
    /// Explicit persistent-state budgets were exceeded.
    #[error("evaluator checkpoint exceeds its limits")]
    Limit,
    /// A readonly/in-flight evaluator is not a writable persistent boundary.
    #[error("evaluator checkpoint requires an idle writable owner")]
    NotIdle,
    /// Actual graph/number validation or cancellation.
    #[error(transparent)]
    Expr(#[from] ExprCodecError),
    /// Actual operation interruption.
    #[error(transparent)]
    Abort(#[from] om_core::Abort),
}
impl Evaluator {
    /// Save an actual readonly projection under a distinct role header. It cannot be decoded
    /// through the writable entry point, and no public writable clone is produced here.
    pub fn encode_readonly_persistent(
        &self,
        build: &str,
        limits: EvalStateLimits,
        ctx: &Interrupt,
    ) -> Result<Vec<u8>, EvalStateError> {
        self.encode_role(build, StateRole::Readonly, limits, ctx)
    }
    /// Restore an immutable projection whose header and metadata both declare the readonly role.
    pub fn decode_readonly_persistent(
        bytes: &[u8],
        build: &str,
        limits: EvalStateLimits,
        ctx: &Interrupt,
    ) -> Result<Self, EvalStateError> {
        Self::decode_role(bytes, build, StateRole::Readonly, limits, ctx)
    }
    /// Freeze current persistent mathematics without evaluating a source string or delayed RHS.
    pub fn encode_persistent(
        &self,
        build: &str,
        limits: EvalStateLimits,
        ctx: &Interrupt,
    ) -> Result<Vec<u8>, EvalStateError> {
        self.encode_role(build, StateRole::Writable, limits, ctx)
    }
    fn encode_role(
        &self,
        build: &str,
        role: StateRole,
        limits: EvalStateLimits,
        ctx: &Interrupt,
    ) -> Result<Vec<u8>, EvalStateError> {
        ctx.tick()?;
        if self.readonly != (role == StateRole::Readonly)
            || self.depth != 0
            || self.evaluating != 0
            || !self.scopes.is_empty()
        {
            return Err(EvalStateError::NotIdle);
        }
        if build.is_empty() || build.len() > 128 {
            return Err(EvalStateError::Invalid);
        }
        if [
            self.defs.own.len(),
            self.defs.down.len(),
            self.defs.attrs.len(),
            self.defs.changed.len(),
        ]
        .iter()
        .any(|&n| n > limits.max_bindings)
            || self.history.len() > limits.max_history
        {
            return Err(EvalStateError::Limit);
        }
        let mut roots = Vec::new();
        let mut push = |value: Expr| -> Result<u32, EvalStateError> {
            if roots.len() >= limits.expressions.max_roots {
                return Err(EvalStateError::Limit);
            }
            let id = u32::try_from(roots.len()).map_err(|_| EvalStateError::Limit)?;
            roots.push(value);
            Ok(id)
        };
        let mut own = Vec::new();
        let mut down = Vec::new();
        let mut attrs = Vec::new();
        let mut changed = Vec::new();
        let mut history = Vec::new();
        let mut entries = self.defs.own.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(symbol, _)| symbol.name());
        for (&symbol, value) in entries {
            ctx.tick()?;
            own.push(Binding {
                symbol: push(Expr::sym(symbol))?,
                value: push(value.clone())?,
            });
        }
        let mut entries = self.defs.down.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(symbol, _)| symbol.name());
        let mut rules = 0usize;
        for (&symbol, values) in entries {
            ctx.tick()?;
            rules = rules
                .checked_add(values.len())
                .filter(|&n| n <= limits.max_rules)
                .ok_or(EvalStateError::Limit)?;
            let key = push(Expr::sym(symbol))?;
            let mut stored = Vec::new();
            for rule in values {
                ctx.tick()?;
                stored.push(RuleWire {
                    lhs: push(rule.lhs.clone())?,
                    rhs: push(rule.rhs.clone())?,
                    delayed: rule.delayed,
                });
            }
            down.push(Down {
                symbol: key,
                rules: stored,
            });
        }
        let mut entries = self.defs.attrs.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(symbol, _)| symbol.name());
        for (&symbol, bits) in entries {
            ctx.tick()?;
            attrs.push(Attr {
                symbol: push(Expr::sym(symbol))?,
                bits: bits.bits(),
            });
        }
        let mut entries = self.defs.changed.iter().collect::<Vec<_>>();
        entries.sort_by_key(|symbol| symbol.name());
        for &symbol in entries {
            ctx.tick()?;
            changed.push(push(Expr::sym(symbol))?);
        }
        for (input, output) in &self.history {
            ctx.tick()?;
            history.push(History {
                input: push(input.clone())?,
                output: push(output.clone())?,
            });
        }
        let meta = Metadata {
            version: 1,
            role,
            build: build.into(),
            package: env!("CARGO_PKG_VERSION").into(),
            metadata_version: om_core::catalog::METADATA_VERSION,
            registry: registry()?,
            settings: Settings {
                iteration_limit: self.settings.iteration_limit,
                recursion_limit: self.settings.recursion_limit,
                record_steps: self.settings.record_steps,
            },
            random: self.random.state(),
            own,
            down,
            attrs,
            changed,
            history,
        };
        meta.validate(limits, roots.len())?;
        let metadata = serde_json::to_vec(&meta).map_err(|_| EvalStateError::Invalid)?;
        if metadata.len() > limits.max_metadata_bytes {
            return Err(EvalStateError::Limit);
        }
        let graph = encode_expressions(&roots, limits.expressions, ctx)?;
        let total = 13usize
            .checked_add(metadata.len())
            .and_then(|n| n.checked_add(graph.len()))
            .filter(|&n| n <= limits.max_bytes)
            .ok_or(EvalStateError::Limit)?;
        let mut out = Vec::with_capacity(total);
        out.extend(if role == StateRole::Writable {
            b"OMES\x01"
        } else {
            b"OMRS\x01"
        });
        out.extend(
            u32::try_from(metadata.len())
                .map_err(|_| EvalStateError::Limit)?
                .to_le_bytes(),
        );
        out.extend(
            u32::try_from(graph.len())
                .map_err(|_| EvalStateError::Limit)?
                .to_le_bytes(),
        );
        out.extend(metadata);
        out.extend(graph);
        Ok(out)
    }
    /// Restore writable data into a new independent evaluator, after exact build/registry checks.
    /// No definition evaluation, source parser, random advancement or owner replacement occurs.
    pub fn decode_persistent(
        bytes: &[u8],
        build: &str,
        limits: EvalStateLimits,
        ctx: &Interrupt,
    ) -> Result<Self, EvalStateError> {
        Self::decode_role(bytes, build, StateRole::Writable, limits, ctx)
    }
    fn decode_role(
        bytes: &[u8],
        build: &str,
        role: StateRole,
        limits: EvalStateLimits,
        ctx: &Interrupt,
    ) -> Result<Self, EvalStateError> {
        ctx.tick()?;
        if bytes.len() > limits.max_bytes {
            return Err(EvalStateError::Limit);
        }
        if bytes.len() < 13
            || &bytes[..5]
                != if role == StateRole::Writable {
                    b"OMES\x01"
                } else {
                    b"OMRS\x01"
                }
        {
            return Err(EvalStateError::Invalid);
        }
        let metadata_size = u32::from_le_bytes(
            bytes[5..9]
                .try_into()
                .map_err(|_| EvalStateError::Invalid)?,
        ) as usize;
        let graph_size = u32::from_le_bytes(
            bytes[9..13]
                .try_into()
                .map_err(|_| EvalStateError::Invalid)?,
        ) as usize;
        if metadata_size > limits.max_metadata_bytes || graph_size > limits.expressions.max_bytes {
            return Err(EvalStateError::Limit);
        }
        let split = 13usize
            .checked_add(metadata_size)
            .ok_or(EvalStateError::Invalid)?;
        if split.checked_add(graph_size) != Some(bytes.len()) || graph_size < 13 {
            return Err(EvalStateError::Invalid);
        }
        let meta: Metadata =
            serde_json::from_slice(&bytes[13..split]).map_err(|_| EvalStateError::Invalid)?;
        if meta.role != role
            || meta.build != build
            || meta.version != 1
            || meta.package != env!("CARGO_PKG_VERSION")
            || meta.metadata_version != om_core::catalog::METADATA_VERSION
            || meta.registry != registry()?
        {
            return Err(EvalStateError::Invalid);
        }
        let root_count = u32::from_le_bytes(
            bytes[split + 5..split + 9]
                .try_into()
                .map_err(|_| EvalStateError::Invalid)?,
        ) as usize;
        meta.validate(limits, root_count)?;
        let roots = decode_expressions(&bytes[split..], limits.expressions, ctx)?;
        let symbol = |id: u32| {
            roots[id as usize]
                .as_symbol()
                .ok_or(EvalStateError::Invalid)
        };
        let mut evaluator = Evaluator::new();
        let mut previous = None;
        for entry in meta.own {
            ctx.tick()?;
            let key = symbol(entry.symbol)?;
            ordered(key, &mut previous)?;
            evaluator
                .defs
                .own
                .insert(key, roots[entry.value as usize].clone());
        }
        previous = None;
        for entry in meta.down {
            ctx.tick()?;
            let key = symbol(entry.symbol)?;
            ordered(key, &mut previous)?;
            evaluator.defs.down.insert(
                key,
                entry
                    .rules
                    .into_iter()
                    .map(|rule| Rule {
                        lhs: roots[rule.lhs as usize].clone(),
                        rhs: roots[rule.rhs as usize].clone(),
                        delayed: rule.delayed,
                    })
                    .collect(),
            );
        }
        previous = None;
        for entry in meta.attrs {
            ctx.tick()?;
            let key = symbol(entry.symbol)?;
            ordered(key, &mut previous)?;
            evaluator.defs.attrs.insert(
                key,
                Attributes::from_bits(entry.bits).ok_or(EvalStateError::Invalid)?,
            );
        }
        previous = None;
        for id in meta.changed {
            ctx.tick()?;
            let key = symbol(id)?;
            ordered(key, &mut previous)?;
            evaluator.defs.changed.insert(key);
        }
        for pair in meta.history {
            ctx.tick()?;
            evaluator.history.push((
                roots[pair.input as usize].clone(),
                roots[pair.output as usize].clone(),
            ));
        }
        evaluator.settings.iteration_limit = meta.settings.iteration_limit;
        evaluator.settings.recursion_limit = meta.settings.recursion_limit;
        evaluator.settings.record_steps = meta.settings.record_steps;
        evaluator.random = om_num::rng::SplitMix64::new(meta.random);
        evaluator.readonly = role == StateRole::Readonly;
        Ok(evaluator)
    }
}
fn ordered(symbol: Symbol, previous: &mut Option<&'static str>) -> Result<(), EvalStateError> {
    let name = symbol.name();
    if previous.is_some_and(|old| old >= name) {
        return Err(EvalStateError::Invalid);
    }
    *previous = Some(name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_all_documented_user_attribute_bits_survive_codec_restore() {
        let mut original = Evaluator::new();
        let key = Symbol::intern("actual_attribute_owner");
        let flags = Attributes::ONE_IDENTITY
            | Attributes::LISTABLE
            | Attributes::HOLD_FIRST
            | Attributes::FLAT;
        original.defs.attrs.insert(key, flags);
        let ctx = Interrupt::default();
        let limits = EvalStateLimits::default();
        let bytes = original
            .encode_persistent("unit-build", limits, &ctx)
            .unwrap();
        let restored = Evaluator::decode_persistent(&bytes, "unit-build", limits, &ctx).unwrap();
        assert_eq!(restored.defs.attrs.get(&key), Some(&flags));
        assert!(Attributes::from_bits(1 << 9).is_none());
    }
    #[test]
    fn actual_machine_signed_zero_and_settings_do_not_pass_through_text_or_defaults() {
        let mut original = Evaluator::new();
        let key = Symbol::intern("saved_machine_zero");
        original.defs.own.insert(key, Expr::real(-0.0));
        original.settings.iteration_limit = 151;
        original.settings.recursion_limit = 17;
        original.settings.record_steps = false;
        let ctx = Interrupt::default();
        let limits = EvalStateLimits::default();
        let bytes = original
            .encode_persistent("unit-build", limits, &ctx)
            .unwrap();
        let restored = Evaluator::decode_persistent(&bytes, "unit-build", limits, &ctx).unwrap();
        let Some(om_num::Number::Real(om_num::Real::Machine(value))) =
            restored.defs.own_value(key).and_then(Expr::as_number)
        else {
            panic!("wrong number kind")
        };
        assert_eq!(value.to_bits(), (-0.0f64).to_bits());
        assert_eq!(restored.settings.iteration_limit, 151);
        assert_eq!(restored.settings.recursion_limit, 17);
        assert!(!restored.settings.record_steps);
    }
}
