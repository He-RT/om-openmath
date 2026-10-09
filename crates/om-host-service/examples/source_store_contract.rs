//! Export genuine Rust frozen plans or verify the physical Swift receipt against the owner.
use om_host_service::{
    document::SourceDocument,
    protocol::{Serial, generated::*},
};
fn file(source: &str) -> NativeSourceFile {
    NativeSourceFile {
        version: 1,
        title: "中文🙂 科研笔记 e\u{301}".into(),
        cells: vec![
            NativeSourceCell {
                id: "cell-1".into(),
                kind: NativeCellKind::Math,
                source: source.into(),
                dialect: Dialect::Modern,
            },
            NativeSourceCell {
                id: "单元格🙂e\u{301}".into(),
                kind: NativeCellKind::Text,
                source: "α + emoji🙂 and e\u{301}\0 raw".into(),
                dialect: Dialect::Auto,
            },
        ],
    }
}
fn main() {
    let doc = "00000000-0000-0000-0000-000000000041".to_owned();
    let mut owner =
        SourceDocument::new(doc, Serial::new(1).unwrap(), file("let f(x)=\n x+1;f(4)")).unwrap();
    let first = owner
        .prepare(
            file("let a=2; a+1"),
            "op-one".into(),
            "tx-one".into(),
            "event-one".into(),
            "2026-10-09T00:00:00Z".into(),
        )
        .unwrap();
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() == 3 && args[1] == "verify" {
        let receipt = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
        owner.accept(&first, &receipt).unwrap();
        assert_eq!(owner.snapshot().revision.get(), 1);
        println!("Actual Swift physical receipt accepted by original Rust source authority");
        return;
    }
    // Prepare a later fixture from candidate data only; this is not a durable acceptance.
    let owner = SourceDocument::restore(first.after.clone(), Serial::new(1).unwrap()).unwrap();
    let second = owner
        .prepare(
            file("let a=5; a+1"),
            "op-two".into(),
            "tx-two".into(),
            "event-two".into(),
            "2026-10-09T00:00:01Z".into(),
        )
        .unwrap();
    println!(
        "{}",
        serde_json::json!({"initial":first.before,"first":first,"second":second,"preparation_only":true})
    );
}
