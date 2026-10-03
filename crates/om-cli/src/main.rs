//! OpenMath terminal frontend.
#![forbid(unsafe_code)]

mod args;
mod commands;
mod editor;
mod hints;
mod host;
mod llm;
mod render;
mod repl;
fn main() {
    let result = run();
    std::process::exit(match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    });
}
fn run() -> Result<i32, String> {
    let args = match args::parse(std::env::args().skip(1).collect()) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return Ok(2);
        }
    };
    if matches!(args.command, args::Command::Help) {
        println!(
            "OpenMath\nUsage: om [-e SOURCE] [--json] [--dialect modern|wolfram|auto] [--language en|zh-CN]\n       om run FILE | config path|show|edit | llm test PROFILE\n       --config PATH | --no-config"
        );
        return Ok(0);
    }
    if matches!(args.command, args::Command::Version) {
        println!("OpenMath {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    let mut host = host::Host::new(&args)?;
    let signal = host.signal.clone();
    ctrlc::set_handler(move || signal.stop()).map_err(|_| "Interrupt handler unavailable")?;
    match args.command {
        args::Command::Eval(source) => host.evaluate(source, false),
        args::Command::Run(path) => host.run_file(&path),
        args::Command::Config(action) => {
            host.config_command(&action)?;
            Ok(0)
        }
        args::Command::Test(profile) => {
            host.next += 1;
            let result = host.drive_llm(om_kernel::protocol::Request::LlmTestProfile {
                request_id: format!("cli-test-{}", host.next),
                profile,
                config: None,
            })?;
            Ok(i32::from(result.failed))
        }
        args::Command::Repl => repl::run(&mut host),
        _ => Ok(0),
    }
}
