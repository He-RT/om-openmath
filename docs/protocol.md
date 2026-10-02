# Kernel JSON protocol

`om_kernel::protocol` defines the shared request, response and event schema.
`om_kernel::Session` implements Evaluate, GetConfig, SetConfig, LoadNotebook,
SaveNotebook and Interrupt in M11.2. Reactive notebook execution, specialized output views, plotting/editor helpers
and shared LLM jobs are implemented. Actual CLI, WASM worker and desktop host
clients remain the M13 integration tasks, using this same contract.

Requests and responses use `Envelope<T> { id, body }`. Their numeric IDs match;
events use ID zero. Browser clients must keep IDs and millisecond timeouts within
`Number.MAX_SAFE_INTEGER`. LLM jobs have separate string `request_id` values.

```json
{"id":42,"body":{"type":"evaluate","cell_id":"a","source":"solve(x^2=2,x)","dialect":"Modern"}}
```

Operation tags use snake_case. Parser dialects are `Modern`, `Wolfram`, `Auto`;
cell kinds/statuses, token classes and message levels retain their capitalized
Rust spellings. Configuration dialects use `modern`, `wolfram`, `auto` as in the
TOML configuration example. Solution kinds are `finite`, `all`, `none`, `region`.
Verification retains the solver representation: `"Exact"`, `"ByConstruction"`,
`{"Numeric":{"digits":200}}` or `"Unverified"`.

`Response::Preview` is flattened into the tagged object:

```json
{"type":"preview","latex":"x^2","diagnostics":[],"tokens":[],"dialect":"Modern","actions":[]}
```

Optional fields serialize as null. Tuples (token/span pairs, coordinates and
ranges) serialize as arrays; parameter maps have string keys. Most DTO fields
are required on input. Option fields may be omitted, and configuration sections
and profile options accept documented defaults. Configuration defaults follow
PLAN §10.8; an omitted capability is false, so a custom provider must explicitly
advertise tool and JSON support.

`StepsView.root` contains `StepView` values with stable IDs, rule IDs, levels,
title keys, LaTeX parameter maps, before/after LaTeX arrays and children. No
expression tree or computational `StepKind` crosses the wire. Owned diagnostic
adapters preserve the parser JSON without leaking incoming codes to static memory.

Notebook files contain only `version` (currently 1), `title` and ordered `cells`.
Each cell contains `id`, `kind`, `source`, `dialect`. Configuration, API keys,
runtime definitions and cached outputs are excluded from the file schema.

A present `ProfileConfig.api_key` serializes and formats for Debug as `"***"`.
Before installing a submitted configuration, call `merge_redacted_keys` with
the current configuration: masks retain the existing key by profile name, while
null/missing keys clear it. A mask for a new profile resolves to no key. Real
credentials in a browser `HttpRequest` belong to the explicit transport request;
configuration masking does not alter HTTP headers. Native persistence and key
resolution use the explicit store binding described below.

Session parses each Math cell with its configured constants and actual function
definitions, then passes raw statements to the existing Evaluator. Parse errors
prevent any statement in that cell from executing. An evaluation failure stops
later statements and retains earlier effects; suppressed statements still enter
history and retain messages. The cell's exec_count is its last successful
statement index. `Cell::input(index)` and `Cell::steps(index)` expose the actual
raw statement and solver derivation through Rust, including suppressed outputs.
Actual solver results are packed as Solutions with genuine verification,
conditions, bindings and optional renderable derivations. Other values retain
accurate Expr/Error items; explicit plotting is sampled by the shared compiler; supported solver results also carry actual automatic visualization requests.

LoadNotebook validates version 1 and unique nonempty IDs before replacing state.
It resets definitions/history and cached outputs, while preserving configuration.
Restored Math/Ask cells are Stale and Text cells are Done. Text/Ask Evaluate
requests are rejected; natural-language requests will use LLM handlers.
LLM handlers remain the subsequent M12 milestone and
return an explicit err.not_implemented error at this stage.

