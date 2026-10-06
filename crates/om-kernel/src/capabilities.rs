//! Readonly executable descriptions; host rendering and task authorization stay separate.
use om_core::catalog::FunctionDescriptor;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Host presentation target requested by the caller; it does not grant permissions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum HostPlatform {
    /// Terminal output.
    Cli,
    /// Tauri desktop UI.
    Desktop,
    /// Browser UI.
    Web,
    /// Native iOS/iPadOS UI.
    Ios,
}
/// A versioned catalog containing only real callbacks.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct FunctionCatalog {
    /// Descriptor format version.
    pub schema_version: u32,
    /// Content revision.
    pub metadata_version: u32,
    /// Actual callable callback variants.
    pub functions: Vec<FunctionDescriptor>,
}
/// Independent kernel, host presentation and task capability information.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct CapabilityInfo {
    /// Compiled kernel release version.
    pub kernel_version: String,
    /// Descriptor format version.
    pub schema_version: u32,
    /// Function descriptor revision.
    pub metadata_version: u32,
    /// Implemented semantic identities; invocation variants are listed in the catalog.
    pub function_ids: Vec<String>,
    /// Host whose documented presentation support was queried.
    pub platform: HostPlatform,
    /// Actual supported UI output families.
    pub rendered_outputs: Vec<String>,
    /// Native scene interaction is unavailable until the 3D milestone passes its gates.
    pub scene_3d: bool,
    /// Current explicit host artifact formats, independent of notebook/agent write permissions.
    #[serde(default)]
    pub export_formats: Vec<String>,
    /// None: this query does not implement or grant future Notebook Agent permissions.
    pub task_permissions: Option<Vec<String>>,
}
/// Build a callback-filtered catalog instead of treating documentation as executable tools.
pub fn function_catalog() -> FunctionCatalog {
    FunctionCatalog {
        schema_version: om_core::catalog::SCHEMA_VERSION,
        metadata_version: om_core::catalog::METADATA_VERSION,
        functions: om_eval::Evaluator::all_specs()
            .filter_map(|spec| om_core::catalog::by_runtime(spec.symbol.name()).cloned())
            .collect(),
    }
}
/// Current presentation support, independent of settings, credentials and notebook contents.
pub fn capabilities(platform: HostPlatform) -> CapabilityInfo {
    let catalog = function_catalog();
    let ids: std::collections::BTreeSet<_> =
        catalog.functions.iter().map(|f| f.id.clone()).collect();
    let rendered_outputs = if matches!(platform, HostPlatform::Cli) {
        vec!["expression", "solutions", "steps"]
    } else {
        vec![
            "expression",
            "solutions",
            "steps",
            "plot_2d",
            "markdown",
            "table",
            "record",
            "numerical_diagnostics",
            "parameter_exploration",
        ]
    };
    CapabilityInfo {
        kernel_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: catalog.schema_version,
        metadata_version: catalog.metadata_version,
        function_ids: ids.into_iter().collect(),
        platform,
        rendered_outputs: rendered_outputs.into_iter().map(String::from).collect(),
        scene_3d: false,
        export_formats: ["svg", "png", "csv", "json"]
            .into_iter()
            .map(String::from)
            .collect(),
        task_permissions: None,
    }
}
