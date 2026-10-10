//! Non-evaluating notebook source analysis shared by native transactions and kernel owners.
use crate::{
    config::{ConfigDialect, Constants},
    protocol::{CellKind, Diagnostic, Dialect, NotebookFile},
};
use om_core::{BUILTIN as B, Symbol};
use om_parse::{ParseEnv, ParseOutput};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Actual syntactic dependency facts. These are not evaluated values or live definitions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceCellAnalysis {
    /// Original stable cell identity.
    pub cell_id: String,
    /// Current declared symbols, sorted by name.
    pub defines: Vec<String>,
    /// Actual lexical free symbols and function heads.
    pub uses: Vec<String>,
    /// Syntactic function declarations used only as parser context.
    pub function_names: Vec<String>,
    /// Actual source diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}
/// Duplicate current source ownership, independent of whether a definition has been executed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceDefinitionConflict {
    /// Conflicting symbol.
    pub symbol: String,
    /// Declaring cell IDs in notebook order.
    pub cell_ids: Vec<String>,
}
/// Complete source-only graph, with no Session/Evaluator construction or statement execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceAnalysis {
    /// Source cells, including empty facts for prose/Ask.
    pub cells: Vec<SourceCellAnalysis>,
    /// Layer-stable dependency ordering of acyclic Math cells.
    pub order: Vec<String>,
    /// True cycle members, distinct from downstream blocked nodes.
    pub cycles: Vec<String>,
    /// Downstream nodes whose prerequisites contain a cycle.
    pub blocked: Vec<String>,
    /// Duplicate definition owners.
    pub conflicts: Vec<SourceDefinitionConflict>,
    /// Directed source edges (definer -> user), in stable order.
    pub edges: Vec<(String, String)>,
}
fn dialect(source: &str, requested: Dialect, config: ConfigDialect) -> om_parse::Dialect {
    match (requested, config) {
        (Dialect::Auto, ConfigDialect::Modern) => om_parse::Dialect::Modern,
        (Dialect::Auto, ConfigDialect::Wolfram) => om_parse::Dialect::Wolfram,
        (Dialect::Auto, _) => om_parse::detect_dialect(source),
        _ => requested.into(),
    }
}
fn functions(parsed: &ParseOutput) -> BTreeSet<Symbol> {
    let mut names = BTreeSet::new();
    for statement in &parsed.statements {
        let expr = &statement.expr;
        if (expr.is_head(B::SET) || expr.is_head(B::SET_DELAYED)) && expr.args().len() == 2 {
            let mut lhs = &expr.args()[0];
            while lhs.is_head(B::CONDITION) && lhs.args().len() == 2 {
                lhs = &lhs.args()[0]
            }
            if lhs.as_symbol().is_none()
                && !lhs.is_head(B::LIST)
                && !lhs.is_head(B::PART)
                && let Some(head) = lhs.head_symbol()
            {
                names.insert(head);
            }
        }
    }
    names
}
/// Analyze all source in a frozen file. Known function bindings come from trusted active state;
/// source declarations are discovered without evaluation, including declarations in later cells.
pub fn analyze_source(
    file: &NotebookFile,
    config: ConfigDialect,
    constants: Constants,
    known_functions: &[String],
) -> Result<SourceAnalysis, String> {
    if file.version != 1
        || file.cells.len() > 10000
        || file
            .cells
            .iter()
            .map(|c| c.source.len() + c.id.len())
            .sum::<usize>()
            > 2 * 1024 * 1024
    {
        return Err("SOURCE_BUDGET_EXCEEDED".into());
    }
    let mut ids = BTreeSet::new();
    if file
        .cells
        .iter()
        .any(|c| c.id.is_empty() || !ids.insert(c.id.as_str()))
    {
        return Err("INVALID_CELL_ID".into());
    }
    let mut env = ParseEnv {
        constants: match constants {
            Constants::Math => om_parse::ConstantMode::Math,
            Constants::Strict => om_parse::ConstantMode::Strict,
        },
        known_functions: known_functions.iter().map(|n| Symbol::intern(n)).collect(),
    };
    for cell in &file.cells {
        if cell.kind == CellKind::Math {
            env.known_functions.extend(functions(&om_parse::parse_with(
                &cell.source,
                dialect(&cell.source, cell.dialect, config),
                &env,
            )));
        }
    }
    let mut cells = Vec::new();
    for cell in &file.cells {
        if cell.kind != CellKind::Math {
            cells.push(SourceCellAnalysis {
                cell_id: cell.id.clone(),
                defines: vec![],
                uses: vec![],
                function_names: vec![],
                diagnostics: vec![],
            });
            continue;
        }
        let parsed = om_parse::parse_with(
            &cell.source,
            dialect(&cell.source, cell.dialect, config),
            &env,
        );
        let (defines, uses) = crate::dependency::analyze(&parsed);
        let sorted = |symbols: BTreeSet<Symbol>| {
            let mut result = symbols
                .into_iter()
                .map(|s| s.name().to_owned())
                .collect::<Vec<_>>();
            result.sort();
            result
        };
        cells.push(SourceCellAnalysis {
            cell_id: cell.id.clone(),
            defines: sorted(defines),
            uses: sorted(uses),
            function_names: sorted(functions(&parsed)),
            diagnostics: parsed.diagnostics.iter().map(Diagnostic::from).collect(),
        });
    }
    let mut owners: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, cell) in cells.iter().enumerate() {
        for name in &cell.defines {
            owners.entry(name.clone()).or_default().push(index);
        }
    }
    let conflicts = owners
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .map(|(name, indices)| SourceDefinitionConflict {
            symbol: name.clone(),
            cell_ids: indices.iter().map(|&i| cells[i].cell_id.clone()).collect(),
        })
        .collect();
    let mut outgoing = vec![BTreeSet::new(); cells.len()];
    let mut degrees = vec![0; cells.len()];
    let mut edges = Vec::new();
    for (to, cell) in cells.iter().enumerate() {
        for name in &cell.uses {
            if let Some(froms) = owners.get(name) {
                for &from in froms {
                    if from != to && outgoing[from].insert(to) {
                        if edges.len() >= 200_000 {
                            return Err("DEPENDENCY_BUDGET_EXCEEDED".into());
                        }
                        degrees[to] += 1;
                        edges.push((cells[from].cell_id.clone(), cell.cell_id.clone()));
                    }
                }
            }
        }
    }
    let mut remaining = file
        .cells
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == CellKind::Math)
        .map(|(i, _)| i)
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::new();
    loop {
        let layer = remaining
            .iter()
            .copied()
            .filter(|&i| degrees[i] == 0)
            .collect::<Vec<_>>();
        if layer.is_empty() {
            break;
        }
        for i in layer {
            remaining.remove(&i);
            ordered.push(cells[i].cell_id.clone());
            for &to in &outgoing[i] {
                degrees[to] -= 1;
            }
        }
    }
    // Iterative Kosaraju identifies true SCC members in O(V+E), without recursive stacks.
    let mut visited = BTreeSet::new();
    let mut finished = Vec::new();
    for &start in &remaining {
        let mut stack = vec![(start, false)];
        while let Some((i, done)) = stack.pop() {
            if done {
                finished.push(i);
                continue;
            }
            if !visited.insert(i) {
                continue;
            }
            stack.push((i, true));
            for &next in outgoing[i].iter().rev() {
                if remaining.contains(&next) && !visited.contains(&next) {
                    stack.push((next, false));
                }
            }
        }
    }
    let mut reverse = vec![Vec::new(); cells.len()];
    for &from in &remaining {
        for &to in &outgoing[from] {
            if remaining.contains(&to) {
                reverse[to].push(from);
            }
        }
    }
    visited.clear();
    let mut cyclic = BTreeSet::new();
    for &start in finished.iter().rev() {
        if visited.contains(&start) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            if visited.insert(i) {
                component.push(i);
                stack.extend(&reverse[i]);
            }
        }
        if component.len() > 1 {
            cyclic.extend(component);
        }
    }
    let cycles = cyclic.iter().map(|&i| cells[i].cell_id.clone()).collect();
    let blocked = remaining
        .difference(&cyclic)
        .map(|&i| cells[i].cell_id.clone())
        .collect();
    Ok(SourceAnalysis {
        cells,
        order: ordered,
        cycles,
        blocked,
        conflicts,
        edges,
    })
}

