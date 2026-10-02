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
    /// Rename the current source notebook without resetting live definitions/history.
    RenameNotebook {
        /// New source notebook title.
        title: String,
    },
    /// Inject the host UI locale for Auto; this never changes persisted preference.
    SetSystemLanguage {
        /// Actual host language.
        language: crate::config::Language,
    },
    /// Source and actual static dependency metadata for client recovery.
    GetNotebookState,
    /// Inspect actual live stored definitions without evaluating them.
    GetVariables,
    /// Rebuild only definition cells after an interrupted Worker restart.
    RestoreDefinitions,
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
        /// Actual response status when supplied by the transport (legacy clients omit it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<u16>,
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
    /// Source-only notebook state and dependency restoration order.
    NotebookState {
        /// Actual current state.
        state: NotebookState,
    },
    /// Actual live definition names and their readonly stored-value descriptions.
    Variables {
        /// Names sorted by spelling, paired with genuine stored-value metadata.
        items: Vec<(String, HoverInfo)>,
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
    /// A terminated worker was replaced and its definition cells restored.
    KernelRestarted {
        /// Localized recovery notice.
        message: String,
    },
    /// Actual provider probe response and injected-clock measurements.
    LlmProfileTest {
        /// Job identifier.
        request_id: RequestId,
        /// Configured profile name.
        profile: String,
        /// Total time to the completed response; absent without a clock.
        latency_ms: Option<f64>,
        /// Time to the first nonempty response bytes; absent without a clock or bytes.
        first_byte_ms: Option<f64>,
        /// Actual model reply, never a fabricated pong.
        response: String,
    },
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
