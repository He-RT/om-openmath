//! Shared execution budgets, actual derivations and timing edge cases.
use super::*;
use om_num::ctx::Clock;
use std::{
    cell::Cell as Budget,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[test]
fn one_shared_budget_and_successful_history_survive_cell_failure() {
    let source = "let a=2;\na+1\nlet b=4";
    let parsed = om_parse::parse(source, om_parse::Dialect::Modern);
    let mut baseline = Session::new(Default::default(), None);
    let ctx = Interrupt::default();
    let before = ctx.steps_left.get();
    let result = baseline.execute(&parsed, &ctx);
    assert_eq!(result.status, CellStatus::Done);
    let consumed = before - ctx.steps_left.get();
    assert!(consumed > 3);
    for budget in [0, 1, consumed - 1] {
        let mut session = Session::new(Default::default(), None);
        let ctx = Interrupt {
            steps_left: Budget::new(budget),
            ..Interrupt::default()
        };
        let result = session.execute(&parsed, &ctx);
        assert_eq!(result.status, CellStatus::Error);
        assert!(
            result
                .output
                .messages
                .iter()
                .any(|message| message.tag == "budget")
        );
        assert_eq!(ctx.steps_left.get(), 0);
        assert_eq!(session.eval.history.len(), result.records.len());
        assert!(session.eval.last_steps.is_none());
        if budget == 0 {
            assert!(session.eval.history.is_empty());
        }
    }
    let mut session = Session::new(Default::default(), None);
    let exact = Interrupt {
        steps_left: Budget::new(consumed),
        ..Interrupt::default()
    };
    assert_eq!(session.execute(&parsed, &exact).status, CellStatus::Done);
}

struct SequenceClock {
    values: Vec<f64>,
    index: AtomicUsize,
}
impl Clock for SequenceClock {
    fn now_ms(&self) -> f64 {
        self.values[self
            .index
            .fetch_add(1, Ordering::Relaxed)
            .min(self.values.len() - 1)]
    }
}

#[test]
fn timing_always_serializes_as_a_finite_nonnegative_number() {
    for values in [vec![10.0, 9.0, 8.0], vec![0.0, 0.0, f64::INFINITY]] {
        let clock = Arc::new(SequenceClock {
            values,
            index: AtomicUsize::new(0),
        });
        let mut session = Session::new(Default::default(), Some(clock));
        let response = session
            .handle(Request::Evaluate {
                cell_id: "a".into(),
                source: "1".into(),
                dialect: Dialect::Modern,
            })
            .0;
        let Response::Evaluated { output, .. } = &response else {
            panic!()
        };
        assert_eq!(output.timing_ms, 0.0);
        let wire = serde_json::to_value(response).unwrap();
        assert_eq!(wire["output"]["timing_ms"], 0.0);
    }
}

#[test]
fn error_level_evaluator_messages_keep_the_held_value_and_mark_the_cell_error() {
    let mut session = Session::new(Default::default(), None);
    session.eval.settings.iteration_limit = 3;
    for source in ["f[1]:=f[2]", "f[2]:=f[1]"] {
        let input = om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap();
        session
            .eval
            .evaluate_statement(&input, &Interrupt::default())
            .unwrap();
    }
    let response = session
        .handle(Request::Evaluate {
            cell_id: "loop".into(),
            source: "f[1]".into(),
            dialect: Dialect::Wolfram,
        })
        .0;
    let Response::Evaluated { output, .. } = response else {
        panic!()
    };
    assert!(
        output
            .messages
            .iter()
            .any(|message| message.tag == "itlim" && message.level == MsgLevel::Error)
    );
    assert!(output.items.iter().any(
        |item| matches!(item,OutputItem::Expr{input_form,..} if input_form.starts_with("Hold["))
    ));
    assert_eq!(session.notebook.cells[0].status, CellStatus::Error);
    assert_eq!(session.notebook.cells[0].exec_count, Some(3));
}

#[test]
fn known_function_reads_follow_precedence_unset_and_never_execute_computed_values() {
    let mut ev = om_eval::Evaluator::new();
    let run = |ev: &mut om_eval::Evaluator, source: &str| {
        ev.evaluate_statement(
            &om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap(),
            &Interrupt::default(),
        )
        .unwrap()
    };
    let f = om_core::Symbol::intern("f");
    run(&mut ev, "f[1]=2");
    assert!(ev.defs.known_functions().contains(&f));
    run(&mut ev, "f=7");
    assert!(!ev.defs.known_functions().contains(&f));
    run(&mut ev, "Unset[f]");
    assert!(ev.defs.known_functions().contains(&f));
    run(&mut ev, "Unset[f[1]]");
    assert!(!ev.defs.known_functions().contains(&f));
    run(&mut ev, "counter=0");
    run(&mut ev, "computed:=(counter=counter+1;#+1&)");
    assert!(
        !ev.defs
            .known_functions()
            .contains(&om_core::Symbol::intern("computed"))
    );
    assert_eq!(run(&mut ev, "counter"), om_core::Expr::int(0));
}
