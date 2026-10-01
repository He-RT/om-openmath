//! Native warm-preview acceptance against the actual five-millisecond requirement.
use om_kernel::{Session, protocol::*};
#[test]
#[ignore = "run explicitly with cargo test -p om-kernel --release --test editor_latency -- --ignored --nocapture"]
fn warm_1000_character_previews_finish_under_five_milliseconds() {
    let mut session = Session::new(Default::default(), None);
    let sources = [
        "solve(x^2+2x=3,x)".to_string(),
        "α  (x)".to_string(),
        (0..80)
            .map(|i| format!("x^2+{i}y"))
            .collect::<Vec<_>>()
            .join("+"),
        "# commentary\n".repeat(65) + "(x^2-1)/(x-1)=0",
    ];
    for source in sources {
        assert!(source.chars().count() <= 1000);
        let request = Request::Preview {
            source: source.clone(),
            dialect: Dialect::Modern,
            cursor: None,
        };
        for _ in 0..8 {
            assert!(matches!(
                session.handle(request.clone()).0,
                Response::Preview(_)
            ));
        }
        let mut durations = vec![];
        for _ in 0..64 {
            let start = std::time::Instant::now();
            let (response, events) = session.handle(request.clone());
            let elapsed = start.elapsed();
            assert!(matches!(response, Response::Preview(_)));
            assert!(events.is_empty());
            durations.push(elapsed);
        }
        durations.sort();
        let median = durations[32];
        let max = durations[63];
        eprintln!(
            "{} chars: median {:?}, maximum {:?}",
            source.chars().count(),
            median,
            max
        );
        assert!(
            max < std::time::Duration::from_millis(5),
            "actual warm preview exceeds 5ms: {max:?}"
        );
    }
    assert!(session.notebook.cells.is_empty());
}
