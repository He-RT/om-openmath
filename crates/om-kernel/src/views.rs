//! Renderable protocol data, separate from expressions and computational step kinds.
use crate::protocol::CellId;
use crate::wire::{Diagnostic, Dialect, Level, Message, Span, TokenClass, Verification};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
/// Current source file and actual metadata; no secrets or expression internals.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct NotebookState {
    /// Source-only persistence DTO.
    pub file: crate::protocol::NotebookFile,
    /// Cells in document order.
    pub cells: Vec<CellState>,
    /// Definition cells in dependency order.
    pub definition_order: Vec<crate::protocol::CellId>,
    /// Definition cells in dependency cycles or blocked by cycles.
    pub cycles: Vec<crate::protocol::CellId>,
}
/// Actual static source dependencies and current execution status.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct CellState {
    /// Source cell ID.
    pub id: crate::protocol::CellId,
    /// Current execution state.
    pub status: crate::protocol::CellStatus,
    /// Defined names, sorted by spelling.
    pub defines: Vec<String>,
    /// Used names, sorted by spelling.
    pub uses: Vec<String>,
    /// Actual last successful statement index, including suppressed statements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub exec_count: Option<u32>,
}
/// CellKind values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum CellKind {
    /// Math.
    Math,
    /// Text.
    Text,
    /// Ask.
    Ask,
}

/// CellStatus values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum CellStatus {
    /// Queued.
    Queued,
    /// Running.
    Running,
    /// Done.
    Done,
    /// Error.
    Error,
    /// Stale.
    Stale,
}

/// CompletionKind values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum CompletionKind {
    /// Function.
    Function,
    /// Symbol.
    Symbol,
    /// Keyword.
    Keyword,
    /// Snippet.
    Snippet,
}

/// PlotKind values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum PlotKind {
    /// Function.
    Function,
    /// Implicit.
    Implicit,
    /// Two-coordinate parameterized curve.
    Parametric,
    /// Sampled Boolean region.
    Region,
    /// Real two-coordinate vector field.
    Field,
    /// Actual finite data points/grid.
    Data,
    /// Frequency counts of finite samples.
    Histogram,
    /// Scalar two-axis value cells.
    Density,
}

/// SolutionKind values accepted by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum SolutionKind {
    /// Finite.
    Finite,
    /// All.
    All,
    /// None.
    None,
    /// Region.
    Region,
}

/// Versioned source-only notebook; configuration and secrets cannot enter this schema.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct NotebookFile {
    /// Notebook format version (currently 1).
    pub version: u32,
    /// Notebook title.
    pub title: String,
    /// Source cells in document order.
    pub cells: Vec<CellInput>,
}

/// Editable source cell without runtime state.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct CellInput {
    /// Frontend-generated cell identifier.
    pub id: CellId,
    /// Cell presentation kind.
    pub kind: CellKind,
    /// Original editable source.
    pub source: String,
    /// Requested source dialect.
    pub dialect: Dialect,
}

/// One local editor completion.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct CompletionItem {
    /// Displayed candidate name.
    pub label: String,
    /// Insertion text or snippet.
    pub insert_text: String,
    /// Short signature or explanation.
    pub detail: Option<String>,
    /// Candidate category.
    pub kind: CompletionKind,
}

/// Documentation or current symbol value under the cursor.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct HoverInfo {
    /// Built-in signature when available.
    pub signature: Option<String>,
    /// Localized description.
    pub summary: String,
    /// Executable examples.
    pub examples: Vec<String>,
    /// Current InputForm value, truncated by the handler.
    pub value: Option<String>,
    /// Cell defining a user symbol.
    pub cell_id: Option<CellId>,
}

/// Fast non-evaluating source preview.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct PreviewResult {
    /// Rendered expression when available.
    pub latex: Option<String>,
    /// Source diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Highlighting spans and categories.
    pub tokens: Vec<(Span, TokenClass)>,
    /// Effective dialect.
    pub dialect: Dialect,
    /// Suggested explicit actions.
    pub actions: Vec<CellAction>,
}

