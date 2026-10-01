//! Shared JSON request, response and event contract for all clients.
pub use crate::config::KernelConfig;
pub use crate::views::*;
pub use crate::wire::*;
pub use om_llm::{ChatMessage, HttpRequest, Role, Suggestion, ToolCall};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Frontend-generated cell identifier.
pub type CellId = String;
/// LLM job identifier, independent of the envelope correlation number.
pub type RequestId = String;

/// A client operation; runtime dispatch is implemented by Session.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Execute a source cell.
    Evaluate {
        /// Target cell.
        cell_id: CellId,
        /// Source to parse and evaluate.
        source: String,
        /// Requested input syntax.
        dialect: Dialect,
    },
    /// Synchronize edited source without executing.
    UpsertCell {
        /// Editable cell.
        cell: CellInput,
    },
    /// Remove a cell.
    DeleteCell {
        /// Target cell.
        cell_id: CellId,
    },
    /// Move a cell within document order.
    MoveCell {
        /// Target cell.
        cell_id: CellId,
        /// New zero-based position.
        to_index: u32,
    },
    /// Execute all cells.
    RunAll,
    /// Parse and render without evaluation.
    Preview {
        /// Source to preview.
        source: String,
        /// Requested input syntax.
        dialect: Dialect,
        /// Optional UTF-8 cursor offset.
        cursor: Option<u32>,
    },
    /// Request local editor completions.
    Complete {
        /// Current source.
        source: String,
        /// UTF-8 cursor offset.
        cursor: u32,
        /// Requested input syntax.
        dialect: Dialect,
    },
    /// Inspect a built-in or symbol.
    Hover {
        /// Current source.
        source: String,
        /// UTF-8 cursor offset.
        cursor: u32,
        /// Requested input syntax.
        dialect: Dialect,
    },
    /// Sample a portable plot request.
    SamplePlot {
        /// Sampling options.
        request: PlotRequest,
    },
    /// Set the cancellation flag.
    Interrupt,
    /// Restore source cells from a file DTO.
    LoadNotebook {
        /// Versioned source notebook.
        file: NotebookFile,
    },
    /// Return the source-only notebook file DTO.
    SaveNotebook,
    /// Return settings with masked keys.
    GetConfig,
    /// Install submitted settings after resolving masked keys.
    SetConfig {
        /// Submitted settings.
        config: KernelConfig,
    },
    /// Translate natural language into a CAS suggestion.
    LlmTranslate {
        /// Job identifier.
        request_id: RequestId,
        /// Natural language request.
        text: String,
        /// Optional target cell.
        cell_id: Option<CellId>,
    },
    /// Explain actual solver steps.
    LlmExplain {
        /// Job identifier.
        request_id: RequestId,
        /// Source cell.
        cell_id: CellId,
        /// Optional single step to explain.
        step_id: Option<String>,
    },
    /// Request insertion text at the cursor.
    LlmComplete {
        /// Job identifier.
        request_id: RequestId,
        /// Source before the cursor.
        prefix: String,
        /// Source after the cursor.
        suffix: String,
        /// Requested source syntax.
        dialect: Dialect,
    },
    /// Start a tool-enabled conversation.
    LlmChat {
        /// Job identifier.
        request_id: RequestId,
        /// Conversation turns.
        messages: Vec<ChatMessage>,
    },
    /// Propose a repair for an errored cell.
    LlmFixError {
        /// Job identifier.
        request_id: RequestId,
        /// Source cell.
        cell_id: CellId,
    },
    /// Test a named provider profile.
    LlmTestProfile {
        /// Job identifier.
        request_id: RequestId,
        /// Configured profile name.
        profile: String,
    },
    /// Cancel an active job.
    LlmCancel {
        /// Job identifier.
        request_id: RequestId,
    },
    /// Feed browser HTTP response text into a job.
    LlmHttpChunk {
        /// Job identifier.
        request_id: RequestId,
        /// Decoded response text fragment.
        chunk: String,
    },
    /// Report browser HTTP completion or failure.
    LlmHttpEnd {
        /// Job identifier.
        request_id: RequestId,
        /// HTTP status, or zero for transport failure.
        status: u16,
        /// Optional transport failure explanation.
        error: Option<String>,
    },
}

/// Synchronous reply to a client request.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// Successful operation without a payload.
    Ok,
    /// Operation failure.
    Error {
        /// Failure explanation.
        message: String,
    },
    /// Evaluation result and dependent cells rerun.
    Evaluated {
        /// Evaluated cell.
        cell_id: CellId,
        /// Cell outputs and messages.
        output: CellOutput,
        /// Dependent cells executed in order.
        reran: Vec<CellId>,
    },
    /// Flattened preview payload.
    Preview(PreviewResult),
    /// Editor candidates and replacement span.
    Completions {
        /// Ranked candidates.
        items: Vec<CompletionItem>,
        /// Inclusive UTF-8 replacement offset.
        from: u32,
        /// Exclusive UTF-8 replacement offset.
        to: u32,
    },
    /// Documentation or current symbol value.
    Hover {
        /// None when no target is recognized.
        info: Option<HoverInfo>,
    },
    /// Sampled plot.
    Plot {
        /// Sampled curves and viewport.
        data: PlotData,
    },
    /// Source-only notebook file.
    Notebook {
        /// Portable file DTO.
        file: NotebookFile,
    },
    /// Current configuration with masked keys.
    Config {
        /// Settings.
        config: KernelConfig,
    },
    /// An LLM job with an optional browser transport request.
    LlmStarted {
        /// Job identifier.
        request_id: RequestId,
        /// Some when the browser must drive HTTP.
        http: Option<HttpRequest>,
    },
}

/// Asynchronous notification; envelope correlation id is zero.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Cell execution state changed.
    CellStatus {
        /// Affected cell.
        cell_id: CellId,
        /// New execution state.
        status: CellStatus,
    },
    /// Dependent cell produced outputs.
    CellOutput {
        /// Affected cell.
        cell_id: CellId,
        /// Cell outputs and messages.
        output: CellOutput,
    },
    /// Streamed assistant text.
    LlmDelta {
        /// Job identifier.
        request_id: RequestId,
        /// Text delta.
        text: String,
    },
    /// Completed CAS tool invocation.
    LlmToolCall {
        /// Job identifier.
        request_id: RequestId,
        /// Tool name.
        name: String,
        /// JSON argument text.
        arguments: String,
        /// CAS result summary.
        result_summary: String,
    },
    /// CAS-checked source suggestion.
    LlmSuggestion {
        /// Job identifier.
        request_id: RequestId,
        /// Suggestion awaiting user action.
        suggestion: Suggestion,
    },
    /// Next browser HTTP request after tools.
    LlmHttp {
        /// Job identifier.
        request_id: RequestId,
        /// Request for browser transport.
        http: HttpRequest,
    },
    /// Job completed.
    LlmDone {
        /// Job identifier.
        request_id: RequestId,
    },
    /// Job failed.
    LlmError {
        /// Job identifier.
        request_id: RequestId,
        /// Failure explanation.
        message: String,
    },
}

/// Transport correlation wrapper; clients use safe JS integer IDs and events use zero.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/kernel/generated/"))]
pub struct Envelope<T> {
    /// Request/reply correlation number. JSON encodes u64 as a number.
    #[ts(type = "number")]
    pub id: u64,
    /// Request, response or event payload.
    pub body: T,
}
