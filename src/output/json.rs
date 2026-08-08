use crate::datasources::cpu_mem::ProcessInfo;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn write(path: impl AsRef<Path>, samples: &[ProcessInfo]) -> Result<(), String> {
    let file =
        File::create(path.as_ref()).map_err(|e| format!("cannot create JSON output: {e}"))?;
    serde_json::to_writer_pretty(BufWriter::new(file), samples)
        .map_err(|e| format!("cannot write JSON output: {e}"))
}
