//! Dependency closure and layer-stable Kahn ordering without evaluating source.
use super::Session;
use crate::protocol::CellKind;
use om_core::Symbol;
use std::collections::BTreeSet;

pub(super) struct Plan {
    pub order: Vec<usize>,
    pub cycles: BTreeSet<usize>,
    pub blocked: BTreeSet<usize>,
    pub edges: Vec<BTreeSet<usize>>,
}
impl Session {
    pub(super) fn cell_symbols(&self, index: usize) -> BTreeSet<Symbol> {
        let cell = &self.notebook.cells[index];
        let mut symbols = cell.defines.clone();
        symbols.extend(
            self.owners
                .iter()
                .filter(|(_, owner)| *owner == &cell.id)
                .map(|(&s, _)| s),
        );
        symbols
    }

    pub(super) fn affected(&self, mut changed: BTreeSet<Symbol>) -> BTreeSet<usize> {
        let mut affected = BTreeSet::new();
        loop {
            let mut added = false;
            for (index, cell) in self.notebook.cells.iter().enumerate() {
                if cell.kind == CellKind::Math
                    && !affected.contains(&index)
                    && !cell.uses.is_disjoint(&changed)
                {
                    affected.insert(index);
                    changed.extend(self.cell_symbols(index));
                    added = true;
                }
            }
            if !added {
                return affected;
            }
        }
    }

    pub(super) fn plan(&self, selected: &BTreeSet<usize>) -> Plan {
        let mut edges = vec![BTreeSet::new(); self.notebook.cells.len()];
        let mut degrees = vec![0; edges.len()];
        for &from in selected {
            // Old live ownership propagates invalidation, but only current source defines an edge.
            let defines = &self.notebook.cells[from].defines;
            for &to in selected {
                if from != to && !defines.is_disjoint(&self.notebook.cells[to].uses) {
                    edges[from].insert(to);
                    degrees[to] += 1;
                }
            }
        }
        let mut remaining = selected.clone();
        let mut order = Vec::new();
        loop {
            let ready: Vec<_> = remaining
                .iter()
                .copied()
                .filter(|&i| degrees[i] == 0)
                .collect();
            if ready.is_empty() {
                break;
            }
            for i in ready {
                remaining.remove(&i);
                order.push(i);
                for &to in &edges[i] {
                    degrees[to] -= 1;
                }
            }
        }
        // A Kahn remainder also contains downstream nodes. Only return-to-self paths are cycles.
        let mut cycles = BTreeSet::new();
        for &start in &remaining {
            let mut work: Vec<_> = edges[start].iter().copied().collect();
            let mut visited = BTreeSet::new();
            while let Some(i) = work.pop() {
                if i == start {
                    cycles.insert(start);
                    break;
                }
                if visited.insert(i) {
                    work.extend(edges[i].iter().copied());
                }
            }
        }
        let blocked = remaining.difference(&cycles).copied().collect();
        Plan {
            order,
            cycles,
            blocked,
            edges,
        }
    }
}
