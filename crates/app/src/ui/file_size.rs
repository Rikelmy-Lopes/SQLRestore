//! Backup file size reading and formatting.

use std::fs;
use std::path::Path;

const BYTES_PER_MB: u64 = 1024 * 1024;
const BYTES_PER_GB: u64 = 1024 * BYTES_PER_MB;

/// Reads the size of an existing file. Returns `None` when it cannot be read.
pub fn file_size(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then_some(metadata.len())
}

/// Formats a byte count as MB or GB.
pub fn format_file_size(bytes: u64) -> String {
    if bytes >= BYTES_PER_GB {
        format!("{:.2} GB", bytes as f64 / BYTES_PER_GB as f64)
    } else {
        format!("{:.2} MB", bytes as f64 / BYTES_PER_MB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_small_sizes_in_megabytes() {
        assert_eq!(format_file_size(0), "0.00 MB");
        assert_eq!(format_file_size(BYTES_PER_MB / 2), "0.50 MB");
        assert_eq!(format_file_size(BYTES_PER_MB), "1.00 MB");
        assert_eq!(format_file_size(BYTES_PER_GB - 1), "1024.00 MB");
    }

    #[test]
    fn formats_large_sizes_in_gigabytes() {
        assert_eq!(format_file_size(BYTES_PER_GB), "1.00 GB");
        assert_eq!(format_file_size(BYTES_PER_GB + BYTES_PER_GB / 2), "1.50 GB");
    }
}
