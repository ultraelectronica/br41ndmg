#!/usr/bin/env python3
"""Turn target/criterion/*/new/estimates.json into a baseline CSV."""
import csv, datetime, glob, json, os, subprocess

ELEMENTS = {
    "resample/mono_44100_to_48000": 44100,
    "resample/stereo_44100_to_48000": 88200,
    "resample/stereo_48000_to_44100": 88200,
    "resample/mono_48000_to_44100": 44100,
    "resample/stereo_44100_to_48000_phases_512_taps_95": 88200,
    "streaming/stereo_44100_to_48000_chunks_512": 88200,
}

def cpu_model():
    for line in open("/proc/cpuinfo"):
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return "unknown"

rows = []
for est_path in sorted(glob.glob("target/criterion/*/*/new/estimates.json")):
    bench = est_path.split("target/criterion/")[1].rsplit("/new/", 1)[0]
    est = json.load(open(est_path))
    mean, median = est["mean"], est["median"]
    rows.append({
        "benchmark": bench,
        "elements_per_iter": ELEMENTS.get(bench, ""),
        "mean_ns": round(mean["point_estimate"]),
        "median_ns": round(median["point_estimate"]),
        "mean_ci_lower_ns": round(mean["confidence_interval"]["lower_bound"]),
        "mean_ci_upper_ns": round(mean["confidence_interval"]["upper_bound"]),
        "std_dev_ns": round(est["std_dev"]["point_estimate"]),
    })

os.makedirs("benchmarks/baseline", exist_ok=True)
out = "benchmarks/baseline/x86_64_linux.csv"
with open(out, "w", newline="") as f:
    f.write(f"# generated {datetime.date.today().isoformat()} by benches/baseline_csv.py\n")
    f.write(f"# cpu: {cpu_model()}\n")
    f.write(f"# rustc: {subprocess.run(['rustc', '--version'], capture_output=True, text=True).stdout.strip()}\n")
    f.write("# command: cargo bench --bench resampler_bench\n")
    w = csv.DictWriter(f, fieldnames=list(rows[0]))
    w.writeheader()
    w.writerows(rows)

print(open(out).read())