Hosts supply an optional `Arc<dyn Clock>` for deadlines and timing. Without it,
timing_ms is zero and the wall-clock deadline is disabled; step budgets and the
shared cancellation flag still work. Each cell shares one Interrupt across all
statements. Hosts can set interrupt_handle during synchronous execution. An initiating
Math evaluation, SamplePlot, RunAll or automatic Delete cascade resets the previous flag
once; automatic cells share it. Config/save/load/source-only edits do not.
SetConfig validates profile-name uniqueness and resolves masks before installing
settings. Kernel error strings retain stable err.* keys with Chinese/English text.

Generate bindings with `cargo test -p om-kernel export_bindings --locked`. All
reachable types are committed under `app/src/kernel/generated/`. The derive
export paths are fixed from each crate's manifest directory, because ts-rs 12's
default base is `bindings/`. Do not edit generated files. CI regenerates them and
checks both tracked differences and unexpected files; frontend typecheck and
tests validate imports, discriminant narrowing and representative JSON shapes.
The single export test also removes trailing line whitespace from ts-rs output;
per-type automatic exporters are disabled to keep regeneration consistent.


Reactive Math cells analyze raw source without evaluation. Current `defines`
contains potential assignment targets; `uses` includes free user function heads,
excluding builtins, cell definitions and lexical pattern/function/iterator names.
Live definition ownership is tracked separately from edited source and follows
actual global evaluator writes, including partial effects before failure.

With reactive enabled, a definition owned by another cell produces
`err.multiple_definitions` before execution or clearing. Valid execution clears
only this cell's live definitions. Changes propagate transitively through uses,
with each topological layer in document order. True cycle members receive
`err.cycle`; blocked downstream cells remain Stale. A stale or failed prerequisite
also blocks automatic execution, including prerequisites outside the triggering
closure. Cycle reporting retains actual previous history and statement records.

Automatic dependent attempts emit Queued, Running, final CellStatus, then
CellOutput. `Evaluated.reran` lists only attempted automatic executions in their
actual order. Disabling auto_run_dependents marks the dependency closure Stale
and retains old output; disabling reactive permits normal sequential redefinition
without automatic propagation or pre-clearing.

UpsertCell synchronizes source without CAS execution, preserves an existing
position, and appends new IDs. It analyzes Math source and marks the edited cell
and affected dependents Stale. Changing Math to Text/Ask releases its live
symbols and removes its CAS output; Text is Done and Ask is Stale. Repeating an
identical upsert leaves current execution state intact. DeleteCell removes only
that cell's source and actually owned definitions, then propagates stale status
or automatic recalculation. MoveCell accepts a final zero-based index smaller
than the number of cells and changes only document order. Empty IDs, missing
cells and invalid positions fail atomically. RunAll executes Math cells once in
document order through the same real evaluator path and emits status/output
events; Text/Ask sources are not executed and cyclic/blocked cells are reported.


Solver cards use actual evaluator callback metadata, including the resolved raw
source, requested/inferred variable order, full SolutionSet and returned value.
Provenance follows actual tail calls and forwarded Set/final compound results;
ordinary lists, cached values, unrelated wrappers, discarded earlier calls and
unsupported results retain Expr. Disabling steps removes derivation JSON while
keeping real solution evidence. SolveValues/NSolveValues and flat FindRoot rules
retain their actual source forms alongside the original solution bindings;
Reduce/Roots can display actual boolean regions and interval endpoints.

Finite multiplicities become repeated solution rows. Conditional/free-variable
solutions and generated parameter domains remain intact. Exact binding values
receive a real read-only N[value,10] numeric string when supported; approximate
values or failed supplemental numerics leave numeric null. The string keeps
existing InputForm precision notation. Formatting never records new history or
mutates live definitions, and infinite interval ends serialize as null.

StepsView preserves real IDs, rule IDs, levels and children. title_key is
step.<rule_id>, expression params and before/after snapshots use LaTeX, and
structured operation/reason/domain/count/message params remain deterministic
strings. No computational expression tree or StepKind is serialized. plot is
optional and follows the actual supported source/solution shape and auto_plot setting.


SamplePlot validates finite ordered ranges, distinct user axes, real parameter
values and the request's source expressions before producing finite PlotData.
Function plots start at 400 points and refine curvature/domain boundaries up to
six levels; implicit plots use 160×160 squares and deterministic contour stitching.
Pole or jump sign changes do not become implicit zeros. Default function y
viewport uses finite 2%–98% quantiles plus padding; explicit y_range is honored.
Non-real/undefined samples are omitted, infinite geometry is never serialized.

