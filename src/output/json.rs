//! JSON exporter for captured process metrics.
//!
//! Provides a seamless serialization pipeline leveraging `serde_json` to output
//! human-readable and machine-parseable JSON files from captured `ProcessInfo` samples.

use crate::datasources::cpu_mem::ProcessInfo;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Writes an array of `ProcessInfo` samples to a JSON file.
///
/// The output is formatted ("pretty-printed") with indentation for readability.
/// Under the hood, this routes the `Serialize` derivation on `ProcessInfo` through
/// a buffered writer directly to disk, keeping memory overhead low during serialization.
///
/// # Arguments
/// * `path` - The destination file path. Overwrites the file if it already exists.
/// * `samples` - A slice of `ProcessInfo` structs collected by the monitor.
pub fn write(path: impl AsRef<Path>, samples: &[ProcessInfo]) -> Result<(), String> {
    // Open destination file
    let file =
        File::create(path.as_ref()).map_err(|e| format!("cannot create JSON output: {e}"))?;

    // Serialize the structs into pretty-printed JSON directly to the buffered writer
    serde_json::to_writer_pretty(BufWriter::new(file), samples)
        .map_err(|e| format!("cannot write JSON output: {e}"))
}
