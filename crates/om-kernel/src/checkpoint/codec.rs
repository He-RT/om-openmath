//! This file is a child of Session so private math/IO fields never become public DTO ownership.
use super::{Session, WorkingSession};
use crate::{
    KernelConfig, Notebook,
    checkpoint::{
        CheckpointBinding, CheckpointError, CheckpointLimits, CheckpointRestore, records::*,
        wire::*,
    },
    notebook::Cell,
    protocol::{CellKind, CellStatus, NotebookFile},
};
use om_core::{
    Expr, Interrupt, Symbol,
    checkpoint::{decode_expressions, encode_expressions},
};
use std::collections::{BTreeMap, BTreeSet};

impl Session {
    /// Encode actual idle source/evaluator/owners/retained records under trusted host provenance.
    /// Credential profiles, LLM jobs, file stores, pointers and the operation cancel token are omitted.
    pub fn encode_checkpoint(
        &self,
        binding: &CheckpointBinding,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<Vec<u8>, CheckpointError> {
        ctx.tick()?;
        binding.validate()?;
        let file = self.notebook.to_file();
        source_limits(&file, limits)?;
        if self
            .notebook
            .cells
            .iter()
            .any(|c| c.status == CellStatus::Running)
        {
            return Err(CheckpointError::NotIdle);
        }
        if self.output_serial > ((1u64 << 53) - 1) {
            return Err(CheckpointError::Limit);
        }
        let evaluator = self
            .eval
            .encode_persistent(&binding.build, limits.evaluator, ctx)?;
        let mut roots = Vec::new();
        let mut contexts = Vec::new();
        let mut context_bytes = Vec::new();
        let mut cells = Vec::new();
        let mut record_count = 0usize;
        for cell in &self.notebook.cells {
            ctx.tick()?;
            record_count = record_count
                .checked_add(cell.records.len())
                .filter(|&n| n <= limits.max_records)
                .ok_or(CheckpointError::Limit)?;
            let symbols = |set: &BTreeSet<Symbol>, roots: &mut Vec<Expr>| {
                let mut names = set.iter().copied().collect::<Vec<_>>();
                names.sort_by_key(|s| s.name());
                names
                    .into_iter()
                    .map(|s| push(roots, &Expr::sym(s), limits, ctx))
                    .collect::<Result<Vec<_>, CheckpointError>>()
            };
            let defines = symbols(&cell.defines, &mut roots)?;
            let uses = symbols(&cell.uses, &mut roots)?;
            let records = cell
                .records
                .iter()
                .map(|record| {
                    encode_record(
                        record,
                        &mut roots,
                        &mut contexts,
                        &mut context_bytes,
                        &binding.build,
                        limits,
                        ctx,
                    )
                })
                .collect::<Result<_, _>>()?;
            let output = cell
                .output
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| CheckpointError::Invalid)?;
            cells.push(CellData {
                input: InputData {
                    id: cell.id.clone(),
                    kind: cell.kind,
                    source: cell.source.clone(),
                    dialect: cell.dialect,
                },
                output,
                status: cell.status,
                defines,
                uses,
                exec_count: cell.exec_count,
                records,
            });
        }
        let mut owner_symbols = self.owners.iter().collect::<Vec<_>>();
        owner_symbols.sort_by_key(|(symbol, _)| symbol.name());
        let owners = owner_symbols
            .into_iter()
            .map(|(symbol, id)| {
                Ok((
                    push(&mut roots, &Expr::sym(*symbol), limits, ctx)?,
                    id.clone(),
                ))
            })
            .collect::<Result<_, CheckpointError>>()?;
        let metadata = Metadata {
            version: 1,
            binding: binding.clone(),
            general: serde_json::to_value(&self.config.general)
                .map_err(|_| CheckpointError::Invalid)?,
            system_language: self.system_language,
            platform: self.host_platform,
            output_serial: self.output_serial,
            title: self.notebook.title.clone(),
            cells,
            owners,
            contexts,
        };
        let metadata = serde_json::to_vec(&metadata).map_err(|_| CheckpointError::Invalid)?;
        if metadata.len() > limits.max_metadata_bytes {
            return Err(CheckpointError::Limit);
        }
        let graph = encode_expressions(&roots, limits.expressions, ctx)?;
        let parts = [
            metadata.as_slice(),
            evaluator.as_slice(),
            graph.as_slice(),
            context_bytes.as_slice(),
        ];
        let total = parts
            .iter()
            .try_fold(21usize, |n, p| n.checked_add(p.len()))
            .filter(|&n| n <= limits.max_bytes)
            .ok_or(CheckpointError::Limit)?;
        let mut bytes = Vec::with_capacity(total);
        bytes.extend(b"OMKS\x01");
        for part in &parts {
            bytes.extend(
                u32::try_from(part.len())
                    .map_err(|_| CheckpointError::Limit)?
                    .to_le_bytes(),
            );
        }
        for part in parts {
            bytes.extend(part);
        }
        Ok(bytes)
    }
    /// Restore a data-only owned candidate after exact expected source/config/build/provenance checks.
    /// It remains unaccepted; host durable checkpoint acceptance must move it into the active owner.
    pub fn decode_checkpoint(
        bytes: &[u8],
        restore: CheckpointRestore<'_>,
        limits: CheckpointLimits,
        ctx: &Interrupt,
    ) -> Result<WorkingSession, CheckpointError> {
        let CheckpointRestore {
            binding: expected,
            source: expected_file,
            general: expected_general,
            clock,
            cancel,
        } = restore;
        ctx.tick()?;
        expected.validate()?;
        source_limits(expected_file, limits)?;
        if bytes.len() > limits.max_bytes {
            return Err(CheckpointError::Limit);
        }
        if bytes.len() < 21 || &bytes[..5] != b"OMKS\x01" {
            return Err(CheckpointError::Invalid);
        }
        let mut sizes = Vec::new();
        for n in 0..4 {
            sizes.push(u32::from_le_bytes(
                bytes[5 + n * 4..9 + n * 4]
                    .try_into()
                    .map_err(|_| CheckpointError::Invalid)?,
            ) as usize);
        }
        if sizes[0] > limits.max_metadata_bytes
            || sizes[1] > limits.evaluator.max_bytes
            || sizes[2] > limits.expressions.max_bytes
            || sizes[3] > limits.max_bytes
        {
            return Err(CheckpointError::Limit);
        }
        let total = sizes
            .iter()
            .try_fold(21usize, |n, size| n.checked_add(*size))
            .ok_or(CheckpointError::Invalid)?;
        if total != bytes.len() {
            return Err(CheckpointError::Invalid);
        }
        let (mut start, mut parts) = (21, Vec::new());
        for size in sizes {
            parts.push(&bytes[start..start + size]);
            start += size;
        }
        let metadata: Metadata =
            serde_json::from_slice(parts[0]).map_err(|_| CheckpointError::Invalid)?;
        if metadata.version != 1 || metadata.binding != *expected {
            return Err(CheckpointError::Invalid);
        }
        let general = crate::checkpoint::wire::general(&metadata.general)?;
        if serde_json::to_value(&general).map_err(|_| CheckpointError::Invalid)?
            != serde_json::to_value(expected_general).map_err(|_| CheckpointError::Invalid)?
        {
            return Err(CheckpointError::Invalid);
        }
        let file = NotebookFile {
            version: 1,
            title: metadata.title.clone(),
            cells: metadata
                .cells
                .iter()
                .map(|c| c.input.source_input())
                .collect(),
        };
        source_limits(&file, limits)?;
        if serde_json::to_value(&file).map_err(|_| CheckpointError::Invalid)?
            != serde_json::to_value(expected_file).map_err(|_| CheckpointError::Invalid)?
        {
            return Err(CheckpointError::Invalid);
        }
        if metadata.output_serial > ((1u64 << 53) - 1)
            || metadata.contexts.len() > limits.max_records
            || metadata.owners.len() > limits.evaluator.max_bindings
        {
            return Err(CheckpointError::Limit);
        }
        let mut record_count = 0usize;
        for c in &metadata.cells {
            if c.status == CellStatus::Running {
                return Err(CheckpointError::NotIdle);
            }
            record_count = record_count
                .checked_add(c.records.len())
                .filter(|&n| n <= limits.max_records)
                .ok_or(CheckpointError::Limit)?;
        }
        let evaluator = om_eval::Evaluator::decode_persistent(
            parts[1],
            &expected.build,
            limits.evaluator,
            ctx,
        )?;
        let roots = decode_expressions(parts[2], limits.expressions, ctx)?;
        let mut contexts = Vec::new();
        let mut context_offset = 0usize;
        for data in metadata.contexts {
            ctx.tick()?;
            let offset = data.offset as usize;
            let length = data.length as usize;
            if offset != context_offset
                || length > limits.max_context_bytes
                || offset
                    .checked_add(length)
                    .is_none_or(|end| end > parts[3].len())
                || data.controls.is_empty()
                || data.controls.len() > 16
            {
                return Err(CheckpointError::Invalid);
            }
            context_offset += length;
            let mut evaluator_limits = limits.evaluator;
            evaluator_limits.max_bytes = evaluator_limits.max_bytes.min(limits.max_context_bytes);
            let ev = om_eval::Evaluator::decode_readonly_persistent(
                &parts[3][offset..context_offset],
                &expected.build,
                evaluator_limits,
                ctx,
            )?;
            let mut seen = BTreeSet::new();
            let mut controls = Vec::new();
            for c in data.controls {
                let (lower, upper, initial) = (
                    f64::from_bits(c.lower),
                    f64::from_bits(c.upper),
                    f64::from_bits(c.initial),
                );
                if !seen.insert(c.name.clone())
                    || c.name.is_empty()
                    || c.name.len() > 256
                    || !lower.is_finite()
                    || !upper.is_finite()
                    || !initial.is_finite()
                    || lower >= upper
                    || initial < lower
                    || initial > upper
                {
                    return Err(CheckpointError::Invalid);
                }
                controls.push(om_eval::explore::ExploreControl {
                    name: c.name,
                    range: (lower, upper),
                    initial,
                });
            }
            contexts.push(Some(crate::explore::Snapshot {
                eval: ev,
                plan: om_eval::explore::ExplorePlan {
                    expression: expr(&roots, data.expression, ctx)?,
                    controls,
                },
            }));
        }
        if context_offset != parts[3].len() {
            return Err(CheckpointError::Invalid);
        }
        let read_symbols = |ids: &[u32]| -> Result<BTreeSet<Symbol>, CheckpointError> {
            if ids.len() > limits.evaluator.max_bindings {
                return Err(CheckpointError::Limit);
            }
            let mut set = BTreeSet::new();
            let mut previous = None;
            for &id in ids {
                let symbol = expr(&roots, id, ctx)?
                    .as_symbol()
                    .ok_or(CheckpointError::Invalid)?;
                let name = symbol.name();
                if previous.is_some_and(|old| old >= name) || !set.insert(symbol) {
                    return Err(CheckpointError::Invalid);
                }
                previous = Some(name);
            }
            Ok(set)
        };
        let mut cells = Vec::new();
        let mut out_ids = BTreeSet::new();
        let mut view_ids = BTreeSet::new();
        for data in metadata.cells {
            ctx.tick()?;
            let defines = read_symbols(&data.defines)?;
            let uses = read_symbols(&data.uses)?;
            let output = data
                .output
                .map(crate::checkpoint::wire::output)
                .transpose()?;
            let mut records = Vec::new();
            for record in data.records {
                if record.out_index as usize > evaluator.history.len()
                    || !out_ids.insert(record.out_index)
                    || !view_ids.insert(record.view_id.clone())
                {
                    return Err(CheckpointError::Invalid);
                }
                let decoded = decode_record(record, &roots, &mut contexts, limits, ctx)?;
                let stored = evaluator
                    .history
                    .get(decoded.out_index as usize - 1)
                    .ok_or(CheckpointError::Invalid)?;
                if !om_core::checkpoint::same_representation(
                    &decoded.input,
                    &stored.0,
                    limits.expressions,
                    ctx,
                )? || !om_core::checkpoint::same_representation(
                    &decoded.value,
                    &stored.1,
                    limits.expressions,
                    ctx,
                )? {
                    return Err(CheckpointError::Invalid);
                }
                records.push(decoded);
            }
            if data
                .exec_count
                .is_some_and(|n| n as usize > evaluator.history.len())
            {
                return Err(CheckpointError::Invalid);
            }
            cells.push(Cell {
                id: data.input.id,
                kind: data.input.kind,
                source: data.input.source,
                dialect: data.input.dialect,
                output,
                status: data.status,
                defines,
                uses,
                exec_count: data.exec_count,
                records,
            });
        }
        if contexts.iter().any(Option::is_some) {
            return Err(CheckpointError::Invalid);
        }
        let live = evaluator.defs.defined_symbols();
        let mut owners = BTreeMap::new();
        let mut previous = None;
        for (root, id) in metadata.owners {
            let symbol = expr(&roots, root, ctx)?
                .as_symbol()
                .ok_or(CheckpointError::Invalid)?;
            let name = symbol.name();
            if previous.is_some_and(|old| old >= name)
                || !live.contains(&symbol)
                || !cells.iter().any(|c| c.id == id && c.kind == CellKind::Math)
                || owners.insert(symbol, id).is_some()
            {
                return Err(CheckpointError::Invalid);
            }
            previous = Some(name);
        }
        let mut config = KernelConfig {
            general,
            ..Default::default()
        };
        config.llm.enabled = false;
        config.llm.profiles.clear();
        config.llm.translate.clear();
        config.llm.explain.clear();
        config.llm.chat.clear();
        config.llm.complete.clear();
        config.llm.fix.clear();
        let mut candidate = Session::with_cancel_token(config, clock, cancel);
        candidate.eval = evaluator;
        candidate.notebook = Notebook {
            title: metadata.title,
            cells,
        };
        candidate.owners = owners;
        candidate.system_language = metadata.system_language;
        candidate.host_platform = metadata.platform;
        candidate.output_serial = metadata.output_serial;
        Ok(WorkingSession { candidate })
    }
}
fn source_limits(file: &NotebookFile, limits: CheckpointLimits) -> Result<(), CheckpointError> {
    if file.version != 1 {
        return Err(CheckpointError::Invalid);
    }
    if file.cells.len() > limits.max_cells || file.title.len() > 16384 {
        return Err(CheckpointError::Limit);
    }
    let mut bytes = file.title.len();
    let mut seen = BTreeSet::new();
    for cell in &file.cells {
        if cell.id.is_empty() || cell.id.len() > 256 || !seen.insert(cell.id.as_str()) {
            return Err(CheckpointError::Invalid);
        }
        bytes = bytes
            .checked_add(cell.id.len())
            .and_then(|n| n.checked_add(cell.source.len()))
            .filter(|&n| n <= limits.max_source_bytes)
            .ok_or(CheckpointError::Limit)?;
    }
    Ok(())
}