/// A localized action carrying editable CAS source.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct CellAction {
    /// Internationalization key.
    pub label_key: String,
    /// Source to insert or run on user action.
    pub source: String,
}

/// Items and messages emitted by a completed cell evaluation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct CellOutput {
    /// Visible outputs in statement order.
    pub items: Vec<OutputItem>,
    /// Evaluation messages, including suppressed statements.
    pub messages: Vec<Message>,
    /// Elapsed evaluation milliseconds.
    pub timing_ms: f64,
}

/// Renderable solution set and optional real intervals.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct SolutionSetView {
    /// Set presentation kind.
    pub kind: SolutionKind,
    /// Requested variable names.
    pub vars: Vec<String>,
    /// Individual solutions.
    pub solutions: Vec<SolutionView>,
    /// Region condition in LaTeX.
    pub region_latex: Option<String>,
    /// Real number-line intervals.
    pub intervals: Vec<IntervalView>,
}

/// One solution with conditions and verification evidence.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct SolutionView {
    /// Assigned variable values.
    pub bindings: Vec<BindingView>,
    /// Condition attached to this solution.
    pub condition_latex: Option<String>,
    /// Optional set-membership presentation, preserving the legacy condition field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub condition_display_latex: Option<String>,
    /// Actual verification evidence.
    pub verified: Verification,
}

/// A variable value rendered for display, copy and numeric inspection.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct BindingView {
    /// Variable name.
    pub var: String,
    /// LaTeX value.
    pub latex: String,
    /// Wolfram source value.
    pub input_form: String,
    /// Modern source value.
    pub modern_form: String,
    /// Twenty-digit numeric form for exact values.
    pub numeric: Option<String>,
    /// Variable display spelling from the actual formatter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub var_latex: Option<String>,
    /// Actual top-level Root index when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub root_index: Option<u32>,
    /// Genuine radical conversion when the evaluator returns one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub radicals: Option<ExpressionView>,
}

/// An expression rendered by the CAS for display and source reuse.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct ExpressionView {
    /// Wolfram source form.
    pub input_form: String,
    /// Modern source form.
    pub modern_form: String,
    /// LaTeX display form.
    pub latex: String,
}

/// A real interval with optional finite numeric endpoints.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct IntervalView {
    /// LaTeX lower bound; None means negative infinity.
    pub lo: Option<String>,
    /// LaTeX upper bound; None means positive infinity.
    pub hi: Option<String>,
    /// Whether the lower bound is included.
    pub lo_closed: bool,
    /// Whether the upper bound is included.
    pub hi_closed: bool,
    /// Finite lower coordinate.
    pub lo_value: Option<f64>,
    /// Finite upper coordinate.
    pub hi_value: Option<f64>,
}

/// A renderable hierarchy of actual solver steps.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct StepsView {
    /// Top-level steps in execution order.
    pub root: Vec<StepView>,
}

/// Only renderable strings and child steps; contains no expression internals.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct StepView {
    /// Stable hierarchical step identifier.
    pub id: String,
    /// Stable computational rule identifier.
    pub rule_id: String,
    /// Default expansion level.
    pub level: Level,
    /// Internationalization key: step. plus rule_id.
    pub title_key: String,
    /// LaTeX strings for template substitutions.
    pub params: BTreeMap<String, String>,
    /// Before expressions in LaTeX.
    pub before_latex: Vec<String>,
    /// After expressions in LaTeX.
    pub after_latex: Vec<String>,
    /// Nested computational steps.
    pub children: Vec<StepView>,
}

