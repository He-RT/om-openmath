//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ReadFunctionCatalog`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadFunctionCatalog {
    /// Contract field `kind`.
    pub kind: ReadFunctionCatalogKind,
}
impl std::fmt::Debug for ReadFunctionCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadFunctionCatalog { redacted }")
    }
}
