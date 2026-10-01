# Kernel JSON protocol

`om_kernel::protocol` defines the shared request, response and event schema.
`om_kernel::Session` implements Evaluate, GetConfig, SetConfig, LoadNotebook,
SaveNotebook and Interrupt in M11.2. Reactive notebook execution, specialized
output views, plotting/editor helpers and LLM jobs follow in the remaining
M11/M12 tasks. The CLI, WASM worker and desktop host will use this same contract.

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
resolution are implemented in M11.8.

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
Preview/completion/hover and LLM handlers remain later milestones and
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
