mod resample_job;
mod tui;

use resample_job::{resolve_output, run_batch, run_single};
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn usage(program: &str) -> ! {
    eprintln!("usage:");
    eprintln!("  {program}                       (interactive browser, starts in cwd)");
    eprintln!("  {program} <dir>                 (interactive browser, starts in <dir>)");
    eprintln!("  {program} -i|--interactive [dir]            (force interactive mode)");
    eprintln!("  {program} <input> <output.wav|out_dir> <rate>");
    eprintln!("  {program} <input_dir> <output_dir> <rate>   (batch)");
    eprintln!();
    eprintln!("Run with no arguments, or pass a directory, to open the interactive");
    eprintln!("file browser. Pass an input, an output target, and a sample rate for");
    eprintln!("the non-interactive path.");
    std::process::exit(2);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_else(|| "br41ndmg".into());
    let argv: Vec<String> = args.collect();

    // Explicit interactive flag: -i / --interactive [dir]
    if argv.iter().any(|a| a == "-i" || a == "--interactive") {
        let start = interactive_start(&argv);
        return tui::run(start).map_err(Into::into);
    }

    match argv.len() {
        // No args: launch browser in cwd.
        0 => tui::run(std::env::current_dir().unwrap_or_default()).map_err(Into::into),
        // Single arg: browse if it's a directory, otherwise usage error.
        1 if Path::new(&argv[0]).is_dir() => tui::run(PathBuf::from(&argv[0])).map_err(Into::into),
        // Three positional args: legacy resample path.
        3.. => {
            let input = &argv[0];
            let output_target = &argv[1];
            let output_rate = argv[2].parse::<u32>()?;
            if Path::new(input).is_dir() {
                run_batch(input, output_target, output_rate)?;
            } else {
                let out = resolve_output(input, output_target, output_rate)?;
                run_single(input, &out, output_rate)?;
            }
            Ok(())
        }
        _ => usage(&program),
    }
}

/// Starting directory for interactive mode: the first non-flag arg if it is a
/// directory, otherwise the current working directory.
fn interactive_start(argv: &[String]) -> PathBuf {
    argv.iter()
        .filter(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .find(|p| p.is_dir())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}
