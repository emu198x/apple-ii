# Apple II shared scripts and MCP

Engineering-frontier work. Steve approved the shared-shell design, scripts and
MCP together, and the published emu198x-shell / serde_json dependencies. He also
confirmed this as the default for future machines on 4 November 2026.

1. Record that default in flagship knowledge/decisions/tools-follow-the-machine-spec.md.
2. Add RAM-only debugger writes and observable instruction boundaries in
   machine-apple-ii-rev0-ntsc/src/lib.rs. Preserve oscillator timing, ROM/I/O
   immunity and debugger-safe peeks. Test CPU-driven stepping, parked/halted
   bounds and memory writes in tests/control.rs.
3. Add crates/runtime-apple-ii as emu198x-runtime-apple-ii, using published
   emu198x-shell and serde_json. Split runtime.rs, input.rs, queries.rs,
   profiles.rs and debug.rs. Actual ticks/fields drive execution, audio and
   captures; reset preserves machine RAM/media. Shared keyboard and debug
   targets enable the existing shared script/MCP executor. Queries expose text,
   PC, clock, keyboard and Disk II state. Mount/eject/export WOZ through the
   existing immutable-source adapters. Reject busy replacements/exports and
   unsupported save states explicitly; do not advertise unwired capabilities.
4. Add integration tests for exact tick/frame accounting, input, reset,
   query enumeration, debug progress/bounds, media protection and script verbs.
   Keep the existing machine 95% workspace coverage floor. Run fmt, Clippy,
   tests, coverage and external ROM/DOS script workloads. Prove negative waits
   and missing firmware fail. Publish the checked runtime for cross-repo use.
5. Add flagship crates/emu198x-apple-ii with the standard main_headless launcher,
   published runtime dependency, explicit ROM/glyph/P5/P6 flags and shared
   --script/--mcp modes. Use the same registered tools and runtime for both.
   Tests exercise the actual CLI and stdio MCP end to end: type BASIC, wait
   for text, inspect/debug, capture PNG and mount/export DOS work disks.
   Record commands, exact fixtures and results; keep ROMs external.
6. Update README/AGENTS/verification and status registry entries in their owning
   repositories. Commit and push verified slices, inspect CI, and remove
   task-owned temporary outputs/builds. Preserve unrelated work.

Native windows, save states and later Apple II boards remain separate work.
The three archived workspaces named in RULES.md were checked and are absent;
existing live shared runtimes and launcher are the implementation precedent.
