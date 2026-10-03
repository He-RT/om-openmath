//! Real terminal REPL and deterministic stdin scripting share one evaluation path.
use crate::{
    commands::Action,
    editor::{Services, TerminalPrompt},
    host::Host,
};
use std::io::{self, IsTerminal};
pub fn run(host: &mut Host) -> Result<i32, String> {
    if !io::stdin().is_terminal() {
        return piped(host);
    }
    use reedline::MenuBuilder;
    let menu = Box::new(reedline::ColumnarMenu::default().with_name("completion_menu"));
    let mut keys = reedline::default_emacs_keybindings();
    keys.add_binding(
        reedline::KeyModifiers::NONE,
        reedline::KeyCode::Tab,
        reedline::ReedlineEvent::UntilFound(vec![
            reedline::ReedlineEvent::Menu("completion_menu".into()),
            reedline::ReedlineEvent::MenuNext,
        ]),
    );
    let mut editor =
        reedline::Reedline::create().with_hinter(Box::new(reedline::DefaultHinter::default()));
    editor = editor
        .with_menu(reedline::ReedlineMenu::EngineCompleter(menu))
        .with_edit_mode(Box::new(reedline::Emacs::new(keys)));
    if host.path.is_some()
        && let Some(dirs) = directories::ProjectDirs::from("org", "openmath", "OpenMath")
    {
        std::fs::create_dir_all(dirs.data_dir()).map_err(|e| e.to_string())?;
        let history =
            reedline::FileBackedHistory::with_file(10000, dirs.data_dir().join("history.txt"))
                .map_err(|e| e.to_string())?;
        editor = editor.with_history(Box::new(history));
    }
    if host
        .session
        .lock()
        .map(|s| s.config.cli.ai_hints)
        .unwrap_or(false)
        && host.confirm_ai("complete")?
    {
        editor = editor.with_hinter(Box::new(crate::hints::AiHinter::new(host)));
    }
    loop {
        if let Ok(mut session) = host.session.lock() {
            session.config.general.dialect = match host.dialect {
                om_kernel::protocol::Dialect::Modern => om_kernel::config::ConfigDialect::Modern,
                om_kernel::protocol::Dialect::Wolfram => om_kernel::config::ConfigDialect::Wolfram,
                om_kernel::protocol::Dialect::Auto => om_kernel::config::ConfigDialect::Auto,
            };
        }
        let services = Services {
            session: host.session.clone(),
            dialect: host.dialect,
        };
        editor = editor
            .with_completer(Box::new(services.clone()))
            .with_highlighter(Box::new(services.clone()))
            .with_validator(Box::new(services));
        match editor
            .read_line(&TerminalPrompt::from(host))
            .map_err(|e| e.to_string())?
        {
            reedline::Signal::Success(input) => match execute(host, input) {
                Ok(Action::Quit) => break,
                Ok(Action::Edit(code)) => {
                    editor.run_edit_commands(&[reedline::EditCommand::InsertString(code)])
                }
                Ok(Action::Continue) => {}
                Err(e) => eprintln!("{e}"),
            },
            reedline::Signal::CtrlC => {}
            reedline::Signal::CtrlD => break,
            _ => break,
        }
    }
    editor.sync_history().map_err(|e| e.to_string())?;
    Ok(0)
}
fn execute(host: &mut Host, input: String) -> Result<Action, String> {
    if input.trim().is_empty() {
        return Ok(Action::Continue);
    }
    if input.trim_start().starts_with(':') || input.trim_start().starts_with('?') {
        host.meta(&input)
    } else {
        let _ = host.evaluate(input, true)?;
        Ok(Action::Continue)
    }
}
fn piped(host: &mut Host) -> Result<i32, String> {
    loop {
        let mut input = String::new();
        if io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        match execute(host, input) {
            Ok(Action::Quit) => break,
            Ok(Action::Edit(code)) => eprintln!(
                "{}: {code}",
                crate::render::text(
                    host.zh(),
                    "可编辑源码（尚未运行）",
                    "Editable source (not run)"
                )
            ),
            Ok(Action::Continue) => {}
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(0)
}