Plot and ContourPlot are protected HoldAll builtins. Examples:

```text
Plot[Sin[x],{x,0,2*Pi}]
Plot[{x,x^2},{x,-2,2},PlotRange->{-1,5}]
ContourPlot[x^2+y^2==1,{x,-2,2},{y,-2,2}]
plot(sin(x),[x,0,2pi])
implicitplot(x^2+y^2=1,[x,-2,2],[y,-2,2])
```

Their visible held results become OutputItem::Plot through the same sampler.
Bodies bind axes locally while bounds read outer values; readonly source
preparation resolves genuine functions/parameters without cancelling raw poles.
PlotRange accepts a y pair or an x/y pair of ranges for Plot; other explicit
options report err.plot. Plot axis/option dependencies follow those scopes.
Suppression retains held history without sampling. A visible rendering failure
retains the real completed statement record, reports Error and stops subsequent
statements/dependents. Standalone sampling records no history. Actual injected
flag/deadline/budget applies to preparation, every stack instruction, refinement
and stitching; the next initiating request can recover.


Automatic solver plots use the actual resolved source, selected domain and
existing complete solution set. One-axis equalities display lhs/rhs curves and
real (root,lhs(root)) points, with at least six units of x width. Two-axis systems
display both implicit residuals and complete real assignments. Rational region
results supply actual interval shading; unbounded shade uses +/-1e308. Unsupported
shapes, free axes/infinite families and nonreal initial one-axis solution sets
leave plot absent. auto_plot=false disables this optional output.

PlotRequest has an optional solve object with InputForm source and selected
domain (Complexes/Reals/Integers/Rationals). PlotData has optional highlights
with current points/shade. Both extensions are omitted when absent, preserving
legacy JSON shapes. These fields carry the equation provenance and updated
geometry through serialized native/WASM requests without a Session cache.
Clients must use data.highlights when present, including empty arrays that clear
previous points or shade, rather than retaining old request highlights.

Every source parameter is an explicit params binding, with default one (two
when one violates an original nonzero restriction) and slider range [-5,5].
Finite binary64 parameter points are substituted exactly as binary rationals.
SamplePlot verifies source/curve/axis correspondence and complete bindings,
then runs genuine NSolve for assignments or Reduce for regions under the original
domain and raw pole/condition restrictions. It returns actual finite real current
highlights. Nonreal updated roots clear real points; a parameter setting violating
all original source branches reports err.plot. Valid union branches are retained.
Requests without solve preserve explicitly supplied legacy highlights. All this
work shares the existing interruption scope, records no extra history and never
changes live definitions or original card verification/steps.


Complete, Hover and Preview are actual non-evaluating editor services. Cursor
positions and replacement ranges are UTF-8 bytes; invalid boundaries return
err.cursor atomically. Effective dialect/constants follow configuration, including
%wl/%modern source markers. They never reset interruption or write history,
definitions, source cells or stored solver provenance.

Complete replaces the whole identifier while matching its prefix before the
cursor. It ranks executable registered names, live symbols, language keywords,
snippets and supported innermost-call options: prefix, then camel/underscore
initials, then subsequence; Solving/Algebra docs lead ties, stable order caps at 50.
Modern labels use lowercase/aliases; logical And/Or/Not and algebraic Root require
capitalized callable insertions, distinct from keywords and root(expr,degree).
Comments, strings, numbers, markers, lists/Part/groupings and completed calls do
not leak keyword options. Extended implemented names share actual parser mapping.

Hover returns real localized builtin documentation or the stored unevaluated
ownvalue/downvalue rules (at most 200 Unicode chars) with the actual live owner
cell. Edited source may be Stale while the previously executed definition remains
inspectable; transfer/Clear/Unset/delete/load follow real evaluator ownership.
Bare defined symbols take precedence over lookalike builtin aliases; actual call
positions use their real builtin mapping. No delayed or computed value is run.

