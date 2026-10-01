//! Tree assembly and lazy disabled recording.
use super::{Level, Step, StepKind, Steps};
/// Event consumer used directly by the executing solver algorithms.
pub trait StepSink {
    /// Append an already constructed event.
    fn push(&mut self, s: Step);
    /// Begin a computational branch.
    fn enter(&mut self, label: &str);
    /// End the innermost computational branch.
    fn exit(&mut self);
    /// Whether events should be constructed.
    fn enabled(&self) -> bool;
    /// Construct an event only when recording is enabled.
    fn record(&mut self, build: impl FnOnce() -> Step)
    where
        Self: Sized,
    {
        if self.enabled() {
            self.push(build());
        }
    }
}
/// Disabled zero-sized event consumer.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSteps;
impl StepSink for NoSteps {
    fn push(&mut self, _s: Step) {}
    fn enter(&mut self, _label: &str) {}
    fn exit(&mut self) {}
    fn enabled(&self) -> bool {
        false
    }
}
/// Ordered hierarchical recorder; open branches are finalized by finish.
#[derive(Default)]
pub struct StepRecorder {
    root: Vec<Step>,
    stack: Vec<Step>,
}
impl StepRecorder {
    /// Start an empty enabled recorder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Close all pending branches and return the recorded tree.
    pub fn finish(mut self) -> Steps {
        while !self.stack.is_empty() {
            self.exit();
        }
        Steps { root: self.root }
    }
    fn next_id(&self) -> String {
        if let Some(parent) = self.stack.last() {
            format!("{}.{}", parent.id, parent.children.len() + 1)
        } else {
            format!("S{}", self.root.len() + 1)
        }
    }
    fn append(&mut self, s: Step) {
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(s);
        } else {
            self.root.push(s);
        }
    }
}
impl StepSink for StepRecorder {
    fn push(&mut self, mut s: Step) {
        let mut pending = vec![(&mut s, self.next_id())];
        while let Some((step, id)) = pending.pop() {
            step.id = id;
            step.rule_id = step.kind.rule_id();
            for (i, child) in step.children.iter_mut().enumerate().rev() {
                pending.push((child, format!("{}.{}", step.id, i + 1)));
            }
        }
        self.append(s);
    }
    fn enter(&mut self, label: &str) {
        let mut branch = Step::new(
            StepKind::Branch {
                label: label.into(),
            },
            vec![],
            vec![],
            Level::Major,
        );
        branch.id = self.next_id();
        self.stack.push(branch);
    }
    fn exit(&mut self) {
        if let Some(branch) = self.stack.pop() {
            self.append(branch);
        }
    }
    fn enabled(&self) -> bool {
        true
    }
}
