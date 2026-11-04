# Foundation verification

Verified locally on 4 November 2026, macOS arm64, Rust 1.99.1. This records the
first foundation's evidence; it is not a family-wide compatibility matrix.

## Observed outcome

The original non-Autostart ROM reaches the Monitor's `*` prompt after RESET.
Ctrl-B and Return enter Integer BASIC. Keyboard input through the emulated
latch produces these results, with no entry-point traps or memory injection:

```text
>PRINT 6*7
42
>10 PRINT 7*8
>20 END
>RUN
56
>LIST
   10 PRINT 7*8
   20 END
```

The headless console also executed `CALL -936` through the ROM to clear the
screen before this capture. The [actual digital raster](evidence/integer-basic.png)
was exported by the example; its text and arithmetic were visually inspected.
The capture is monochrome and uses the external logical character patterns.

![Integer BASIC executing and listing the test program](evidence/integer-basic.png)

Full [firmware-test output](evidence/rom-boot.txt) and
[headless-console output](evidence/console.txt) retain the observed screens.
Firmware identities and source slices are in [hardware.md](hardware.md) and
[fixtures.json](fixtures.json). The ROM SHA-256 is
`831989e3efdedd7e8dbd217f25c7101ee2d6e5aaee3de0e0f2e67b80f59fa096`.

## Checks

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test --workspace`: 14 fixture-free tests; the firmware test is
  explicitly ignored here, not counted as a boot pass.
- Explicit external-ROM test: 1 passed, including Monitor, BASIC arithmetic,
  stored program, RUN, LIST, a non-flat text raster and a non-halted CPU.
- `cargo llvm-cov --workspace --summary-only`: 95.60% line coverage
  (369/386 lines); the initial CI floor is 95%. This excludes the external ROM
  check and is a test-coverage measurement, not an accuracy score.
- The example was built and executed to produce the captured output above.

The scanner checks cover every visible text cell and both pages, the folded
memory map, original-II A12 during horizontal blanking, 65 CPU periods and
912 master ticks per line, 262 lines per frame, the long cycle, and video
fetches preceding CPU accesses. A RAM change after a fetch changes the next
pass, not the byte already loaded into the text shifter.

CPU-executed programs test reset-vector reads, ROM-write protection, keyboard
mirrors and strobe behaviour, and all three accesses of `INC $C03F`: read,
dummy write, final write. Audio counts are checked against elapsed crystal
time and the speaker's raw output levels.

## Checks demonstrated to fail

Each temporary mutation was applied alone, tested, and restored before the
final clean verification:

- Making speaker toggles write-only fails the read-modify-write invariant:
  `assertion \`left == right\` failed`, `left: 2`, `right: 3`.
  [Output](evidence/negative-speaker-rmw.txt).
- Removing the horizontal-state dependency in the clock network stops scanner
  progress and fails `assertion failed: line_edges.len() > 500`, before the
  scanline-period assertions. [Output](evidence/negative-long-cycle.txt).
- Requesting the ignored firmware test without its environment inputs fails
  with `set EMU198X_APPLE_II_ROM: NotPresent`, rather than silently skipping.
  [Output](evidence/negative-missing-fixtures.txt).

During test development, a program without `END` correctly printed
`*** NO END ERR`. The final workload includes `20 END`. Input pacing also
allows the ROM's own screen scrolling to finish; acknowledging a keyboard
strobe alone does not mean a command has completed.

## Remaining qualification

No running-reference differential trace, original-hardware capture, authentic
revision-0 character-mask verification, analogue flash/composite/speaker
measurement, native window/input/audio-device check, graphics or Disk II
qualification is claimed. The existing CPU's qualification is not proof of
all Apple II board behaviour. See README for the exact implemented surface.

The umbrella repo-map check accepts this repository's entry. Its full run
still exits 1 for eight unrelated unmapped workspaces/repos; running the check
against the umbrella's unchanged HEAD map, excluding this newly created repo,
reproduced the same eight problems. No unrelated workspaces were changed.
