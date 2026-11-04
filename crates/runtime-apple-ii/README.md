# Apple II shared runtime

`AppleIIRuntime::from_firmware` builds the original NTSC revision-0 board with
48 KB RAM from a shared-shell `FirmwareSet`. Required IDs: `apple-ii-rom`
(8192 bytes), `apple-ii-characters` (512 bytes). Optional `disk-ii-p5` and
`disk-ii-p6` (256 bytes each) must be supplied together.

Implements shared MachineCore, DebugTarget and KeyboardTarget contracts for
JSON scripts and stdio MCP through `emu198x-shell`. `AppleIIQueries` exposes
screen text, CPU/crystal/frame state, keyboard backpressure and Disk II state.
Execution follows actual oscillator edges and native video boundaries. Debug
writes affect RAM only. RESET preserves machine time, RAM and mounted disks.

WOZ disks mount in `floppy-0` / `floppy-1`, read-only by default. Opt into a
writable in-memory work copy and export to a new host path through the shell.
Replacing, ejecting or exporting while the Disk II motor runs fails. Source
paths are never written. Save states and native windows are not implemented.
No firmware ships with this crate. See repository verification for external
Monitor, Integer BASIC and DOS fixtures and qualification limits.
