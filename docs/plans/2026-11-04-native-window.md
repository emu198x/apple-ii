# Apple II native window

The agreed design uses the existing `emu198x-ui` harness, with the same
published runtime as scripts and MCP. The existing UI dependency is approved.
This is engineering-frontier work; no machine timing or hardware changes.

1. State the NTSC raster in `crates/runtime-apple-ii/src/runtime.rs`, test
   display geometry against the existing master-clock serializer, and release
   runtime 0.1.1. Check formatting, workspace Clippy and workspace tests.
2. In the flagship's `crates/emu198x-apple-ii/`, add the conventional default
   optional `ui` feature, conditional launcher and thin `src/ui.rs` adapter.
   Reuse runtime character encoding for layout-aware typing. Left Alt/Option
   holds target Ctrl, matching the shared deliberate target-chord mechanism;
   host application shortcuts retain their existing meaning. Pacing derives
   from the runtime clock and native frame budget. Use shared audio, menus,
   pause/reset and startup media; do not add an event loop or control dialect.
3. Test adapter character/control delivery with authored firmware, video/audio
   packets and reset preservation. Run binary tests and Clippy with UI on and
   off; run the existing external-ROM script/MCP tests unchanged.
4. Launch the real native window with the recorded external firmware. Check
   BASIC entry, shifted punctuation, graphics, pause/resume, reset and startup
   disk. Record actual observations and distinguish audio packet delivery from
   audibility on the host device.
5. Update flagship binary README/help and native UI audit plus family
   verification and project status. Commit and push verified changes in their
   owning repos. Keep suite history linear. Remove task build directories and
   stop the task window before reporting completion.

Out of scope: later boards, save states, cassette, paddles, new menus or a
second input implementation. Firmware and loaded disk paths remain immutable.
