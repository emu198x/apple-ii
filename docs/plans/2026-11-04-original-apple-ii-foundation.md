# Original Apple II foundation

Engineering-frontier work, agreed with Steve on 4 November 2026. Cover the
family eventually; start with the original Apple II, text, keyboard and speaker.
Reuse of `emu198x-mos-6502` 0.6.1 is explicitly approved. No other new direct
dependencies.

## Design

- Own family repository `Emu198x/apple-ii/`, per the flagship's
  `knowledge/decisions/workspace-shape.md` amendment of 2 November.
- First board: original NTSC revision 0, 48 KB RAM fitted. A different board
  gets a separate machine crate, per RULES.md rule 34. Do not add II+/IIe
  model branches or a speculative family abstraction.
- `crates/machine-apple-ii-rev0-ntsc/` publishes under
  `emu198x-machine-apple-ii-rev0-ntsc`. It uses the existing NMOS 6502 and one
  14.31818 MHz master-clock timebase, including the long cycle. Video fetches
  precede CPU accesses; reads have real I/O side effects, debugger peeks do not.
- Supply the original 8 KB E000–FFFF ROM and 512-byte, 64-glyph character
  patterns as caller-owned inputs. No firmware is committed. Missing optional
  D000/D800 ROM sockets and empty slots return the scanner's floating bus.
- Reset is an explicit RESET-key action on revision 0. The original Monitor
  reaches Integer BASIC through Ctrl-B and Return, not an Autostart-ROM shortcut.
- Text pixels are fetched from live memory as the scanner advances. Preserve
  inverse/flashing codes and both text pages. Output is monochrome digital
  text, not a claim of revision-0 composite colour qualification.
- Graphics, disk, cassette transport, game peripherals, save states, shell
  runtime/native window and later boards remain subsequent milestones. Do not
  advertise this as a shipping emulator in the suite registry.

## Tasks and checks

1. Record source and fixture provenance in `docs/hardware.md` and `README.md`.
   Consult the primary library's Sather and original Apple manuals, the vendored
   MiSTer timing/scanner, and MAME's original-II map. Check archives before code;
   the three archive paths named in RULES.md are absent on this machine.
2. Implement `src/timing.rs`, `src/video.rs`, `src/lib.rs` and `Cargo.toml`:
   master clock, CPU bus, original ROM sockets, keyboard latch/strobe, speaker
   edges, video switches and 40-column text. Verify timing and scanner addressing
   against independent reference-derived observations, including blanking.
3. Add integration tests in `tests/foundation.rs`: CPU-executed reset/vector and
   ROM immutability, keyboard mirrors/strobe, read-modify-write I/O accesses,
   screen folds/page selection and fetch-before-write ordering, and speaker
   samples derived from master-clock time. Exercise observable invariants.
4. Add `tests/rom_boot.rs` and `examples/boot.rs` using external firmware.
   The ignored firmware check must fail when explicitly requested without its
   inputs. Execute original Monitor → Ctrl-B/Return → Integer BASIC → PRINT
   arithmetic and a short RUN/LIST program; inspect screen memory and pixels.
   Retain firmware hashes and the actual results in `docs/verification.md`.
5. Run format, Clippy with warnings denied, all fixture-free tests and explicit
   firmware tests. Demonstrate meaningful tests fail under controlled mutations,
   restore, then run the clean checks. Add CI without external ROM downloads.
6. Commit and push the new repository; register it in umbrella
   `docs/repo-map.yaml`, verify the map, commit and push that change separately.
   No publishing to crates.io or native-host integration in this slice.
