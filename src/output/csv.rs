use crate::datasources::cpu_mem::ProcessInfo;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn write(path: impl AsRef<Path>, samples: &[ProcessInfo]) -> Result<(), String> {
    let file = File::create(path.as_ref()).map_err(|e| format!("cannot create CSV output: {e}"))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "timestamp_ms,pid,name,uptime_seconds,cpu_percent,memory_bytes,system_memory_bytes,virtual_memory_bytes,disk_read_bytes,disk_written_bytes,system_network_received_bytes,system_network_transmitted_bytes").map_err(|e| e.to_string())?;
    for sample in samples {
        let escaped_name = sample.name.replace('"', "\"\"");
        writeln!(
            writer,
            "{},{},\"{}\",{},{:.2},{},{},{},{},{},{},{}",
            sample.timestamp_ms,
            sample.pid,
            escaped_name,
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
    writer.flush().map_err(|e| e.to_string())
}
