use br41ndmg::io::{read_audio, write_wav};
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn usage(program: &str) -> ! {
    eprintln!("usage:");
    eprintln!("  {program} <input.wav|input.flac> <output.wav|output_dir> <target_sample_rate>");
    eprintln!("  {program} <input_dir> <output_dir> <target_sample_rate>   (batch)");
    eprintln!();
    eprintln!("Output naming:");
    eprintln!("  - If <output> ends in .wav and is not an existing directory,");
    eprintln!("    it is used verbatim as the output file path.");
    eprintln!("  - Otherwise <output> is treated as a directory (created if missing)");
    eprintln!("    and the file is auto-named <original-stem>_<rate>Hz.wav.");
    eprintln!("If <input> is a directory, every .wav/.flac inside it is resampled");
    eprintln!("into <output_dir> using the same naming rule.");
    std::process::exit(2);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_else(|| "br41ndmg".into());
    let (input, output_target, output_rate) = match (args.next(), args.next(), args.next()) {
        (Some(i), Some(o), Some(r)) => (i, o, r.parse::<u32>()?),
        _ => usage(&program),
    };

    if Path::new(&input).is_dir() {
        run_batch(&input, &output_target, output_rate)?;
    } else {
        let out = resolve_output(&input, &output_target, output_rate)?;
        resample_one(&input, &out, output_rate)?;
    }
    Ok(())
}

fn has_wav_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
}

fn auto_output_name(input: &Path, rate: u32) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    PathBuf::from(format!("{stem}_{rate}Hz.wav"))
}

fn resolve_output(input: &str, target: &str, rate: u32) -> Result<PathBuf, String> {
    let target_path = Path::new(target);
    if has_wav_extension(target_path) && !target_path.is_dir() {
        return Ok(target_path.to_path_buf());
    }
    std::fs::create_dir_all(target_path).map_err(|e| format!("create {}: {e}", target))?;
    Ok(target_path.join(auto_output_name(Path::new(input), rate)))
}

fn resample_one(input: &str, output: &Path, rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    let buf = read_audio(input)?;
    let result = buf.resample_to(rate)?;
    write_wav(output, &result)?;
    println!(
        "{} -> {}: {} frames @ {} Hz to {} frames @ {} Hz",
        input,
        output.display(),
        buf.frame_count(),
        buf.sample_rate(),
        result.frame_count(),
        result.sample_rate()
    );
    Ok(())
}

fn run_batch(
    input_dir: &str,
    output_dir: &str,
    rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new(output_dir);
    std::fs::create_dir_all(out_dir)?;

    let mut entries: Vec<PathBuf> = std::fs::read_dir(input_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase())
                    .as_deref(),
                Some("wav") | Some("flac")
            )
        })
        .collect();
    entries.sort();

    if entries.is_empty() {
        eprintln!("no .wav/.flac files in {input_dir}");
        return Ok(());
    }

    let mut failures = 0usize;
    for entry in &entries {
        let out = out_dir.join(auto_output_name(entry, rate));
        if let Err(error) = resample_one(&entry.to_string_lossy(), &out, rate) {
            eprintln!("failed {}: {error}", entry.display());
            failures += 1;
        }
    }
    println!(
        "batch done: {} ok, {} failed ({} Hz -> {output_dir})",
        entries.len() - failures,
        failures,
        rate
    );
    if failures > 0 {
        return Err(format!("{failures} file(s) failed").into());
    }
    Ok(())
}
