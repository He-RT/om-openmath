//! Dependency-free command-line parsing preserves source as one literal argument.
use om_kernel::{config::Language, protocol::Dialect};
use std::path::PathBuf;
#[derive(Clone)]
pub enum Command {
    Repl,
    Eval(String),
    Run(PathBuf),
    Config(String),
    Test(String),
    Export(crate::artifact::ExportArgs),
    Help,
    Version,
}
pub struct Args {
    pub command: Command,
    pub dialect: Option<Dialect>,
    pub language: Language,
    pub json: bool,
    pub no_config: bool,
    pub config: Option<PathBuf>,
}
pub fn parse(mut values: Vec<String>) -> Result<Args, String> {
    let mut args = Args {
        command: Command::Repl,
        dialect: None,
        language: Language::Auto,
        json: false,
        no_config: false,
        config: None,
    };
    let mut positional = vec![];
    let mut format = None;
    let mut output = None;
    let mut input = None;
    let mut width = 1000;
    let mut height = 600;
    let mut overwrite = false;
    let mut export_options = false;
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--format" | "-f" | "--output" | "-o" | "--input" | "--width" | "--height" => {
                let flag = values[i].clone();
                export_options = true;
                i += 1;
                let value = values
                    .get(i)
                    .ok_or_else(|| format!("Missing value for {flag}"))?
                    .clone();
                match flag.as_str() {
                    "--format" | "-f" => format = Some(value),
                    "--output" | "-o" => output = Some(PathBuf::from(value)),
                    "--input" => input = Some(PathBuf::from(value)),
                    "--width" => width = value.parse().map_err(|_| "Invalid export width")?,
                    _ => height = value.parse().map_err(|_| "Invalid export height")?,
                }
            }
            "--overwrite" => {
                overwrite = true;
                export_options = true;
            }
            "--json" => args.json = true,
            "--no-config" => args.no_config = true,
            "--help" | "-h" => args.command = Command::Help,
            "--version" | "-V" => args.command = Command::Version,
            "--dialect" | "--language" | "--config" | "-e" => {
                let flag = values[i].clone();
                i += 1;
                let value = values
                    .get(i)
                    .ok_or_else(|| format!("Missing value for {flag}"))?
                    .clone();
                match flag.as_str() {
                    "--dialect" => {
                        args.dialect = Some(match value.as_str() {
                            "modern" => Dialect::Modern,
                            "wolfram" => Dialect::Wolfram,
                            "auto" => Dialect::Auto,
                            _ => return Err("Unknown dialect".into()),
                        })
                    }
                    "--language" => {
                        args.language = match value.as_str() {
                            "en" => Language::En,
                            "zh-CN" => Language::ZhCn,
                            "auto" => Language::Auto,
                            _ => return Err("Unknown language".into()),
                        }
                    }
                    "--config" => args.config = Some(value.into()),
                    _ => args.command = Command::Eval(value),
                }
            }
            "--" => {
                positional.extend(values.drain(i + 1..));
                break;
            }
            value if value.starts_with('-') => return Err(format!("Unknown option: {value}")),
            _ => positional.push(values[i].clone()),
        }
        i += 1;
    }
    if !positional.is_empty() {
        args.command = match positional.as_slice() {
            [cmd] | [cmd, _] if cmd == "export" => {
                let source = if positional.len() == 2 {
                    Some(positional[1].clone())
                } else if let Command::Eval(source) = &args.command {
                    Some(source.clone())
                } else {
                    None
                };
                if source.is_some() == input.is_some() {
                    return Err("export需要唯一 -e SOURCE、SOURCE位置参数或--input FILE".into());
                }
                let format = format.take().ok_or("export需要--format svg|png|csv|json")?;
                if !["svg", "png", "csv", "json"].contains(&format.as_str()) {
                    return Err("Unsupported export format".into());
                }
                Command::Export(crate::artifact::ExportArgs {
                    source,
                    input: input.take(),
                    output: output.take().ok_or("export需要--output FILE")?,
                    format,
                    width,
                    height,
                    overwrite,
                })
            }
            [cmd, path] if cmd == "run" => Command::Run(path.into()),
            [cmd, action]
                if cmd == "config" && ["path", "show", "edit"].contains(&action.as_str()) =>
            {
                Command::Config(action.clone())
            }
            [cmd, action, name] if cmd == "llm" && action == "test" => Command::Test(name.clone()),
            _ => {
                return Err(
                    "Usage: om [-e SOURCE] | run FILE | config path|show|edit | llm test PROFILE"
                        .into(),
                );
            }
        };
    }
    if format.is_some()
        || output.is_some()
        || input.is_some()
        || export_options && !matches!(args.command, Command::Export(_))
    {
        return Err("Export options require the export command".into());
    }
    Ok(args)
}
