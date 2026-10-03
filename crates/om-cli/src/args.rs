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
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
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
    Ok(args)
}
