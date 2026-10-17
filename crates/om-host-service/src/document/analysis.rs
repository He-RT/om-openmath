//! Read-only native editor analysis. No preview references, source commits or evaluator are made.
use super::{coordinator::SourceCoordinator, coordinator::kernel_file};
use crate::protocol::{Serial, generated::NativeSourceFile};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Exact owner scope and a proposed native editor file; never an instruction to commit it.
pub struct SourceAnalysisInput {
    /// Existing source document identity.
    pub document_id: String,
    /// Existing host document generation.
    pub generation: Serial,
    /// Confirmed source snapshot identity.
    pub base_hash: String,
    /// Proposed editor sources, parsed without evaluation.
    pub file: NativeSourceFile,
}

#[derive(Serialize)]
/// Source-only facts bound to the input's confirmed owner version.
pub struct SourceAnalysisReply {
    /// Version of this read-only extension.
    pub protocol_version: u32,
    /// Analyzed document identity.
    pub document_id: String,
    /// Analyzed host generation.
    pub generation: Serial,
    /// Confirmed source snapshot hash supplied by the caller.
    pub base_hash: String,
    /// Original Rust parser diagnostics and dependency graph.
    pub analysis: om_kernel::source::SourceAnalysis,
}

/// All mutable owner data is cloned under the short source lock; parsing happens afterwards.
pub struct SourceAnalysisContext {
    /// Frozen document identity.
    pub document_id: String,
    /// Frozen document generation.
    pub generation: Serial,
    /// Frozen confirmed snapshot hash.
    pub base_hash: String,
    /// Frozen actual parser configuration and function bindings.
    pub coordinator: SourceCoordinator,
}
impl SourceAnalysisContext {
    /// Reject mismatched owner scope and parse the proposed file without retaining references.
    pub fn analyze(self, input: SourceAnalysisInput) -> Result<SourceAnalysisReply, String> {
        if self.document_id != input.document_id
            || self.generation != input.generation
            || self.base_hash != input.base_hash
        {
            return Err("STALE_SOURCE".into());
        }
        let analysis = om_kernel::source::analyze_source(
            &kernel_file(&input.file),
            self.coordinator.dialect,
            self.coordinator.constants,
            &self.coordinator.known_functions,
        )?;
        Ok(SourceAnalysisReply {
            protocol_version: 1,
            document_id: self.document_id,
            generation: self.generation,
            base_hash: self.base_hash,
            analysis,
        })
    }
}