Preview returns real parser diagnostics/fixes/highlights/dialect. Any Error
suppresses LaTeX/actions; empty source displays none. Multiple valid statements
select the cursor's containing/preceding statement (first when before all math),
or the last when cursor is absent. Display canonicalizes raw syntax only. A bare
Equal offers action.solve for up to three lexical free axes, excluding function
heads/binders, wrapping original source in dialect-correct Solve syntax so original
poles/comments survive. Generated actions execute through the same Session.
The explicit native warm-release <=1000-char acceptance measures real calls
against <5ms; representative final maximum was about .662ms.


The native feature provides ConfigStore and Session::new_native for real
config.toml persistence. The default path comes from
ProjectDirs::from("org","openmath","OpenMath"). Missing files return documented
defaults; partial TOML preserves defaults, invalid files remain unchanged.
Session::new remains pure even when the native feature is enabled, so portable
clients/tests have no implicit user-directory or credential access. Real native
hosts explicitly bind the store at initialization.

Bound SetConfig validates and commits before installing new state. Wire masks
preserve key fields by profile name, null/missing clears managed fallback/vault
values, empty strings are explicit values, and removed profiles release their
managed entries. Newly entered native keys default to keyring
service="openmath", user=profile name; explicit Plaintext storage supports the
prescribed file fallback with private Unix permissions. Unchanged legacy keys
are not silently migrated. Trusted TOML encoding writes actual fallback data,
never the wire's *** mask; corrupt stored masks are rejected.

Runtime credential resolution is environment variable, then vault, then fallback
field. Resolved values are accessed only through SecretKey for a real transport
request. Config presentation exposes only the literal *** presence marker; it
never copies resolved env/vault values back into persistent inputs. Missing or
unavailable vaults permit existing fallback use; locked/encoding failures remain
explicit, and a requested new vault write does not silently become plaintext.
External environment variables are not deleted by clearing stored fields.

Actual adjacent temporary files use create_new, private Unix mode, flush/sync
and atomic replacement. All attempted vault updates are restored on reported
write/replace failure; an unsuccessful rollback is reported explicitly. Error
messages carry safe syntax offsets/IO kinds without credential/source excerpts.
Notebook loading preserves the bound config store; notebook files still contain
only source cells. Native tests use isolated paths and synthetic providers and
never query or change live user credentials.


om-llm now builds actual sans-IO OpenAI-compatible/Anthropic chat requests from
runtime Profile, actual message/tool replay and configured capabilities. Runtime
paths use try_build_chat_request: malformed profile/header/schema/conversation
returns a typed secret-safe error. The fixed-signature build_chat_request is a
validated-input convenience that panics on invalid construction. Tool-call/result
associations are checked per active round while actual provider IDs are retained.
Custom headers merge deterministically/case-insensitively; browser Anthropic
direct-access is target-specific. Configured models/endpoints remain editable.

HttpRequest transport JSON retains actual authorization for explicit transport;
Debug shows no endpoint/body/header values. Runtime Profile keys and configuration
extra-header values are likewise excluded from Debug. Construction performs no
network IO, does not retrieve a live key and never invokes CAS or a Job yet.

SseDecoder checked byte/text feeds preserve arbitrary split UTF-8, BOM, CR/LF/CRLF
and multiline data under a one-MiB frame/line budget. Finish discards incomplete
SSE events and rejects incomplete encoding; NDJSON validates and returns a final
complete JSON record at EOF. Failure is terminal and checked callers receive no
partial events from that failing call. Legacy feed is a validated-stream
convenience; runtime routes must use feed_bytes/try_feed and handle LlmError.

Provider deltas preserve actual text, tool indices/IDs/names/argument fragments,
finish reasons and errors. Only OpenAI choice0 is selected; reasoning and unknown
metadata are ignored. Anthropic initial empty tool input does not add {} ahead of
streamed JSON fragments. Specific finish reasons followed by terminal done
markers remain separate events; the later Job consumer must handle these
idempotently and wait for HTTP completion before running tools or reporting Done.

