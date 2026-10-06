//! Actual sampled/retained data exports, independent chunk/pixel checks and no CAS reevaluation.
use om_core::Interrupt;
use om_kernel::{Session, artifact, protocol::*};
fn evaluate(s: &mut Session, id: &str, source: &str) -> CellOutput {
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!()
    };
    assert!(output.messages.is_empty(), "{output:?}");
    output
}
fn figure(s: &mut Session, source: &str) -> PlotFigure {
    let o = evaluate(s, "plot", source);
    let OutputItem::Plot { request, data } = &o.items[0] else {
        panic!("{o:?}")
    };
    PlotFigure {
        data: data.clone(),
        axis_x: request.var_x.clone(),
        axis_y: request.var_y.clone().unwrap_or_else(|| "y".into()),
        title: "地月 L2 α".into(),
        color: None,
        region: request.kind == PlotKind::Region,
        parameters: Default::default(),
        width: 1000,
        height: 600,
    }
}
fn bytes(response: Response) -> Vec<u8> {
    let Response::Artifact { artifact } = response else {
        panic!("{response:?}")
    };
    artifact::decode(&artifact).unwrap()
}
fn query(o: &CellOutput, id: &str) -> ValueQuery {
    let OutputItem::Expr {
        out_index,
        presentation: Some(p),
        ..
    } = &o.items[0]
    else {
        panic!("{o:?}")
    };
    ValueQuery {
        cell_id: id.into(),
        out_index: *out_index,
        view_id: p.view_id.clone(),
        path: vec![],
        offset: 32,
        limit: 32,
        column_offset: 8,
        column_limit: 8,
        include_source: false,
    }
}
#[test]
fn svg_keeps_real_domains_colors_values_metadata_and_escaped_text() {
    let mut s = Session::new(Default::default(), None);
    let mut f = figure(&mut s, "plot(x^2,x:1..1000,scale:\"log_log\")");
    f.title = "地月 & <script>不能执行</script> α".into();
    f.axis_x = "α".into();
    f.parameters.insert("a".into(), 4.);
    let expected = serde_json::to_value(&f.data).unwrap();
    let text = String::from_utf8(bytes(
        s.handle(Request::ExportPlot {
            figure: f,
            format: PlotExportFormat::Svg,
        })
        .0,
    ))
    .unwrap();
    assert!(text.contains("&lt;script&gt;") && !text.contains("<script>"));
    assert!(text.contains("log_log"));
    assert!(text.contains("1000000.0"));
    assert!(text.contains("地月"));
    assert!(text.contains("α"));
    let m = text
        .split("<metadata>")
        .nth(1)
        .unwrap()
        .split("</metadata>")
        .next()
        .unwrap()
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    let value: serde_json::Value = serde_json::from_str(&m).unwrap();
    assert_eq!(value["figure"]["data"], expected);
    assert_eq!(value["figure"]["parameters"]["a"], 4.);
}
#[test]
fn png_is_lossless_rgba_with_real_unicode_metadata_fonts_and_expected_curve_pixels() {
    let mut s = Session::new(Default::default(), None);
    let f = figure(&mut s, "plot(0,x:-1..1,plot_range:[-1,1])");
    let png = bytes(
        s.handle(Request::ExportPlot {
            figure: f,
            format: PlotExportFormat::Png,
        })
        .0,
    );
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let mut at = 8;
    let mut z = vec![];
    let mut text = String::new();
    let mut dimensions = (0, 0);
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffff_u32;
        for b in data {
            c ^= u32::from(*b);
            for _ in 0..8 {
                let mask = 0_u32.wrapping_sub(c & 1);
                c = (c >> 1) ^ (0xedb88320 & mask);
            }
        }
        c ^ 0xffff_ffff
    }
    while at < png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let kind = &png[at + 4..at + 8];
        let data = &png[at + 8..at + 8 + len];
        assert_eq!(
            crc(&png[at + 4..at + 8 + len]),
            u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().unwrap())
        );
        match kind {
            b"IHDR" => {
                dimensions = (
                    u32::from_be_bytes(data[..4].try_into().unwrap()),
                    u32::from_be_bytes(data[4..8].try_into().unwrap()),
                );
                assert_eq!(&data[8..], [8, 6, 0, 0, 0]);
            }
            b"IDAT" => z.extend_from_slice(data),
            b"iTXt" => text = String::from_utf8(data[13..].to_vec()).unwrap(),
            _ => {}
        }
        at += len + 12;
    }
    assert_eq!(dimensions, (1000, 600));
    assert!(text.contains("地月 L2 α"));
    let metadata: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        metadata["figure"]["data"]["curves"][0]["segments"][0][0][1],
        0.
    );
    let mut raw = vec![];
    let mut at = 2;
    loop {
        let flag = z[at];
        assert_eq!(flag & 6, 0);
        let n = u16::from_le_bytes(z[at + 1..at + 3].try_into().unwrap());
        assert_eq!(
            n ^ u16::from_le_bytes(z[at + 3..at + 5].try_into().unwrap()),
            65535
        );
        raw.extend_from_slice(&z[at + 5..at + 5 + n as usize]);
        at += 5 + n as usize;
        if flag & 1 == 1 {
            break;
        }
    }
    let mut a = 1_u32;
    let mut b = 0_u32;
    for x in &raw {
        a = (a + u32::from(*x)) % 65521;
        b = (b + a) % 65521;
    }
    assert_eq!(
        (b << 16) | a,
        u32::from_be_bytes(z[at..at + 4].try_into().unwrap())
    );
    assert_eq!(raw.len(), 600 * (4000 + 1));
    assert!(raw.chunks(4001).all(|row| row[0] == 0));
    let pixel = |x: usize, y: usize| &raw[y * 4001 + 1 + x * 4..y * 4001 + 1 + x * 4 + 4];
    assert_eq!(pixel(300, 303), &[33, 133, 74, 255]);
    assert_eq!(pixel(320, 310), &[255, 255, 255, 255]);
    assert!(
        raw[15 * 4001..35 * 4001]
            .iter()
            .filter(|b| **b < 200)
            .count()
            > 100
    ); // Actual rasterized Chinese/Greek title, not empty labels.
}
#[test]
fn full_table_and_nested_value_exports_ignore_page_offsets_preserve_fields_and_do_not_execute_text()
{
    let mut s = Session::new(Default::default(), None);
    let mut csv = "x,说明\n".to_string();
    for i in 0..40 {
        csv.push_str(&format!("{i},中文🙂 {i}\n"));
    }
    let source = format!("parse_csv({})", serde_json::to_string(&csv).unwrap());
    let o = evaluate(&mut s, "data", &source);
    let q = query(&o, "data");
    let text = String::from_utf8(bytes(
        s.handle(Request::ExportValue {
            query: q.clone(),
            format: DataExportFormat::Csv,
        })
        .0,
    ))
    .unwrap();
    assert_eq!(text, csv.replace("\n", "\r\n"));
    let json = String::from_utf8(bytes(
        s.handle(Request::ExportValue {
            query: q.clone(),
            format: DataExportFormat::Json,
        })
        .0,
    ))
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["rows"].as_array().unwrap().len(), 40);
    assert_eq!(value["rows"][39]["说明"], "中文🙂 39");
    let o = evaluate(
        &mut s,
        "record",
        "{table:[[1,2],[3,4]],text:\"assign(leak,9)\"}",
    );
    let mut nested = query(&o, "record");
    nested.path = vec![0, 1];
    assert_eq!(
        String::from_utf8(bytes(
            s.handle(Request::ExportValue {
                query: nested,
                format: DataExportFormat::Csv
            })
            .0
        ))
        .unwrap(),
        "1,2\r\n3,4\r\n"
    );
    evaluate(&mut s, "data", "[99,100]");
    assert!(matches!(
        s.handle(Request::ExportValue {
            query: q,
            format: DataExportFormat::Csv
        })
        .0,
        Response::Error { .. }
    ));
    let o = evaluate(&mut s, "probe", "leak");
    assert!(matches!(&o.items[0],OutputItem::Expr{input_form,..}if input_form=="leak"));
}
#[test]
fn ephemeral_value_tokens_export_the_actual_selected_parameters_without_rerunning_source() {
    let mut s = Session::new(Default::default(), None);
    let o = evaluate(
        &mut s,
        "e",
        "explore([[a,a^2]],controls:{a:0..4},initial:{a:3})",
    );
    let OutputItem::Explore { result, .. } = &o.items[0] else {
        panic!()
    };
    let token = result.value_token.clone().unwrap();
    let csv = String::from_utf8(bytes(
        s.handle(Request::ExportValueToken {
            token: token.clone(),
            format: DataExportFormat::Csv,
        })
        .0,
    ))
    .unwrap();
    assert_eq!(csv, "3,9\r\n");
    let json = String::from_utf8(bytes(
        s.handle(Request::ExportValueToken {
            token,
            format: DataExportFormat::Json,
        })
        .0,
    ))
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap(),
        serde_json::json!([[3, 9]])
    );
}
#[test]
fn forged_geometry_formats_resources_non_data_and_interrupts_return_failures() {
    let mut s = Session::new(Default::default(), None);
    let mut f = figure(&mut s, "plot(x,x:0..1)");
    f.width = 999999;
    assert!(matches!(
        s.handle(Request::ExportPlot {
            figure: f,
            format: PlotExportFormat::Svg
        })
        .0,
        Response::Error { .. }
    ));
    let mut f = figure(&mut s, "plot(x,x:0..1)");
    f.data.curves[0].segments[0][0].0 = f64::NAN;
    assert!(matches!(
        s.handle(Request::ExportPlot {
            figure: f,
            format: PlotExportFormat::Svg
        })
        .0,
        Response::Error { .. }
    ));
    let mut f = figure(&mut s, "plot(x,x:0..1)");
    f.color = Some("red\"/><script>bad</script>".into());
    assert!(matches!(
        s.handle(Request::ExportPlot {
            figure: f,
            format: PlotExportFormat::Svg
        })
        .0,
        Response::Error { .. }
    ));
    let f = figure(&mut s, "plot(x,x:0..1)");
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        artifact::export_plot(&f, PlotExportFormat::Png, &ctx),
        Err(artifact::ArtifactError::Abort(_))
    ));
    let o = evaluate(&mut s, "symbolic", "[hold(assign(leak,9))]");
    let q = query(&o, "symbolic");
    assert!(matches!(
        s.handle(Request::ExportValue {
            query: q,
            format: DataExportFormat::Json
        })
        .0,
        Response::Error { .. }
    ));
    let a = artifact::encode(&[0, 1, 254, 255], "binary", "bin", &Interrupt::default()).unwrap();
    assert_eq!(a.base64, "AAH+/w==");
    assert_eq!(artifact::decode(&a).unwrap(), [0, 1, 254, 255]);
}
