//! Preview, evaluation, dependencies and geometry consume the same continuation parser.
pub mod support;
use om_kernel::{Session, protocol::*};
use support::{expressions, output, sequential};

fn preview(session: &mut Session, source: &str) {
    let (response, events) = session.handle(Request::Preview {
        source: source.into(),
        dialect: Dialect::Modern,
        cursor: None,
    });
    assert!(events.is_empty());
    let Response::Preview(result) = response else {
        panic!("{response:?}")
    };
    assert!(
        result.diagnostics.is_empty(),
        "{source}: {:?}",
        result.diagnostics
    );
}

#[test]
fn preview_is_nonexecuting_and_evaluation_uses_the_exact_same_multiline_source() {
    for (source, expected) in [
        ("let square(x) =\n  x^2+1;\nsquare(3)", "10"),
        ("let cube = fn(x) =>\n x^3;\nmap(cube,[2])", "{8}"),
        ("[1,2,3] |>\n map(fn(x)=>x^2)", "{1, 4, 9}"),
        ("1 + # 中文 🧮\r\n\r\n 2", "3"),
    ] {
        let mut session = sequential();
        preview(&mut session, source);
        assert!(session.notebook.cells.is_empty());
        let actual = output(&mut session, "source", source, Dialect::Modern);
        assert!(actual.messages.is_empty(), "{actual:?}");
        assert_eq!(expressions(&actual).last().unwrap().1, expected);
    }
}

#[test]
fn multiline_definitions_preserve_real_dependency_recalculation() {
    let mut session = Session::new(Default::default(), None);
    for (cell, source) in [("a", "let a =\n 2"), ("b", "a+1")] {
        session.handle(Request::Evaluate {
            cell_id: cell.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        });
    }
    let (response, events) = session.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "let a =\n 5".into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { reran, .. } = response else {
        panic!("{response:?}")
    };
    assert_eq!(reran, ["b"]);
    assert!(!events.is_empty());
    let output = session
        .notebook
        .cells
        .iter()
        .find(|c| c.id == "b")
        .unwrap()
        .output
        .as_ref()
        .unwrap();
    assert_eq!(expressions(output).last().unwrap().1, "6");
}

fn grass_geometry(source: &str) -> Scene3DData {
    let mut session = sequential();
    preview(&mut session, source);
    let result = output(&mut session, "grass", source, Dialect::Modern);
    assert!(result.messages.is_empty(), "{:?}", result.messages);
    let Some(OutputItem::Scene3D {
        data: Some(data),
        unavailable: None,
        ..
    }) = result.items.last()
    else {
        panic!("{result:?}")
    };
    data.clone()
}

#[test]
fn original_grass_block_with_definition_newlines_produces_identical_real_geometry() {
    let multiline = include_str!("../../../docs/acceptance/pre-alpha.4/fixtures/grass-block.om");
    let single = multiline.replace("=\n  ", "= ");
    let data = grass_geometry(multiline);
    let original = grass_geometry(&single);
    assert_eq!(data.meshes.len(), 12 * 12 * 5 + 1);
    assert_eq!(
        serde_json::to_value(&data).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    for mesh in &data.meshes {
        for position in &mesh.positions {
            assert!(
                position
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 0.5000000000001)
            );
        }
    }
}
