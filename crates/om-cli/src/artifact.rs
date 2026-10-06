//! Noninteractive artifact export uses real retained values or sampled geometry and verifies persisted bytes.
use crate::host::Host;
use om_kernel::{artifact, protocol::*};
use std::{io::Write, path::PathBuf, sync::atomic::Ordering};
#[derive(Clone)]
pub struct ExportArgs {
    pub source: Option<String>,
    pub input: Option<PathBuf>,
    pub output: PathBuf,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub overwrite: bool,
}
impl Host {
    pub fn export_artifact(&mut self, args: ExportArgs) -> Result<i32, String> {
        if args.output.exists() && !args.overwrite {
            return Err("导出目标已存在；使用 --overwrite 明确覆盖".into());
        }
        if let Some(input) = &args.input {
            let code = self.run_file(input)?;
            if code != 0 {
                return Ok(code);
            }
        } else if let Some(source) = args.source {
            let code = self.evaluate(source, false)?;
            if code != 0 {
                return Ok(code);
            }
        }
        let (id, output) = self.last.as_ref().ok_or("没有成功的导出结果")?;
        let item = output.items.last().ok_or("没有可导出的输出")?;
        let item = if let OutputItem::Explore { result, .. } = item {
            result.item.as_ref()
        } else {
            item
        };
        let request = match args.format.as_str() {
            "svg" | "png" => {
                let OutputItem::Plot { request, data } = item else {
                    return Err("SVG/PNG需要真实二维绘图输出".into());
                };
                let mut parameters = request.params.clone();
                if let Some(OutputItem::Explore { result, .. }) = output.items.last() {
                    parameters.extend(result.values.clone());
                }
                Request::ExportPlot {
                    format: if args.format == "svg" {
                        PlotExportFormat::Svg
                    } else {
                        PlotExportFormat::Png
                    },
                    figure: PlotFigure {
                        data: data.clone(),
                        axis_x: if request.kind == PlotKind::Parametric {
                            "x".into()
                        } else {
                            request.var_x.clone()
                        },
                        axis_y: request.var_y.clone().unwrap_or_else(|| "y".into()),
                        title: "OpenMath".into(),
                        color: request.options.as_ref().and_then(|o| o.color.clone()),
                        region: request.kind == PlotKind::Region,
                        parameters,
                        width: args.width,
                        height: args.height,
                    },
                }
            }
            "csv" | "json" => {
                let format = if args.format == "csv" {
                    DataExportFormat::Csv
                } else {
                    DataExportFormat::Json
                };
                if let Some(OutputItem::Explore { result, .. }) = output.items.last() {
                    Request::ExportValueToken {
                        token: result.value_token.clone().ok_or("探索结果不是可导出数据")?,
                        format,
                    }
                } else {
                    let OutputItem::Expr {
                        out_index,
                        presentation: Some(page),
                        ..
                    } = item
                    else {
                        return Err("CSV/JSON需要实际保留的表格、列表、矩阵或记录".into());
                    };
                    Request::ExportValue {
                        query: ValueQuery {
                            cell_id: id.clone(),
                            out_index: *out_index,
                            view_id: page.view_id.clone(),
                            path: vec![],
                            offset: 0,
                            limit: 32,
                            column_offset: 0,
                            column_limit: 8,
                            include_source: false,
                        },
                        format,
                    }
                }
            }
            _ => return Err("导出格式支持svg/png/csv/json；OBJ在三维阶段接入".into()),
        };
        self.signal.evaluating.store(true, Ordering::Relaxed);
        let response = self
            .session
            .lock()
            .map_err(|_| "内核状态不可用")?
            .handle(request)
            .0;
        self.signal.evaluating.store(false, Ordering::Relaxed);
        let Response::Artifact { artifact } = response else {
            if let Response::Error { message } = response {
                return Err(message);
            }
            return Err("内核没有返回导出字节".into());
        };
        let bytes = artifact::decode(&artifact).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(args.overwrite)
            .create_new(!args.overwrite)
            .open(&args.output)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        if std::fs::read(&args.output).map_err(|e| e.to_string())? != bytes {
            return Err("导出文件回读校验失败".into());
        }
        let path = args.output.canonicalize().map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::json!({"artifact":{"path":path,"format":artifact.extension,"byte_len":artifact.byte_len},"persisted":true})
        );
        Ok(0)
    }
}
