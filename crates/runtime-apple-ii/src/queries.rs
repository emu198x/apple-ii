//! Family-owned facts used by the shared bounded-wait operations.
use crate::AppleIIRuntime;
use emu198x_shell::{QueryError, QueryResult, SessionQueryProvider};
use serde_json::json;
const PATHS: &[&str] = &[
    "bios.loaded",
    "boot.detected",
    "cpu.pc",
    "cpu.master_clock",
    "machine.frame_count",
    "machine.ram_bytes",
    "screen.text",
    "keyboard.pending",
    "disk.installed",
    "disk.motor_on",
    "disk.selected_drive",
    "disk.quarter_tracks",
    "disk.bit_reads",
    "disk.bit_writes",
];
/// Original-board query catalogue, shared by script and MCP sessions.
#[derive(Default, Clone, Copy)]
pub struct AppleIIQueries;
impl SessionQueryProvider<AppleIIRuntime> for AppleIIQueries {
    fn query_paths(&self, _m: &AppleIIRuntime, prefix: Option<&str>) -> Vec<String> {
        let mut paths: Vec<String> = PATHS
            .iter()
            .filter(|p| prefix.is_none_or(|prefix| p.starts_with(prefix)))
            .map(|p| p.to_string())
            .collect();
        paths.sort();
        paths
    }
    fn query(&self, m: &AppleIIRuntime, path: &str) -> Result<Option<QueryResult>, QueryError> {
        let status = m.machine.disk_ii().map(|c| c.status());
        let value = match path {
            "bios.loaded" => json!(true),
            "boot.detected" => json!(
                m.machine
                    .text_rows()
                    .iter()
                    .any(|s| s.trim_start().starts_with('*') || s.trim_start().starts_with('>'))
            ),
            "cpu.pc" => json!(m.machine.cpu().regs.pc),
            "cpu.master_clock" => json!(m.machine.master_clock()),
            "machine.frame_count" => json!(m.machine.frame_count()),
            "machine.ram_bytes" => json!(49152),
            "screen.text" => json!(m.machine.text_rows().join("\n")),
            "keyboard.pending" => json!(m.machine.key_pending() || !m.input.queue.is_empty()),
            "disk.installed" => json!(status.is_some()),
            "disk.motor_on" => json!(status.is_some_and(|s| s.motor_on)),
            "disk.selected_drive" => json!(status.map(|s| s.selected_drive)),
            "disk.quarter_tracks" => json!(status.map(|s| s.quarter_tracks)),
            "disk.bit_reads" => json!(status.map(|s| s.bit_reads)),
            "disk.bit_writes" => json!(status.map(|s| s.bit_writes)),
            _ => return Ok(None),
        };
        Ok(Some(QueryResult {
            path: path.into(),
            value,
        }))
    }
}
