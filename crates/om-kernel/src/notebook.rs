//! Notebook source storage and actual retained statement results.
use crate::protocol::{CellId, CellInput, CellKind, CellOutput, CellStatus, Dialect, NotebookFile};
use om_core::{Expr, Symbol};
use om_solve::Steps;
use std::collections::BTreeSet;

/// Source cells and title in document order.
#[derive(Default)]
pub struct Notebook {
    /// Ordered editable cells.
    pub cells: Vec<Cell>,
    /// Notebook title.
    pub title: String,
}

/// A source cell and its current execution state.
pub struct Cell {
    /// Frontend-generated identifier.
    pub id: CellId,
    /// Math, prose or natural-language request.
    pub kind: CellKind,
    /// Original editable source, including invalid input.
    pub source: String,
    /// Requested source dialect; Auto remains a persisted choice.
    pub dialect: Dialect,
    /// Most recent output, absent on loading a source-only file.
    pub output: Option<CellOutput>,
    /// Current execution state.
    pub status: CellStatus,
    /// Potential definitions in the current raw source.
    pub defines: BTreeSet<Symbol>,
    /// Free dependencies in the current raw source, including user function heads.
    pub uses: BTreeSet<Symbol>,
    /// Last successful statement history index in the current execution.
    pub exec_count: Option<u32>,
    pub(crate) records: Vec<StatementRecord>,
}

pub(crate) struct StatementRecord {
    pub input: Expr,
    pub value: Expr,
    pub steps: Option<Steps>,
    pub suppress_output: bool,
    pub out_index: u32,
}

impl Cell {
    /// Original uncanonicalized statement, including suppressed outputs.
    pub fn input(&self, out_index: u32) -> Option<&Expr> {
        self.records
            .iter()
            .find(|record| record.out_index == out_index)
            .map(|record| &record.input)
    }

    /// Actual solver derivation for a recorded statement, when recording was enabled.
    pub fn steps(&self, out_index: u32) -> Option<&Steps> {
        self.records
            .iter()
            .find(|record| record.out_index == out_index)
            .and_then(|record| record.steps.as_ref())
    }

    pub(crate) fn from_input(input: CellInput) -> Self {
        Self {
            id: input.id,
            kind: input.kind,
            source: input.source,
            dialect: input.dialect,
            output: None,
            status: if input.kind == CellKind::Text {
                CellStatus::Done
            } else {
                CellStatus::Stale
            },
            defines: BTreeSet::new(),
            uses: BTreeSet::new(),
            exec_count: None,
            records: vec![],
        }
    }

    fn source_input(&self) -> CellInput {
        CellInput {
            id: self.id.clone(),
            kind: self.kind,
            source: self.source.clone(),
            dialect: self.dialect,
        }
    }
}

pub(crate) enum FileError {
    Version(u32),
    EmptyId,
    DuplicateId(String),
}

impl Notebook {
    pub(crate) fn from_file(file: NotebookFile) -> Result<Self, FileError> {
        if file.version != 1 {
            return Err(FileError::Version(file.version));
        }
        let mut seen = BTreeSet::new();
        for cell in &file.cells {
            if cell.id.is_empty() {
                return Err(FileError::EmptyId);
            }
            if !seen.insert(&cell.id) {
                return Err(FileError::DuplicateId(cell.id.clone()));
            }
        }
        Ok(Self {
            title: file.title,
            cells: file.cells.into_iter().map(Cell::from_input).collect(),
        })
    }

    pub(crate) fn to_file(&self) -> NotebookFile {
        NotebookFile {
            version: 1,
            title: self.title.clone(),
            cells: self.cells.iter().map(Cell::source_input).collect(),
        }
    }
}