/// Portable plot source, viewport, highlights and parameter sliders.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct PlotRequest {
    /// Additive explicit mathematical domains/data/options for new two-dimensional kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub options: Option<crate::plot_views::PlotOptions2D>,
    /// Function or implicit sampling.
    pub kind: PlotKind,
    /// InputForm source expressions.
    pub exprs: Vec<String>,
    /// Horizontal variable.
    pub var_x: String,
    /// Vertical variable for implicit sampling.
    pub var_y: Option<String>,
    /// Horizontal viewport.
    pub x_range: (f64, f64),
    /// Optional vertical viewport.
    pub y_range: Option<(f64, f64)>,
    /// Current parameter values.
    pub params: BTreeMap<String, f64>,
    /// Highlighted solution coordinates.
    pub points: Vec<(f64, f64)>,
    /// Shaded x intervals; use +/-1e308 for infinity.
    pub shade: Vec<(f64, f64)>,
    /// Slider ranges.
    pub param_ranges: BTreeMap<String, (f64, f64)>,
    /// Resolved original solving source for genuine highlight updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub solve: Option<PlotSolveSource>,
}

/// Selected solver domain, independent of display coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub enum PlotDomain {
    /// All finite complex values.
    Complexes,
    /// Real values.
    Reals,
    /// Integer values.
    Integers,
    /// Exact rational values.
    Rationals,
}
/// Transported raw source and actual domain for parameter re-solving.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct PlotSolveSource {
    /// Resolved raw equation/inequality source in InputForm.
    pub source: String,
    /// Actual domain selected by the original solver.
    pub domain: PlotDomain,
}
/// Actual current finite solution coordinates and region shading.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct PlotHighlights {
    /// Recomputed real complete assignments.
    pub points: Vec<(f64, f64)>,
    /// Current actual real intervals, using infinity sentinels.
    pub shade: Vec<(f64, f64)>,
}

/// Sampled continuous segments and actual viewport.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct PlotData {
    /// Actual additional geometry, absent on legacy curve data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geometry: Option<crate::plot_views::PlotGeometry2D>,
    /// Display transform, absent on legacy linear plots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub scale: Option<crate::plot_views::PlotScale>,
    /// Sampled curves.
    pub curves: Vec<Curve>,
    /// Horizontal viewport.
    pub x_range: (f64, f64),
    /// Vertical viewport.
    pub y_range: (f64, f64),
    /// Current highlights; absent for legacy curves without highlights/source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub highlights: Option<PlotHighlights>,
}

/// One labeled curve split at discontinuities.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Curve {
    /// Curve label.
    pub label: String,
    /// Continuous polylines.
    pub segments: Vec<Vec<(f64, f64)>>,
}

/// Visible statement output, tagged independently of responses.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputItem {
    /// A held expression with genuine isolated initial computation and local controls.
    Explore {
        /// Actual history index.
        out_index: u32,
        /// Opaque retained output identity.
        view_id: String,
        /// Held source in Wolfram syntax.
        input_form: String,
        /// Held source in modern syntax.
        modern_form: String,
        /// Actual finite controls.
        controls: Vec<crate::explore_views::ExploreControl>,
        /// Actual initial readonly result.
        result: Box<crate::explore_views::ExploreResult>,
    },
    /// A symbolic or numeric expression.
    Expr {
        /// Output history index.
        out_index: u32,
        /// Wolfram source form.
        input_form: String,
        /// Modern source form.
        modern_form: String,
        /// LaTeX display form.
        latex: String,
        /// Optional structured first page; omitted for original scalar wire outputs.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        presentation: Option<crate::value_views::ValuePage>,
    },
    /// Solver output with actual derivation and an optional visualization.
    Solutions {
        /// Output history index.
        out_index: u32,
        /// Wolfram source form.
        input_form: String,
        /// Modern source form.
        modern_form: String,
        /// Solution cards and region.
        view: SolutionSetView,
        /// Renderable computational steps when enabled.
        steps: Option<StepsView>,
        /// Automatic visualization request when supported.
        plot: Option<PlotRequest>,
    },
    /// An explicitly requested plot.
    Plot {
        /// Source and sampling options.
        request: PlotRequest,
        /// Sampled result.
        data: PlotData,
    },
    /// Statement failure.
    Error {
        /// Failure explanation.
        message: String,
        /// Responsible source span, if available.
        span: Option<Span>,
    },
}
