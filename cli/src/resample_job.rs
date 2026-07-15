use br41ndmg::io::{read_audio, write_wav};
use std::path::{Path, PathBuf};

pub fn is_audio_extension(ext: &str) -> bool {
    matches!(ext.to_ascii_lowercase().as_str(), "wav" | "flac")
}

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(is_audio_extension)
}

fn has_wav_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
}

pub fn auto_output_name(input: &Path, rate: u32) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    PathBuf::from(format!("{stem}_{rate}Hz.wav"))
}

/// Resolve a legacy CLI output target: an explicit `.wav` path wins, otherwise
/// `target` is treated as a directory (created if missing).
pub fn resolve_output(input: &str, target: &str, rate: u32) -> Result<PathBuf, String> {
    let target_path = Path::new(target);
    if has_wav_extension(target_path) && !target_path.is_dir() {
        return Ok(target_path.to_path_buf());
    }
    std::fs::create_dir_all(target_path).map_err(|e| format!("create {}: {e}", target))?;
    Ok(target_path.join(auto_output_name(Path::new(input), rate)))
}

/// Sorted list of audio files directly inside `dir` (non-recursive).
pub fn list_audio_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_audio_file(p))
        .collect();
    entries.sort();
    Ok(entries)
}

/// Read, resample, and write a single file. Performs no printing so callers
/// (CLI or TUI) own their own reporting.
pub fn resample_file(
    input: &Path,
    output: &Path,
    rate: u32,
) -> Result<(), br41ndmg::ResampleError> {
    let buf = read_audio(input)?;
    let result = buf.resample_to(rate)?;
    write_wav(output, &result)
}

/// Outcome of processing a batch of inputs.
pub struct BatchOutcome {
    pub ok: usize,
    pub failed: Vec<(PathBuf, String)>,
}

impl BatchOutcome {
    pub fn is_ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Resample `inputs` into `output_dir` (created if missing), invoking
/// `on_progress(index, total, current_path)` before each file.
pub fn process_files<F: FnMut(usize, usize, &Path)>(
    inputs: &[PathBuf],
    output_dir: &Path,
    rate: u32,
    mut on_progress: F,
) -> BatchOutcome {
    let _ = std::fs::create_dir_all(output_dir);
    let total = inputs.len();
    let mut failed = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        on_progress(index, total, input);
        let out = output_dir.join(auto_output_name(input, rate));
        if let Err(error) = resample_file(input, &out, rate) {
            failed.push((input.clone(), error.to_string()));
        }
    }
    BatchOutcome {
        ok: total - failed.len(),
        failed,
    }
}

/// Legacy CLI single-file path: resample and print a one-line summary.
pub fn run_single(input: &str, output: &Path, rate: u32) -> Result<(), Box<dyn std::error::Error>> {
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

/// Legacy CLI batch path: resample every audio file in `input_dir`, print a
/// summary, and return an error if anything failed.
pub fn run_batch(
    input_dir: &str,
    output_dir: &str,
    rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new(output_dir);
    std::fs::create_dir_all(out_dir)?;

    let entries = list_audio_files(Path::new(input_dir))?;
    if entries.is_empty() {
        eprintln!("no .wav/.flac files in {input_dir}");
        return Ok(());
    }

    let outcome = process_files(&entries, out_dir, rate, |index, total, path| {
        println!("[{}/{}] {}", index + 1, total, path.display());
    });

    for (path, error) in &outcome.failed {
        eprintln!("failed {}: {error}", path.display());
    }
    println!(
        "batch done: {} ok, {} failed ({} Hz -> {output_dir})",
        outcome.ok,
        outcome.failed.len(),
        rate
    );

    if !outcome.is_ok() {
        return Err(format!("{} file(s) failed", outcome.failed.len()).into());
    }
    Ok(())
}
