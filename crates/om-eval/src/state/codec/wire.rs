use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Metadata {
    pub version: u32,
    pub build: String,
    pub package: String,
    pub metadata_version: u32,
    pub registry: Vec<Registry>,
    pub settings: Settings,
    pub random: u64,
    pub own: Vec<Binding>,
    pub down: Vec<Down>,
    pub attrs: Vec<Attr>,
    pub changed: Vec<u32>,
    pub history: Vec<History>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Settings {
    pub iteration_limit: u32,
    pub recursion_limit: u32,
    pub record_steps: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub symbol: u32,
    pub value: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Down {
    pub symbol: u32,
    pub rules: Vec<RuleWire>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RuleWire {
    pub lhs: u32,
    pub rhs: u32,
    pub delayed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Attr {
    pub symbol: u32,
    pub bits: u16,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    pub input: u32,
    pub output: u32,
}
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Registry {
    pub id: String,
    pub name: String,
    pub bits: u16,
    pub min: u32,
    pub max: Option<u32>,
}
pub(super) fn registry() -> Result<Vec<Registry>, EvalStateError> {
    crate::Evaluator::all_specs()
        .map(|spec| {
            let name = spec.symbol.name();
            let descriptor = om_core::catalog::by_runtime(name).ok_or(EvalStateError::Invalid)?;
            let (min, max) = match spec.arity {
                crate::Arity::Exactly(n) => (u32::from(n), Some(u32::from(n))),
                crate::Arity::Range(a, b) => (u32::from(a), Some(u32::from(b))),
                crate::Arity::AtLeast(n) => (u32::from(n), None),
                crate::Arity::Any => (0, None),
            };
            Ok(Registry {
                id: descriptor.id.clone(),
                name: name.into(),
                bits: spec.attrs.bits(),
                min,
                max,
            })
        })
        .collect()
}
impl Metadata {
    pub(super) fn validate(
        &self,
        limits: EvalStateLimits,
        roots: usize,
    ) -> Result<(), EvalStateError> {
        if roots > limits.expressions.max_roots
            || [
                self.own.len(),
                self.down.len(),
                self.attrs.len(),
                self.changed.len(),
            ]
            .iter()
            .any(|&n| n > limits.max_bindings)
            || self.history.len() > limits.max_history
        {
            return Err(EvalStateError::Limit);
        }
        if self.build.is_empty()
            || self.build.len() > 128
            || self.settings.recursion_limit == 0
            || self.settings.iteration_limit == 0
        {
            return Err(EvalStateError::Invalid);
        }
        let mut used = vec![false; roots];
        let mut reference = |id: u32| -> Result<(), EvalStateError> {
            let slot = used.get_mut(id as usize).ok_or(EvalStateError::Invalid)?;
            if *slot {
                return Err(EvalStateError::Invalid);
            }
            *slot = true;
            Ok(())
        };
        let mut rules = 0usize;
        for entry in &self.own {
            reference(entry.symbol)?;
            reference(entry.value)?;
        }
        for entry in &self.down {
            reference(entry.symbol)?;
            rules = rules
                .checked_add(entry.rules.len())
                .filter(|&n| n <= limits.max_rules)
                .ok_or(EvalStateError::Limit)?;
            for rule in &entry.rules {
                reference(rule.lhs)?;
                reference(rule.rhs)?;
            }
        }
        for entry in &self.attrs {
            reference(entry.symbol)?;
            if crate::Attributes::from_bits(entry.bits).is_none() {
                return Err(EvalStateError::Invalid);
            }
        }
        for id in &self.changed {
            reference(*id)?;
        }
        for pair in &self.history {
            reference(pair.input)?;
            reference(pair.output)?;
        }
        if used.iter().any(|seen| !seen) {
            return Err(EvalStateError::Invalid);
        }
        Ok(())
    }
}
