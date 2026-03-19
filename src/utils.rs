pub fn validate_cutoff(cutoff: f64) {
    assert!(
        cutoff.is_finite() && cutoff > 0.0 && cutoff <= 0.5,
        "cutoff must be in (0, 0.5]"
    );
}
