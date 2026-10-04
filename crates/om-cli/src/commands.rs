//! Literal file/config operations and genuine REPL meta commands.
use crate::{host::Host, render};
use om_kernel::{KernelConfig, protocol::*};
use std::{
    io::{self, Write},
    path::Path,
    process::Command,
};
pub enum Action {
    Continue,
    Quit,
    Edit(String),
}
impl Host {
    pub fn run_file(&mut self, path: &Path) -> Result<i32, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        if path.extension().is_some_and(|e| e == "omnb") {
            let file: NotebookFile =
                serde_json::from_str(&text).map_err(|_| "Invalid notebook file")?;
            if file.version != 1 {
                return Err("Unsupported notebook version".into());
            }
            let mut status = 0;
            for cell in file.cells {
                if cell.kind == CellKind::Math {
                    let dialect = self.dialect;
                    self.dialect = cell.dialect;
                    let result = self.evaluate(cell.source, true);
                    self.dialect = dialect;
                    status = status.max(result?);
                } else if !self.json {
                    println!("{}", cell.source);
                }
            }
            Ok(status)
        } else {
            self.evaluate(text, true)
        }
    }
    pub fn config_command(&self, action: &str) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("Configuration is disabled by --no-config")?;
        match action {
            "path" => println!("{}", path.display()),
            "show" => {
                let mut session = self
                    .session
                    .lock()
                    .map_err(|_| "Session state unavailable")?;
                let Response::Config { mut config } = session.handle(Request::GetConfig).0 else {
                    return Err("Configuration cannot be read".into());
                };
                config.general = self.stored_general.clone();
                for p in &mut config.llm.profiles {
                    for value in p.extra_headers.values_mut() {
                        *value = "***".into();
                    }
                    redact(&mut p.extra_body);
                }
                if self.json {
                    println!(
                        "{}",
                        serde_json::to_string(&config).map_err(|e| e.to_string())?
                    );
                } else {
                    println!(
                        "{}",
                        toml::to_string_pretty(&config).map_err(|e| e.to_string())?
                    );
                }
            }
            "edit" => {
                if !path.exists() {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    std::fs::write(
                        path,
                        toml::to_string_pretty(&KernelConfig::default())
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                }
                let editor = std::env::var("VISUAL")
                    .or_else(|_| std::env::var("EDITOR"))
                    .unwrap_or_else(|_| {
                        if cfg!(target_os = "windows") {
                            "notepad"
                        } else {
                            "vi"
                        }
                        .into()
                    });
                let parts = editor_args(&editor)?;
                let (program, args) = parts.split_first().ok_or("Editor command is empty")?;
                if !Command::new(program)
                    .args(args)
                    .arg(path)
                    .status()
                    .map_err(|e| e.to_string())?
                    .success()
                {
                    return Err("Editor failed".into());
                }
            }
            _ => return Err("Unknown configuration command".into()),
        }
        Ok(())
    }
    pub fn meta(&mut self, input: &str) -> Result<Action, String> {
        let input = input.trim();
        let (name, rest) = input.split_once(' ').unwrap_or((input, ""));
        match name {
            ":quit" | ":q" => return Ok(Action::Quit),
            ":help" if !rest.trim().is_empty() => self.catalog_command(name, rest.trim())?,
            ":functions" | ":options" | ":capabilities" => {
                self.catalog_command(name, rest.trim())?
            }
            ":help" => println!(
                "{}",
                render::text(
                    self.zh(),
                    ":help [函数] :functions :options 函数 :capabilities :steps :latex :dialect modern|wolfram|auto :clear :ask 问题 :explain :config :quit",
                    ":help [FUNCTION] :functions :options FUNCTION :capabilities :steps :latex :dialect modern|wolfram|auto :clear :ask QUESTION :explain :config :quit"
                )
            ),
            ":dialect" => {
                self.dialect = match rest.trim() {
                    "modern" => Dialect::Modern,
                    "wolfram" => Dialect::Wolfram,
                    "auto" => Dialect::Auto,
                    _ => return Err("Expected modern|wolfram|auto".into()),
                };
            }
            ":steps" => {
                let steps = self
                    .last
                    .as_ref()
                    .and_then(|(_, o)| {
                        o.items.iter().rev().find_map(|i| {
                            if let OutputItem::Solutions { steps: Some(s), .. } = i {
                                Some(s)
                            } else {
                                None
                            }
                        })
                    })
                    .ok_or("No recorded steps")?;
                render::steps(steps, self.zh());
            }
            ":latex" => {
                let (_, output) = self.last.as_ref().ok_or("No recorded output")?;
                for item in &output.items {
                    match item {
                        OutputItem::Expr { latex, .. } => println!("{latex}"),
                        OutputItem::Solutions { view, .. } => {
                            if let Some(region) = &view.region_latex {
                                println!("{region}");
                            }
                            for s in &view.solutions {
                                for b in &s.bindings {
                                    println!(
                                        "{} = {}",
                                        b.var_latex.as_deref().unwrap_or(&b.var),
                                        b.latex
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            ":clear" => {
                let names = {
                    let mut session = self
                        .session
                        .lock()
                        .map_err(|_| "Session state unavailable")?;
                    if let Response::Variables { items } = session.handle(Request::GetVariables).0 {
                        items.into_iter().map(|(name, _)| name).collect::<Vec<_>>()
                    } else {
                        vec![]
                    }
                };
                if !names.is_empty() {
                    let original = self.dialect;
                    self.dialect = Dialect::Wolfram;
                    let _ = self.evaluate(format!("Clear[{}];", names.join(",")), false)?;
                    self.dialect = original;
                }
            }
            ":config" => self.config_command(if rest.is_empty() { "show" } else { rest })?,
            ":explain" => {
                if self.confirm_ai("explain")? {
                    let _ = self.explain()?;
                }
            }
            ":ask" | "?" => {
                if rest.trim().is_empty() {
                    return Err("An AI question is required".into());
                }
                if !self.confirm_ai("ask")? {
                    return Ok(Action::Continue);
                }
                let answer = self.ask(rest.into())?;
                for suggestion in answer.suggestions {
                    if self.json {
                        println!(
                            "{}",
                            serde_json::to_string(&suggestion).map_err(|e| e.to_string())?
                        );
                    } else {
                        println!(
                            "{}\n{}",
                            if self.dialect == Dialect::Wolfram {
                                &suggestion.wolfram
                            } else {
                                &suggestion.modern
                            },
                            suggestion.explanation
                        );
                    }
                    eprint!(
                        "{} [Y/n/e] ",
                        render::text(self.zh(), "运行 / 取消 / 编辑", "Run / Cancel / Edit")
                    );
                    io::stderr().flush().map_err(|e| e.to_string())?;
                    let mut choice = String::new();
                    io::stdin()
                        .read_line(&mut choice)
                        .map_err(|e| e.to_string())?;
                    let source = if self.dialect == Dialect::Wolfram {
                        suggestion.wolfram
                    } else {
                        suggestion.modern
                    };
                    match choice.trim().to_ascii_lowercase().as_str() {
                        "y" => {
                            let _ = self.evaluate(source, true)?;
                        }
                        "e" => return Ok(Action::Edit(source)),
                        _ => {}
                    }
                }
            }
            _ => return Err("Unknown meta command; use :help".into()),
        }
        Ok(Action::Continue)
    }
}
fn redact(body: &mut std::collections::BTreeMap<String, serde_json::Value>) {
    fn value(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::String(s) => *s = "***".into(),
            serde_json::Value::Object(m) => {
                for v in m.values_mut() {
                    value(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    value(v);
                }
            }
            _ => {}
        }
    }
    for v in body.values_mut() {
        value(v);
    }
}
fn editor_args(text: &str) -> Result<Vec<String>, String> {
    let mut words = vec![];
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for c in text.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' && quote != Some('\'') {
            escaped = true;
        } else if quote == Some(c) {
            quote = None;
        } else if quote.is_none() && (c == '\'' || c == '"') {
            quote = Some(c);
        } else if quote.is_none() && c.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if quote.is_some() || escaped {
        return Err("Unclosed editor quoting".into());
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(words)
}
