//! `oci-arm-catcher` — grab Oracle Cloud's free ARM capacity the moment it appears.

/// `println!` that exits quietly when stdout is a closed pipe (`… | head -1`),
/// the way `cat` and `grep` do, instead of panicking. Declared before the modules so it
/// shadows the std macro everywhere in the binary.
macro_rules! println {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        if let Err(e) = writeln!(std::io::stdout(), $($arg)*) {
            crate::stdout_failed(e);
        }
    }};
}

/// A failed write to stdout: a closed pipe ends the program quietly with 141 (128 + SIGPIPE,
/// what a shell reports for a tool killed by SIGPIPE); anything else is a real error.
fn stdout_failed(e: std::io::Error) -> ! {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        std::process::exit(141);
    }
    panic!("failed printing to stdout: {e}");
}

mod args;
mod exit;
mod run;
mod settings;
mod ui;
mod update;

use std::process::ExitCode;

use std::time::Duration;

use clap::Parser as _;

use crate::args::Args;
use crate::update::UpdateCheck;

fn main() -> ExitCode {
    // clap exits with code 2 by itself on a parse error, matching our usage code.
    let args = Args::parse();
    let mut update = UpdateCheck::start();

    let code = match run::run(&args, &mut update) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui::report_error(&error);
            ExitCode::from(exit::code_for(&error))
        }
    };
    // A no-op if `run` already reported it.
    update.report(Duration::from_millis(1500));
    code
}
