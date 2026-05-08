//! Load optional text assets from the local `resources/` tree (or `TSW_RESOURCES`).
use std::path::PathBuf;

/// Root directory for shipped assets (`prelude/*.typ`, etc.).
///
/// Resolution order:
/// 1. `TSW_RESOURCES` — explicit directory (absolute or relative to the process cwd)
/// 2. `./resources` — relative to the current working directory
pub fn resource_root() -> PathBuf {
    std::env::var("TSW_RESOURCES")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("resources"))
}

/// Read `relative_path` under [`resource_root`]. On failure, returns `fallback` (typically
/// `include_str!` so the binary still works when no on-disk tree is present).
pub fn load_utf8(relative_path: &str, fallback: &'static str) -> String {
    let path = resource_root().join(relative_path);
    std::fs::read_to_string(&path).unwrap_or_else(|_| fallback.to_string())
}
