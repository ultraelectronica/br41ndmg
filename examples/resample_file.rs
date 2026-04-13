use br41ndmg::io::{read_wav, write_wav};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_else(|| "resample_file".into());
    let input_path = match args.next() {
        Some(value) => value,
        None => {
            eprintln!("usage: {program} <input.wav> <output.wav> <target_sample_rate>");
            std::process::exit(2);
        }
    };
    let output_path = match args.next() {
        Some(value) => value,
        None => {
            eprintln!("usage: {program} <input.wav> <output.wav> <target_sample_rate>");
            std::process::exit(2);
        }
    };
    let output_rate: u32 = match args.next() {
        Some(value) => value.parse()?,
        None => {
            eprintln!("usage: {program} <input.wav> <output.wav> <target_sample_rate>");
            std::process::exit(2);
        }
    };

    let input = read_wav(&input_path)?;
    let output = input.resample_to(output_rate)?;
    write_wav(&output_path, &output)?;

    println!(
        "resampled {} frames at {} Hz to {} frames at {} Hz",
        input.frame_count(),
        input.sample_rate(),
        output.frame_count(),
        output.sample_rate()
    );

    Ok(())
}
