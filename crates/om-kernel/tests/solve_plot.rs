//! Automatic plots use actual source/set evidence and transported parameter solving.
pub mod support;
use om_kernel::{Session, protocol::*};
use support::*;
fn plot(o: &CellOutput) -> PlotRequest {
    let OutputItem::Solutions { plot: Some(p), .. } = &o.items[0] else {
        panic!("{o:?}")
    };
    p.clone()
}
fn solve(s: &mut Session, src: &str) -> CellOutput {
    output(s, "solve", src, Dialect::Wolfram)
}
#[test]
fn equality_plots_use_actual_sides_roots_and_minimum_viewport() {
    let mut s = sequential();
    let o = solve(&mut s, "Solve[x^2==4,x]");
    let p = plot(&o);
    assert_eq!(p.kind, PlotKind::Function);
    assert_eq!(p.exprs.len(), 2);
    assert_eq!(p.var_x, "x");
    assert_eq!(p.x_range, (-4.0, 4.0));
    assert_eq!(p.points, [(-2.0, 4.0), (2.0, 4.0)]);
    same(&p.exprs[0], "x^2");
    same(&p.exprs[1], "4");
    let p = plot(&solve(&mut s, "Solve[x==1,x]"));
    assert_eq!(p.x_range, (-2.0, 4.0));
    assert_eq!(p.points, [(1.0, 1.0)]);
    let p = plot(&solve(&mut s, "Solve[x^2==-1,x,Reals]"));
    assert_eq!(p.x_range, (-5.0, 5.0));
    assert!(p.points.is_empty());
    assert_eq!(s.notebook.cells[0].exec_count, Some(3));
}
#[test]
fn actual_radical_root_numeric_and_reactive_points_are_finite_and_correct() {
    let mut s = sequential();
    for src in [
        "Solve[x^2==2,x]",
        "Solve[x^3-x-1==0,x,Reals]",
        "NSolve[x^2==2,x]",
        "FindRoot[x^2==2,{x,1}]",
    ] {
        let p = plot(&solve(&mut s, src));
        assert!(!p.points.is_empty());
        for (x, y) in &p.points {
            assert!(x.is_finite() && y.is_finite());
            if src.contains("x^2") {
                assert!((x * x - 2.0).abs() < 1e-10);
            }
        }
    }
    let mut s = Session::new(Default::default(), None);
    for (id, src) in [("a", "let a=2"), ("b", "solve(x^2=a,x)")] {
        s.handle(Request::Evaluate {
            cell_id: id.into(),
            source: src.into(),
            dialect: Dialect::Modern,
        });
    }
    let (r, e) = s.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "let a=9".into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { reran, .. } = r else {
        panic!()
    };
    assert_eq!(reran, ["b"]);
    let Event::CellOutput { output: o, .. } = e.last().unwrap() else {
        panic!()
    };
    assert_eq!(plot(o).points, [(-3.0, 9.0), (3.0, 9.0)]);
    assert_eq!(s.notebook.cells[1].exec_count, Some(4));
}
#[test]
fn actual_regions_shade_unbounded_intervals_and_two_axis_equations_highlight_real_solutions() {
    let mut s = sequential();
    let p = plot(&solve(&mut s, "Reduce[(x-1)/(x+2)>=0,x]"));
    assert_eq!(p.shade, [(-1e308, -2.0), (1.0, 1e308)]);
    assert_eq!(p.exprs.len(), 1);
    let p = plot(&solve(&mut s, "Solve[{x+y==1,x-y==0},{x,y}]"));
    assert_eq!(p.kind, PlotKind::Implicit);
    assert_eq!(p.var_y.as_deref(), Some("y"));
    assert_eq!(p.exprs.len(), 2);
    assert_eq!(p.points, [(0.5, 0.5)]);
    assert!(p.y_range.is_some());
}
#[test]
fn unsupported_nonreal_infinite_free_axes_and_disabled_auto_plot_do_not_gain_plots() {
    let mut s = sequential();
    for src in [
        "Solve[x^2==-1,x]",
        "Solve[x==x,x]",
        "Solve[x+y==1,{x,y}]",
        "Solve[Sin[x]==0,x]",
        "Solve[{x==1,y==2,z==3},{x,y,z}]",
    ] {
        let o = solve(&mut s, src);
        assert!(
            matches!(&o.items[0], OutputItem::Solutions { plot: None, .. }),
            "{src}: {o:?}"
        );
    }
    s.config.general.auto_plot = false;
    let o = solve(&mut s, "Solve[x^2==4,x]");
    assert!(matches!(
        o.items[0],
        OutputItem::Solutions { plot: None, .. }
    ));
    s.config.general.auto_plot = true;
    s.config.general.show_steps = false;
    let o = solve(&mut s, "Solve[x^2==4,x]");
    assert!(matches!(
        o.items[0],
        OutputItem::Solutions {
            plot: Some(_),
            steps: None,
            ..
        }
    ));
}
