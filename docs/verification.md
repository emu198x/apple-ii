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
measurement, native window/input/audio-device check, Disk II qualification is claimed. Graphics evidence is recorded below. The existing CPU's qualification is not proof of
all Apple II board behaviour. See README for the exact implemented surface.

The umbrella repo-map check accepts this repository's entry. Its full run
still exits 1 for eight unrelated unmapped workspaces/repos; running the check
against the umbrella's unchanged HEAD map, excluding this newly created repo,
reproduced the same eight problems. No unrelated workspaces were changed.

## Graphics milestone

The revision-0 machine now renders text, lo-res, hi-res and mixed mode from
live fetches on either page. The existing 280×192 monochrome API is preserved;
`colour_framebuffer()` adds a 560×192 nominal colour presentation.

The same identified original ROM executes `GR`, `COLOR=15`, `PLOT 2,2`,
`HLIN 4,8 AT 4` and `VLIN 6,10 AT 10`. The external test checks the resulting
RAM and white PICTURE output. Typed `POKE` commands select hi-res and verify
alternating dots, followed by a D7-only change that leaves the raster unchanged.
`TEXT` restores the console and `PRINT 9*9` produces `81`.
[Both firmware tests pass](evidence/graphics-rom.txt).

Two test-workload corrections came from observing the ROM:

- `GR` takes longer than the former keyboard wait. The next `C` was accepted
  after 1,947,935 master ticks **in addition to** the preceding 1,000,000-tick
  settling interval, about 206 ms in total. The console and firmware helper now
  allow one emulated second for acknowledgement, retaining a finite failure.
- Integer BASIC reports `*** >32767 ERR` for `POKE 49239,0` and the other
  unsigned I/O addresses. The workload uses signed equivalents (`-16297`,
  `-16304`, `-16302`). No optional Programmer's Aid ROM or Applesoft `HGR`
  command is assumed. The first VLIN assertion also had the wrong folded RAM
  address; row 6 of lo-res uses text row 3, address `$058A` at column 10.

The example exported these colour rasters; both were visually inspected.
The lo-res program draws all sixteen colours and retains mixed-mode text.
The hi-res workload types a small caller-owned 6502 fill routine into RAM,
executes it with BASIC's `CALL`, and selects full hi-res through `POKE`.
It fills the page with `$55`, producing violet/green strips and white joins.
This routine is test code, not firmware.

![Sixteen lo-res colours above the mixed-mode prompt](evidence/lores-colour.png)

![Revision-0 hi-res alternating dots](evidence/hires-colour.png)

Reproduce with the existing ROM environment variables, choosing a new PPM path:

```sh
cargo run --example boot -- "$EMU198X_APPLE_II_ROM" "$EMU198X_APPLE_II_CHARS" /tmp/lores.ppm --colour < docs/evidence/lores-input.bas
cargo run --example boot -- "$EMU198X_APPLE_II_ROM" "$EMU198X_APPLE_II_CHARS" /tmp/hires.ppm --colour < docs/evidence/hires-input.bas
```

The [lo-res console](evidence/lores-console.txt), [hi-res console](evidence/hires-console.txt)
and typed input files retain the workloads. The console prints text RAM even
while graphics is selected; the exported raster is the actual display output.
Text colour fringes reflect revision 0's persistent colour burst.

Final checks cover 22 fixture-free tests and two explicit external-ROM tests,
plus format, Clippy and the example build. [Coverage](evidence/graphics-coverage.txt)
is 97.92% (564/576 lines), above the unchanged 95% floor. Coverage includes test
code and is not an accuracy measurement. New invariants cover all 7,680 visible
hi-res byte addresses on each page, all lo-res patterns and nibble halves,
D7 invariance for all 128 hi-res patterns, both modes/pages with mixed selection
at line 160, and RAM changes after a fetch appearing only on the next pass.

The square-filter fixture generator reproduces 64 window/phase cases and a
560-sample row crossing byte boundaries and right-edge blanking. Build it with
an existing C++17 compiler; it adds no build/runtime dependency:

```sh
c++ -std=c++17 -Wall -Wextra -Werror tools/reference-square-filter.cpp -o /tmp/apple-ii-reference
/tmp/apple-ii-reference /tmp/square.csv /tmp/row.csv
cmp /tmp/square.csv crates/machine-apple-ii-rev0-ntsc/tests/fixtures/mame-square-filter.csv
cmp /tmp/row.csv crates/machine-apple-ii-rev0-ntsc/tests/fixtures/mame-square-filter-row.csv
```

Removing lo-res's odd-byte phase correction demonstrably fails with
`nibble=1, y=0`; [negative-test output](evidence/negative-lores-phase.txt)
is retained. The mutation was restored before the clean checks. This is
function-level reference agreement, not a running-MAME differential trace or
an analogue display qualification. Disk II and native host integration remain
future milestones.
