//! Prepare authentic source plans for physical retention tests; preparation is not persistence.
use om_host_service::{
    document::SourceDocument,
    protocol::{Serial, generated::*},
};
fn main() {
    let mut owner = SourceDocument::new(
        "00000000-0000-0000-0000-000000000041".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "history fixture".into(),
            cells: vec![NativeSourceCell {
                id: "cell".into(),
                kind: NativeCellKind::Math,
                source: "let a=0".into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap();
    let initial = owner.snapshot().clone();
    let mut plans = Vec::new();
    for n in 1..=206 {
        let mut file = owner.snapshot().file.clone();
        file.cells[0].source = format!("let a={n}\n#中文🙂 {}", "history bytes ".repeat(512));
        let plan = owner
            .prepare(
                file,
                format!("op-{n}"),
                format!("tx-{n}"),
                format!("event-{n}"),
                "prepared-only-time".into(),
            )
            .unwrap();
        owner = SourceDocument::restore(plan.after.clone(), Serial::new(1).unwrap()).unwrap();
        plans.push(plan);
    }
    println!(
        "{}",
        serde_json::json!({"initial":initial,"plans":plans,"preparation_only":true})
    );
}
