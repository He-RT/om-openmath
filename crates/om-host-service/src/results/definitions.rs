//! Workbench definitions are a projection of an accepted checkpoint, never a source evaluation.
use crate::{
    document::{analysis::SourceAnalysisInput, coordinator::kernel_file},
    kernel::acceptance::AcceptedKernelState,
    protocol::Serial,
};
use om_kernel::{
    checkpoint::{CheckpointLimits, CheckpointRestore},
    protocol::{CellStatus, Request, Response},
};
use om_num::ctx::Interrupt;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicBool},
};

/// One actual stored definition, or an unbound declaration in an actually failed cell.
#[derive(Clone, Debug, Serialize)]
pub struct DefinitionItem {
    /// Actual symbol name.
    pub name: String,
    /// Unevaluated InputForm, at most 120 Unicode scalar values including an ellipsis.
    pub value: Option<String>,
    /// Successful write owner, or the failed declaration owner.
    pub cell_id: String,
    /// Its source/dependencies or accepted cell state are no longer current.
    pub stale: bool,
    /// No value was accepted for this declaration in an errored cell.
    pub error: bool,
}
/// Scope permits the native UI to discard replies after a source or document change.
#[derive(Serialize)]
pub struct DefinitionsReply {
    /// Version of this independent readonly extension.
    pub protocol_version: u32,
    /// Actual document identity.
    pub document_id: String,
    /// Actual open lifetime.
    pub generation: Serial,
    /// Confirmed source identity behind the proposed editor file.
    pub base_hash: String,
    /// Actual accepted checkpoint, absent before the first bootstrap.
    pub checkpoint_id: Option<String>,
    /// Actual accepted mathematical revision.
    pub kernel_state_revision: Serial,
    /// Sorted stored definitions and failed declarations.
    pub items: Vec<DefinitionItem>,
}

/// Retains only compact strings for the most recently queried immutable checkpoint.
#[derive(Default)]
pub(super) struct DefinitionsCache {
    checkpoint: String,
    items: Vec<DefinitionItem>,
}
impl DefinitionsCache {
    /// This method runs exclusively on the existing independent result worker.
    pub(super) fn list(
        &mut self,
        input: SourceAnalysisInput,
        accepted: Option<&AcceptedKernelState>,
        general: &om_kernel::config::GeneralConfig,
        ctx: &Interrupt,
    ) -> Result<DefinitionsReply, String> {
        ctx.tick().map_err(|e| e.to_string())?;
        let mut reply = DefinitionsReply {
            protocol_version: 1,
            document_id: input.document_id,
            generation: input.generation,
            base_hash: input.base_hash,
            checkpoint_id: None,
            kernel_state_revision: Serial::new(0)?,
            items: vec![],
        };
        let Some(accepted) = accepted else {
            return Ok(reply);
        };
        let state = accepted.state();
        if state.binding().document_id != reply.document_id
            || state.binding().document_generation != reply.generation.get()
        {
            return Err("STALE_DOCUMENT".into());
        }
        if self.checkpoint != state.hash() {
            let mut session = state
                .restore(
                    CheckpointRestore {
                        binding: state.binding(),
                        source: state.source(),
                        general: state.general(),
                        clock: None,
                        cancel: Arc::new(AtomicBool::new(false)),
                    },
                    CheckpointLimits::default(),
                    ctx,
                )
                .map_err(|e| e.to_string())?
                .into_session();
            // GetVariables reads the definition table and formats stored expressions only.
            let Response::Variables { items } = session.handle(Request::GetVariables).0 else {
                return Err("INVALID_DEFINITIONS".into());
            };
            let mut definitions = items
                .into_iter()
                .filter_map(|(name, info)| {
                    Some((
                        name.clone(),
                        DefinitionItem {
                            name,
                            value: info.value.map(|value| summary(&value)),
                            cell_id: info.cell_id?,
                            stale: false,
                            error: false,
                        },
                    ))
                })
                .collect::<BTreeMap<_, _>>();
            let analysis = om_kernel::source::analyze_source(
                state.source(),
                state.general().dialect,
                state.general().constants,
                &state.execution_context().known_functions,
            )?;
            for (cell, status) in state.cell_states() {
                if *status == CellStatus::Error {
                    for name in analysis
                        .cells
                        .iter()
                        .filter(|c| c.cell_id == *cell)
                        .flat_map(|c| &c.defines)
                    {
                        definitions
                            .entry(name.clone())
                            .or_insert_with(|| DefinitionItem {
                                name: name.clone(),
                                value: None,
                                cell_id: cell.clone(),
                                stale: false,
                                error: true,
                            });
                    }
                }
            }
            self.items = definitions.into_values().collect();
            self.checkpoint = state.hash().into();
        }
        let changed = om_kernel::source::assess_source_change(
            state.source(),
            &kernel_file(&input.file),
            general.dialect,
            general.constants,
            &state.execution_context().known_functions,
            &state.execution_context().owned,
            serde_json::to_value(general).map_err(|_| "INVALID_CONFIGURATION")?
                != serde_json::to_value(state.general()).map_err(|_| "INVALID_CONFIGURATION")?,
        )?;
        let stale = changed
            .affected_cells
            .into_iter()
            .chain(
                state
                    .cell_states()
                    .iter()
                    .filter(|(_, status)| *status == CellStatus::Stale)
                    .map(|(id, _)| id.clone()),
            )
            .collect::<std::collections::BTreeSet<_>>();
        reply.checkpoint_id = Some(accepted.receipt().checkpoint_id.clone());
        reply.kernel_state_revision = accepted.receipt().kernel_state_revision;
        reply.items = self
            .items
            .iter()
            .cloned()
            .map(|mut item| {
                item.stale = stale.contains(&item.cell_id);
                item
            })
            .collect();
        Ok(reply)
    }
}
fn summary(value: &str) -> String {
    if value.chars().count() <= 120 {
        value.into()
    } else {
        value
            .chars()
            .take(119)
            .chain(std::iter::once('…'))
            .collect()
    }
}
