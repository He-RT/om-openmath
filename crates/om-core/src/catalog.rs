//! Executable function metadata generated from the audited documentation catalog.
//! Descriptions never grant permission: argument evaluation may access session state.
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Accepted raw argument counts (including Wolfram option rules).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "catalog-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "catalog-ts", ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/")))]
pub struct AritySchema {
    /// Inclusive minimum.
    pub min: u32,
    /// Inclusive maximum; absent for variadic calls.
    pub max: Option<u32>,
}
/// Machine-verifiable syntax value types; expressions remain unevaluated during parsing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "catalog-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "catalog-ts", ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/")))]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// Arbitrary mathematical source expression.
    Expression,
    /// True or False.
    Boolean,
    /// Nonnegative or bounded integer.
    Integer,
    /// Finite real number.
    Real,
    /// String or a listed symbol spelling.
    Enum,
    /// Symbol, for example a generated-parameter head.
    Symbol,
}
/// One positional argument or named option with actual source defaults.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "catalog-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "catalog-ts", ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/")))]
pub struct ParameterSchema {
    /// Preferred source name.
    pub name: String,
    /// Wolfram rule key for an option, absent for positional parameters.
    pub runtime_name: Option<String>,
    /// Syntax value type; symbolic values are checked after evaluation by the callback.
    pub value_type: ParameterType,
    /// Required in this invocation.
    pub required: bool,
    /// Actual default expression in Wolfram InputForm, absent when inferred or context dependent.
    pub default_source: Option<String>,
    /// Reason a default is contextual, instead of inventing a literal value.
    pub default_context: Option<String>,
    /// Permitted literal names, if restricted.
    pub enum_values: Vec<String>,
    /// Inclusive lower bound for numeric literals.
    pub min: Option<f64>,
    /// Inclusive upper bound for numeric literals.
    pub max: Option<f64>,
    /// Repeated argument role.
    pub variadic: bool,
    /// This role is held by the evaluator.
    pub held: bool,
}
/// Conservative top-level effect classification; nested expressions still require isolation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "catalog-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "catalog-ts", ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/")))]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    /// No direct mutation or host IO.
    Pure,
    /// Reads stored session values.
    ReadSession,
    /// Can change definitions or session random state.
    WriteSession,
    /// Modifies the document through a future host operation.
    WriteDocument,
    /// Requires a host IO capability.
    HostIo,
    /// Unknown effect; never grants execution rights.
    Unclassified,
}
/// Description of one actual callback, with an identity shared by equivalent invocation variants.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "catalog-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "catalog-ts", ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/")))]
pub struct FunctionDescriptor {
    /// Stable semantic identity.
    pub id: String,
    /// Preferred family name; variants remain distinct until unified modes exist.
    pub canonical_name: String,
    /// Actual callback symbol.
    pub name: String,
    /// Preferred currently executable modern invocation.
    pub modern_name: String,
    /// Accepted additional spellings; no planned names are included.
    pub aliases: Vec<String>,
    /// Raw callback arity.
    pub arity: AritySchema,
    /// Positional parameter roles.
    pub parameters: Vec<ParameterSchema>,
    /// Currently accepted named options.
    pub options: Vec<ParameterSchema>,
    /// Historical syntax-only options; callbacks still diagnose unsupported behavior.
    pub compatibility_syntax: Vec<String>,
    /// Actual evaluator attributes.
    pub attributes: Vec<String>,
    /// One-based main input position; zero forbids automatic pipe insertion.
    pub pipe_arg: u32,
    /// Direct callback effects; not an authorization claim.
    pub effect_class: EffectClass,
    /// Mathematical support boundary in Chinese.
    pub support: String,
    /// Return description, without changing existing result shapes.
    pub return_type: String,
    /// Actually supported precision categories, with qualifications below.
    pub precision_modes: Vec<String>,
    /// Precision limitations.
    pub precision_notes: String,
    /// Current computation platforms, separate from host rendering support.
    pub platforms: Vec<String>,
}
/// Version of the executable descriptor schema.
pub const SCHEMA_VERSION: u32 = 1;
/// Description revision; independent of the application release version.
pub const METADATA_VERSION: u32 = 20;
static FUNCTIONS: LazyLock<Vec<FunctionDescriptor>> = LazyLock::new(|| {
    // The generated JSON is validated by Python and registry contract tests in CI.
    serde_json::from_str(include_str!("catalog/runtime.json"))
        .expect("invariant: checked executable catalog JSON matches FunctionDescriptor")
});
static DOCUMENTATION: LazyLock<serde_json::Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("catalog/documentation.json"))
        .expect("invariant: checked generated documentation catalog is JSON")
});
/// Audited documentation, including explicit planned/deferred entries; never an executable tool schema.
pub fn documentation() -> &'static serde_json::Value {
    &DOCUMENTATION
}
/// Actual callback descriptions in deterministic runtime-name order.
pub fn functions() -> &'static [FunctionDescriptor] {
    &FUNCTIONS
}
/// Find an actual callback by its case-sensitive runtime symbol.
pub fn by_runtime(name: &str) -> Option<&'static FunctionDescriptor> {
    functions().iter().find(|entry| entry.name == name)
}
/// Resolve only implemented modern aliases; short names are never reserved by this table.
pub fn by_alias(name: &str) -> Option<&'static FunctionDescriptor> {
    if name.chars().count() < 2 {
        return None;
    }
    functions().iter().find(|entry| {
        entry.modern_name.eq_ignore_ascii_case(name)
            || entry.aliases.iter().any(|s| s.eq_ignore_ascii_case(name))
    })
}
