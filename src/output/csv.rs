//! CSV exporter for captured process metrics.
//!
//! Provides a highly efficient, buffered exporter for saving the `ProcessInfo`
//! structs into a standard CSV format suitable for analysis in spreadsheets
//! or data processing tools (like Pandas or R).

use crate::datasources::cpu_mem::ProcessInfo;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Writes an array of `ProcessInfo` samples to a CSV file.
///
/// This implementation uses a `BufWriter` to minimize syscalls and optimize disk I/O,
/// and handles proper CSV character escaping for string fields (e.g., process names with spaces or quotes).
///
/// # Arguments
/// * `path` - The destination file path. Overwrites the file if it already exists.
/// * `samples` - A slice of `ProcessInfo` structs collected by the monitor.
pub fn write(path: impl AsRef<Path>, samples: &[ProcessInfo]) -> Result<(), String> {
    // Open destination file and wrap it in a buffered writer
    let file = File::create(path.as_ref()).map_err(|e| format!("cannot create CSV output: {e}"))?;
    let mut writer = BufWriter::new(file);
    
    // Write CSV Header
    writeln!(writer, "timestamp_ms,pid,name,executable_path,thread_count,uptime_seconds,cpu_percent,memory_bytes,system_memory_bytes,virtual_memory_bytes,disk_read_bytes,disk_written_bytes,system_network_received_bytes,system_network_transmitted_bytes").map_err(|e| e.to_string())?;
    
    // Iterate over samples and serialize each row
    for sample in samples {
        // According to CSV spec, double quotes within fields must be escaped by doubling them
        let escaped_name = sample.name.replace('"', "\"\"");
        let escaped_executable_path = sample
            .executable_path
            .as_deref()
            .unwrap_or_default()
            .replace('"', "\"\"");
            
        writeln!(
            writer,
            "{},{},\"{}\",\"{}\",{},{},{:.2},{},{},{},{},{},{},{}",
            sample.timestamp_ms,
            sample.pid,
            escaped_name, // Wrapped in quotes in format string below
            escaped_executable_path, // Wrapped in quotes
            sample
                .thread_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            sample.uptime_seconds,
            sample.cpu_percent,
            sample.memory_bytes,
            sample.system_memory_bytes,
            sample.virtual_memory_bytes,
            sample.disk_read_bytes,
            sample.disk_written_bytes,
            sample.network_received_bytes,
            sample.network_transmitted_bytes
        )
        .map_err(|e| e.to_string())?;
    }
    
    // Ensure all internal buffers are written out to the OS filesystem
    writer.flush().map_err(|e| e.to_string())
}
