use br41ndmg::io::{AudioBuffer, write_wav};
use std::f32::consts::PI;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tone_resampled.wav".into());

    let input_rate = 44_100;
    let output_rate = 48_000;
    let duration_seconds = 1.0;
    let frequency_hz = 440.0;
    let frame_count = (input_rate as f32 * duration_seconds) as usize;

    let samples = (0..frame_count)
        .map(|index| {
            let t = index as f32 / input_rate as f32;
            (2.0 * PI * frequency_hz * t).sin() * 0.5
        })
        .collect();

    let input = AudioBuffer::new(input_rate, 1, samples)?;
    let output = input.resample_to(output_rate)?;
    write_wav(&output_path, &output)?;

    println!(
        "wrote {} frames at {} Hz to {}",
        output.frame_count(),
        output.sample_rate(),
        output_path
    );

    Ok(())
}
