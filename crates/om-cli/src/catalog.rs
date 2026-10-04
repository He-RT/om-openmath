//! REPL help reads the same executable descriptors as editor and capability queries.
use crate::host::Host;
use om_kernel::protocol::*;

impl Host {
    pub(crate) fn catalog_command(&mut self, command: &str, target: &str) -> Result<(), String> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| "Session state unavailable")?;
        if command == ":capabilities" {
            let (reply, _) = session.handle(Request::GetCapabilities {
                platform: HostPlatform::Cli,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&reply)
                    .map_err(|e| e.to_string())?
                    .replace('\n', if self.json { "" } else { "\n" })
            );
            return Ok(());
        }
        let Response::FunctionCatalog { catalog } = session.handle(Request::GetFunctionCatalog).0
        else {
            return Err("Function catalog unavailable".into());
        };
        if command == ":functions" {
            if self.json {
                println!(
                    "{}",
                    serde_json::to_string(&catalog).map_err(|e| e.to_string())?
                );
            } else {
                for f in catalog.functions {
                    println!("{}  {}  {}", f.modern_name, f.id, f.name);
                }
            }
            return Ok(());
        }
        let descriptor = catalog
            .functions
            .iter()
            .find(|f| {
                f.id == target
                    || f.name == target
                    || f.modern_name.eq_ignore_ascii_case(target)
                    || f.aliases.iter().any(|a| a.eq_ignore_ascii_case(target))
            })
            .ok_or_else(|| format!("没有当前可执行函数：{target}"))?;
        if command == ":options" {
            if self.json {
                println!(
                    "{}",
                    serde_json::to_string(&descriptor.options).map_err(|e| e.to_string())?
                );
            } else {
                for p in &descriptor.options {
                    println!(
                        "{}: {}",
                        p.name,
                        p.default_source
                            .as_deref()
                            .or(p.default_context.as_deref())
                            .unwrap_or("必填")
                    );
                }
            }
            return Ok(());
        }
        let Response::Hover {
            info: Some(documentation),
        } = session
            .handle(Request::Hover {
                source: format!("{}[]", descriptor.name),
                cursor: 0,
                dialect: Dialect::Wolfram,
            })
            .0
        else {
            return Err("Function documentation unavailable".into());
        };
        if self.json {
            println!(
                "{}",
                serde_json::json!({"descriptor":descriptor,"documentation":documentation})
            );
        } else {
            println!(
                "{} ({})\n{}\n{}",
                descriptor.modern_name,
                descriptor.id,
                documentation.signature.unwrap_or_default(),
                documentation.summary
            );
            for source in documentation.examples {
                println!("  {source}");
            }
        }
        Ok(())
    }
}