/// True only for changes to the relative Math sequence or raw Math content/kind/dialect.
/// Title/prose/Ask edits and pure prose reordering cannot change CAS execution by themselves.
pub fn math_source_changed(before: &NotebookFile, after: &NotebookFile) -> bool {
    before
        .cells
        .iter()
        .filter(|c| c.kind == CellKind::Math)
        .map(|c| (&c.id, &c.source, c.dialect))
        .ne(after
            .cells
            .iter()
            .filter(|c| c.kind == CellKind::Math)
            .map(|c| (&c.id, &c.source, c.dialect)))
}

/// Actual live definition ownership contributes invalidation even if syntax no longer declares it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceOwnership {
    /// Original symbol name.
    pub symbol: String,
    /// The cell that actually executed the write.
    pub cell_id: String,
}
/// Actual current live definition context, never inferred by executing or parsing notebook text.
#[derive(Clone, Debug)]
pub struct SourceExecutionContext {
    /// Actual current user function names.
    pub known_functions: Vec<String>,
    /// Actual successful write owners, including partial effects.
    pub owned: Vec<SourceOwnership>,
}
/// Non-executing effect plan. A computation worker applies it only after source commit acceptance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceInvalidation {
    /// Whether main mathematical execution can change.
    pub math_changed: bool,
    /// All stale affected Math cells, sorted by identity.
    pub affected_cells: Vec<String>,
    /// Actual old definition owners to retire, not an instruction to execute source.
    pub retire_owner_cells: Vec<String>,
    /// Symbol changes including old successful ownership.
    pub changed_symbols: Vec<String>,
    /// Current source-only dependency facts.
    pub analysis: SourceAnalysis,
}
/// Discover invalidation from both old/new lexical dependencies and actual write ownership.
pub fn assess_source_change(
    before: &NotebookFile,
    after: &NotebookFile,
    config: ConfigDialect,
    constants: Constants,
    known_functions: &[String],
    owned: &[SourceOwnership],
    calculation_changed: bool,
) -> Result<SourceInvalidation, String> {
    let previous = analyze_source(before, config, constants, known_functions)?;
    let math_order = |file: &NotebookFile| {
        file.cells
            .iter()
            .filter(|c| c.kind == CellKind::Math)
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    };
    let order_changed = math_order(before) != math_order(after);
    let math_changed = calculation_changed || math_source_changed(before, after);
    let mut affected = BTreeSet::new();
    if math_changed {
        for cell in before
            .cells
            .iter()
            .chain(&after.cells)
            .filter(|c| c.kind == CellKind::Math)
        {
            let old = before.cells.iter().find(|c| c.id == cell.id);
            let new = after.cells.iter().find(|c| c.id == cell.id);
            if order_changed
                || calculation_changed
                || old.zip(new).is_none_or(|(a, b)| {
                    a.kind != b.kind || a.source != b.source || a.dialect != b.dialect
                })
            {
                affected.insert(cell.id.clone());
            }
        }
    }
    let retired_functions = owned
        .iter()
        .filter(|o| affected.contains(&o.cell_id))
        .map(|o| o.symbol.as_str())
        .collect::<BTreeSet<_>>();
    let after_functions = known_functions
        .iter()
        .filter(|n| !retired_functions.contains(n.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let analysis = analyze_source(after, config, constants, &after_functions)?;
    let mut changed = BTreeSet::new();
    loop {
        for cell in previous
            .cells
            .iter()
            .chain(&analysis.cells)
            .filter(|c| affected.contains(&c.cell_id))
        {
            changed.extend(cell.defines.iter().cloned());
        }
        for owner in owned.iter().filter(|o| affected.contains(&o.cell_id)) {
            changed.insert(owner.symbol.clone());
        }
        let mut added = false;
        for cell in previous.cells.iter().chain(&analysis.cells) {
            if cell.uses.iter().any(|name| changed.contains(name))
                && affected.insert(cell.cell_id.clone())
            {
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    let retire_owner_cells = owned
        .iter()
        .filter(|o| affected.contains(&o.cell_id))
        .map(|o| o.cell_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(SourceInvalidation {
        math_changed,
        affected_cells: affected.into_iter().collect(),
        retire_owner_cells,
        changed_symbols: changed.into_iter().collect(),
        analysis,
    })
}
/// UI language changes do not alter CAS semantics; all other current General settings can alter
/// parsing, execution policy, work budget or produced mathematical diagnostics/steps/plots.
pub fn calculation_settings_changed(
    before: &crate::config::GeneralConfig,
    after: &crate::config::GeneralConfig,
) -> bool {
    before.dialect != after.dialect
        || before.constants != after.constants
        || before.reactive != after.reactive
        || before.auto_run_dependents != after.auto_run_dependents
        || before.show_steps != after.show_steps
        || before.auto_plot != after.auto_plot
        || before.eval_timeout_ms != after.eval_timeout_ms
}
