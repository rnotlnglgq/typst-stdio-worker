/// Format a byte count using binary prefixes (KiB, MiB, …).
pub(crate) fn format_u64(n: u64) -> String {
    const UNIT: f64 = 1024.0;
    if n < 1024 {
        return format!("{} B", n);
    }
    let mut v = n as f64;
    let labels = ["KiB", "MiB", "GiB", "TiB"];
    let mut divisions = 0usize;
    while v >= UNIT && divisions < labels.len() {
        v /= UNIT;
        divisions += 1;
    }
    format!("{:.2} {}", v, labels[divisions - 1])
}