Protocol reference checks used the primary [OpenAI chat reference](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create),
[Claude streaming documentation](https://platform.claude.com/docs/en/build-with-claude/streaming)
and [WHATWG SSE framing standard](https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation).
The project's prescribed compatibility max_tokens/json_object shape is retained;
model-specific migration or live-provider validation is not claimed.


FIM construction and raw response parsing are implemented in om-llm.
OpenaiFim uses /completions, OllamaFim /api/generate and MistralFim
/v1/fim/completions with the prescribed provider-specific prompt/suffix/token
fields, temperature0 and newline stops. The actual HttpRequest is nonstreaming.
Chat-kind completion profiles use real nonstreaming insertion-only chat messages
with prefix⟨CURSOR⟩suffix and target-correct Anthropic headers/stop_sequences.
Runtime calls use try_build_fim_request; the fixed-signature convenience defaults
Native and requires validated inputs. No endpoint/model or credential is invented.

parse_fim_response keeps actual raw strings unchanged, including whitespace,
newlines and empty text. It selects genuine choice0 or native response/text
blocks, rejects malformed/missing/wrong-type/tool/nonassistant/incomplete states
and limits the JSON body to one MiB. Known Anthropic reasoning blocks remain
private; only actual text blocks are joined. Remote errors carry untrusted
message access for later profile-aware sanitization, while Display/Debug stays
generic and cannot dump a reflected key. First-line/local-completion/lexer/CAS
filtering and source insertion remain the later M12.4/M12.6/M13 stages.

Provider format checks use [Ollama generate](https://docs.ollama.com/api/generate)
and [Mistral FIM](https://docs.mistral.ai/api/endpoint/fim) primary documentation.
DeepSeek's prescribed compatibility fields are implemented from PLAN; direct
retrieval of its FIM pages timed out during this task, and no live-provider
validation is claimed.


The actual pure Job state machine now consumes these requests/decoders. Feature
and JobInput pairing selects real chat/text/structured/FIM flows; invalid inputs
return Failed without a dummy request. Native on_raw_bytes and browser on_bytes
share checked UTF-8 framing. Actual text, indexed calls and finish markers remain
pending until successful on_http_end; no tools or Done are issued merely because
finish deltas arrived. One-MiB response/text/results and 64-tool bounds apply.

RunTools returns complete actual IDs/names/object arguments in index order for
the later readonly kernel handler. Actual host result IDs must match exactly
once; assistant/tool history replay is provider-correct, including IDs reused in
a later completed round. Six invocation rounds plus a final text HTTP request
are allowed; a seventh invocation fails. The core never invokes CAS or network.

Structured results require a supplied SuggestionValidator; no model modern/latex
is trusted and no default validator fabricates a Suggestion. Failed actual host
validation can retry twice with real sanitized diagnostics. Production prompt/parser/formatter hookup and completion filtering are now
available through the pure host helpers described below. FIM decoding preserves
actual raw response bytes before filtering. TestProfile probes use eight tokens
and actual provider formats.

Done/Failed/cancel are terminal and cached. Late chunks cannot resurrect work.
An actual still-open HTTP round can supply real failure status after stream error,
while ended failures remain unchanged. Provider/transport/retry error messages
exclude known keys/custom-header credentials and 401/403 includes a key hint.
Raw transport authorization remains real. Source insertion, native transport
and kernel LLM request/event orchestration remain M12.5/M12.6/M13.


`om_llm::prompts` embeds translate/explain/fix/chat instructions with `include_str!`.
The eight translation examples match PLAN. Template substitution scans only the
original template, while task/source/diagnostic data remains in separate user
messages. Actual readonly evaluate/solve/propose_cell schemas accompany chat;
the core still performs no tool execution.

`Session::prepare_llm_input` reads actual registered functions, live symbol names,
source, parser diagnostics, CAS messages and retained statement records. Explain
requires a current successful cell with actual recorded steps. It sends genuine
InputForm input/result and the same StepsView used for output rendering; selecting
a step sends only that recorded step and its descendants with stable IDs. Missing,
stale or failed derivations return an error, never a fabricated explanation.

The production SuggestionParser extracts the first opening through last closing
JSON brace (rejecting reversed/missing bounds), requires unique string wolfram and
explanation fields, and ignores all extra model fields. It requires one nonempty
unsuppressed Wolfram expression using the real parser. Wolfram/modern/LaTeX are
rendered from its raw tree, preserving x/x and other original poles. It performs
no evaluation, source insertion, notebook/history or cancellation changes. Job
uses the actual diagnostics for at most two correction retries.

Completion context includes only the preceding three Math sources, using the last
matching current source to identify its notebook position, or the last three
Math cells if no match exists. send_context=false sends only the current prefix
and suffix. Wolfram comment delimiters are escaped; Modern logical lines are
individually commented, including Unicode line separators and dialect markers.
No settings, credentials, rendered output or Text/Ask cell data enters context.
Inputs/context/responses have a deterministic one-MiB bound.

`filter_llm_completion` preserves insertion spacing, strips boundary line breaks,
truncates the first logical newline, and uses real parser lexical diagnostics to
reject illegal characters/escapes/named characters while allowing incomplete
syntax. It checks prefix+insertion+suffix and suppresses the first actual local
completion or its untyped suffix. Protocol job dispatch, native/browser transport
and user-controlled proposal/ghost UI remain subsequent tasks.


With `om-llm/http`, `drive_native(&mut Job, &reqwest::Client, on_event)` drives
actual fresh HTTP rounds using the same checked requests and arbitrary-byte Job
adapter. It automatically follows genuine parse-correction HTTP rounds and stops
at Done/Failed or RunTools for the host to execute; calling it again in terminal
or waiting-tool state does not issue duplicate HTTP. The host resumes after
supplying actual tool_results. Partly pre-fed rounds are rejected instead of
being replayed. This module performs no CAS tool execution or secret lookup.

`drive_native_cancellable` accepts a CancellationToken. Each profile timeout
covers both headers and the complete body of that round. Cancellation drops the
in-flight request and is checked between events in a single received byte chunk,
so a callback cancellation cannot forward later deltas or finish successfully.
Only successful HTTP responses forward model stream events. All bodies, including
non2xx errors, stay within the shared one-MiB limit. Real provider status/messages
pass through Job's sanitizer/key hint; transport failures are classified without
formatting reqwest errors or including URLs, bodies or credentials.

Native product hosts must use `native_client()` (or explicitly disable redirects
on their supplied client). reqwest0.13.5 exposes redirect policy only at client
construction; this API cannot inspect arbitrary external clients. The factory
keeps normal TLS verification and environment proxy settings, and refuses
redirects that might forward provider/custom credentials. The browser transport
and actual kernel native-job lifecycle remain M12.6/M13 integration work.

Thirteen native tests use only loopback wiremock or a controlled chunked socket,
synthetic credentials, real response fixtures and genuine parser delegation.
They cover actual provider authorization/body/text/FIM, two tool rounds, two parse
correction rounds, one-byte UTF-8 chunks, HTTP/key errors, malformed/bounded data,
connection/headers/body failures and cancellation within one response chunk.
Native TLS certificate-data license text is retained in the repository notices.


Session now handles all LlmTranslate/Explain/Complete/Chat/FixError/TestProfile,
LlmHttpChunk/End and LlmCancel requests. The sole Job lives in Session: Browser
is the pure default target, and new_native selects Native with the bound real
credential resolver (environment, vault, raw fallback). A profile mask never
becomes an HTTP key. Feature routing/enabled/profile/capability checks use actual
config; explicit profile tests can run while automatic AI is disabled.

LlmStarted and subsequent LlmHttp carry actual transport DTOs for the host.
Browser transports use them directly. Native product hosts must intercept these
internal DTOs before frontend serialization, using the no-redirect client and
shared drive_native_http raw adapter. They queue actual bytes/status to
llm_http_bytes, with no second Job, reconstructed SSE or lossy packet decoding.
The subsequent desktop/CLI client task must implement this interception/queue.

LlmHttpChunk adds optional status (u16); old absent-status JSON roundtrips
unchanged and uses the legacy success-stream convention. New transports supply
the actual status. Non2xx bodies never become model text; HTTP status, bounded
provider messages/key hints and sanitized failures remain actual Job decisions.
Conflicting chunk/end status fails. Active decode failures terminate immediately;
late chunks/end/cancel cannot resurrect a terminal job. Up to16 active IDs and256
terminal tombstones bound storage; fresh clients must not reuse remembered IDs.
Loading a valid new notebook cancels pending work before replacing records.

Chat/Explain deltas stream actual text. Translate/Fix emit only checked
LlmSuggestion and LlmDone; Completion emits at most one actual filtered
LlmDelta plus LlmDone. Failed work emits LlmError. LlmProfileTest additionally
carries the actual response, total latency_ms and first_byte_ms from the injected
clock; absent clock/bytes remain null. No probe reply or timing is fabricated.

The actual evaluate and solve tools use strict objects, genuine Wolfram parsing,
current readonly definitions, independent cancellation and a5-second injected
clock deadline. Missing/invalid clock fails rather than pretending a guarantee.
Direct/nested mutation forms are refused; indirect user-function writes are
blocked by the real readonly evaluator. Parent notebook/history/definitions,
solver records and normal Interrupt remain untouched. Solve returns genuine
InputForm solutions, actual recorded step title keys/messages and an explicit
supported flag; unsupported input stays unevaluated with real messages.
propose_cell parses the effective requested syntax using live function semantics,
renders the raw tree and emits a card without execution. Six actual tool rounds
are replayed; the seventh fails before invoking CAS.

llm_cancellation_handle can be cloned independently of the Session owner's
borrow/queue, cancelling both the tool flag and native token. Configured keys
reflected in tool arguments are rejected before execution; public arguments and
tool results/replay are sanitized (including nested JSON escaping), while genuine
HTTP authorization stays in the internal transport only. Real default/native
fixtures include action execution, retries, CAS/tool limitations, deadlines,
cancellation, context/lexical completion filtering, byte/status handling and
synthetic native credential precedence. User-facing clients/UI/CLI/packaging and
full actual CLI corpus acceptance remain M13 work.


The native desktop KernelHost now owns a genuine Session::new_native on a dedicated
thread with bounded64-entry messages, matching envelope replies and Channel event
IDs zero. TauriClient subscribes and invokes the actual kernel_request,
kernel_subscribe and kernel_interrupt commands; named secret_set/delete delegate
to the bound profile vault, never echoing raw credentials. Startup storage errors
fail explicitly. Native HTTP DTOs are intercepted before every public reply/event;
the host sends only LlmStarted{http:null} and public tool/text/suggestion/timing or
terminal events. Acknowledged raw byte messages feed the sole Session Job.
The two async IO workers and16 blocking byte workers are bounded; enqueue/ack waits
poll cancellation and the whole request deadline. Host disposal cancels jobs,
interrupts CAS and joins the owner. Real isolated native tests prove math, synthetic
secret operations, actual loopback HTTP, cancellation, interruption/recovery and
busy-owner deadline behavior. The debug desktop executable was built/launched;
visible packaged-window inspection is retained for final desktop acceptance.

The actual om-wasm Kernel class exposes a constructor accepting optional config
JSON and request(JSON Envelope). It injects a browser Date clock and returns
{response: Envelope<Response>, events: Envelope<Event>[]} using genuine Session
dispatch. Bad JSON/zero/unsafe-JS correlation IDs do not execute. A Worker loads
the built bindgen binary; WasmClient pairs replies by ID, distributes events,
disposes waiters on failure and runs one byte-safe browser LlmDriver. Initial
LlmStarted and subsequent LlmHttp events trigger real fetch rounds with actual
status, redirect:error, bounded streaming UTF8, per-profile deadlines and abort.
HTTP continuations come from events, correcting the older PLAN pseudo-code that
expected a nonexistent LlmStarted response after HTTP end.

GetNotebookState returns source-only NotebookFile, per-cell static defines/uses
and status, actual topological definition_order and cycles. RestoreDefinitions
resets the evaluator/records, executes only definition cells through the existing
planner, reports cycles/failure and marks other math cells Stale. KernelRestarted
is the recovery notice. Worker interrupt saves the latest synchronized source and
in-memory config, terminates/replaces the real Worker, rejects pending requests,
loads source and restores definitions; it never runs every computation to recover.
Secrets stay in runtime config and never enter notebook files/recovery metadata.

npm run build:wasm uses exactly wasm-bindgen-cli0.2.129 and the locked release
wasm32 target. dev/test/build generate the ignored JS/WASM package automatically;
no unexplained precompiled artifact is committed. Browser Playwright tests use
this real Worker/binary for Solve, diagnostics, running-computation termination,
definition restore and local mocked provider fetch through the real Job validator.
MockKernel/transport units cover correlation and lifecycle only. Notebook/editor
UI, output/plot/steps/AI/settings, full CLI authority, packages and final docs/E2E
remain subsequent M13 tasks.


The notebook/editor UI now uses real CodeMirror Preview tokens/diagnostics/fixes,
Complete/Hover and byte-to-UTF16 mappings (Greek/surrogate boundaries preserved).
Title edits use RenameNotebook without reloading definitions; ephemeral host
SetSystemLanguage makes persisted Auto follow the UI locale. GetVariables lists
actual live stored definitions rather than treating static source declarations as
live values. Optional CellState.exec_count supplies genuine suppressed execution
indices while old absent-field JSON remains unchanged.

Each UI cell has a source/run generation. Edits synchronize before execution;
outputs/cascade responses are accepted only with matching current-source metadata
after queued edits. Worker interruption reconciles the latest frontend source,
including mutations still waiting in the serialized queue, and restores only
actual definition cells; native interruption preserves the existing evaluator.
Old request cleanup cannot mutate a newer run. Snapshot saves leave unsaved
changes flagged if the user edits during a file save. Browser source-only download
and native explicitly picked dialog/fs files contain only title/version/cells.

Safe conventional Markdown uses a token AST rendered by React, literal raw HTML,
restricted links, no unsolicited remote images and trust:false KaTeX math/code
boundaries. Theme preference alone persists in browser storage. General controls,
keyboard palette, text/math/question cell sources, real docs/variables, restart
notices and responsive375/720/1280 layouts are present; richer solution/region
objects, full steps/plots/ghost/AI/profile panels are still upcoming tasks.

Following the user's authorized current DeepSeek test, extra_body adds optional
explicit provider JSON fields to ProfileConfig/runtime Profile. Empty maps preserve
all old wire/request fixtures. Protocol-owned routing/model/messages/tools/prompt/
stream/sampling/limit fields cannot be overridden; parameters are bounded and their
values are not Debug printed. The verified current editable preset uses
deepseek-flash with thinking disabled for chat/tool/probe requests, preserving
this project's specified no-private-reasoning replay contract. Dedicated FIM uses
the current model on the existing beta endpoint. Thinking-mode tool reasoning
replay is not claimed by this implementation.

The opt-in native example live_deepseek accepts an ephemeral stdin key; ordinary
CI/tests do not invoke it. With the user-provided test credential, actual shared
Session/native driver calls returned200 for probe(pong,~644ms), translation(real
parsed Solve[x^2==4,x,Reals],~675ms), FIM(filtered insertion x,~515ms), and a two-round
chat that actually invoked the readonly solve tool and cited its x=-2/x=2 CAS
result(~1907ms). No key was written to config/notebook/source/logs and no existing
user notebook context was transmitted. These live results are separate evidence
from offline fixture gates. Official sources: https://api-docs.deepseek.com/ and
https://api-docs.deepseek.com/api/create-chat-completion/.

Output inspection uses `inspect_expression {source, numeric}` and returns
`expression {value: {input_form, modern_form, latex}}`. It accepts one bounded
Wolfram expression and evaluates an isolated readonly fork; numeric=true calls
actual N[source,20]. Notebook cells, output history and parent cancellation are
unchanged. Direct/nested assignments and clearing are rejected; indirect writes
remain prohibited by readonly definitions. Host clock deadlines are capped at
five seconds, with an independent 1,048,576-step budget.

BindingView adds optional var_latex/root_index/radicals from actual formatting,
Root AST and supported ToRadicals conversion. Numeric exact-value hints now
come from twenty-digit N; plotting coordinates keep their previous precision.
SolutionView adds optional condition_display_latex for k ∈ ℤ style display while
retaining legacy condition_latex. Generated constants have collision-free
display aliases; copy source always retains the original C[n] objects and rules.
Repeated rows are grouped only for chip display; copy-all retains multiplicity.
No solutions, all values, finite roots and regions retain their genuine set kind
and actual Exact/ByConstruction/Numeric/Unverified evidence. Basic inline plots
use actual SamplePlot data; viewport/slider interactions remain M13.5.
