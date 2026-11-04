# Original Apple II graphics

Engineering-frontier continuation of the agreed foundation → graphics → Disk II
sequence. Keep the original NTSC revision-0 board and existing CPU dependency.
No new dependencies, new board abstractions or breaking API changes.

## Design

Preserve the existing 280×192 monochrome `framebuffer()` signature. Extend it to
graphics and add a separate 560×192 `colour_framebuffer()` view, one horizontal
sample per 14M tick. Keep signal generation separate from colour decoding:

- Text: normalized glyph bit 6 first, each dot held for two master ticks.
- Hi-res: RAM bit 0 first, seven dots held for two ticks each. Revision 0
  ignores D7; there is no later-board half-dot delay.
- Lo-res: lower nibble for the upper four scanlines, upper nibble for the lower
  four; circulate its four bits at 14M. Odd-byte cycles start at Q2, even at Q0.
- Select text on mixed-mode scanlines 160–191; all modes and both display
  pages use live scanner fetches. Do not read future RAM to colour current pixels.
- Decode captured signal samples using MAME's documented four-bit square-filter
  presentation (composite colour mode 2), preserving neighbour context across
  byte boundaries. A nominal reference palette is presentation, not a measured
  television response. Revision 0 retains colour burst in text, so its colour
  view can show text fringes; its monochrome view remains available.
- Decode only after the required future signal samples have actually arrived;
  flush the right edge against blanking. No frame-end re-read of RAM, no fake
  solid-colour lo-res blocks, and no per-byte reset of chroma phase.

## Files and verification

1. `src/video.rs`: serial text/lo-res/hi-res generation, mixed selection and both
   outputs. Add `src/colour.rs` for the reference filter and palette. Cite Sather
   ch.8 and the vendored MAME implementation; retain BSD attribution for adapted
   reference code in `THIRD_PARTY_NOTICES.md`.
2. `src/lib.rs`: retain `GraphicsNotImplemented` and the existing Result return
   type for source compatibility; return the completed raster in all modes.
   Export colour dimensions and add the colour accessor. Do not change CPU
   cadence, memory map or the timing gate network.
3. `src/timing.rs` tests: verify every visible hi-res address and both pages
   against independently expressed row-address arithmetic; verify mixed-mode
   text/hi-res scanner selection at line 160.
4. `tests/graphics.rs` and existing foundation tests: CPU-executed mode/page
   switches; lo-res upper/lower nibble, all 16 patterns, odd/even byte phases;
   hi-res bit order, D7 invariance, byte-boundary context, mixed-mode bottom
   text, page changes and fetched-byte stability. Keep all foundation checks.
5. Retain a reference-generated colour-filter fixture under `tests/fixtures/`,
   covering all four-bit windows at all four phases, plus actual signal runs
   crossing byte boundaries. Check generated evidence positively and show a
   meaningful test fails under a temporary phase/D7 mutation before restoring.
6. Extend `tests/rom_boot.rs`: original Integer BASIC `GR`, `COLOR=`, `PLOT`,
   `HLIN`/`VLIN`, `TEXT` and hi-res RAM/switch selection exercised through
   typed `POKE` commands. Original Integer BASIC has no built-in `HGR`; the
   optional Programmer's Aid ROM remains absent. Keep inputs identified by the existing fixture
   hashes. Capture lo-res and hi-res output without committing firmware.
7. Extend `examples/boot.rs` with an additive colour-export option. Update
   README, hardware provenance and `docs/verification.md` to the exact new
   surface and limitations, retaining reference and source identities.
8. Run format, Clippy, tests, explicit firmware tests, build and coverage; retain
   the existing 95% coverage floor. Commit, push and open a PR under Protect main.
   Remove task-owned temporary reference builds/exports and disposable caches.

The vendor reference and primary source library already exist. The three
archive paths named by flagship RULES.md remain absent. This slice does not add
Disk II, native UI, cassette transport, later boards or analogue qualification.
