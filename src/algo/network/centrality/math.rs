pub(super) fn square_root(value: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    #[cfg(feature = "std")]
    {
        value.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        let mut estimate = if value < 1.0 { 1.0 } else { value };
        for _ in 0..32 {
            estimate = 0.5 * (estimate + value / estimate);
        }
        estimate
    }
}
