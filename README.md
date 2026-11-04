# Apple II

[![CI](https://github.com/emu198x/apple-ii/actions/workflows/ci.yml/badge.svg)](https://github.com/emu198x/apple-ii/actions/workflows/ci.yml)

The Apple II family's first machine: the **original NTSC revision-0 board**,
with 48 KB fitted, running the original Monitor and Integer BASIC through the
published `emu198x-mos-6502` core.

This is a machine library and a headless console example. It is not yet a native
window or an installable Emu198x suite emulator. II+, IIe and subsequent boards
will have their own machine crates here; shared hardware will be extracted when
the second board needs it.

## What runs

- Real reset-vector execution, Monitor, Ctrl-B/Return entry into Integer BASIC.
- `PRINT 6*7` produces `42`; a stored `PRINT 7*8` program runs and lists.
- 40×24 text on either page, interleaved display memory, inverse/flashing codes,
  and a 280×192 monochrome digital raster fetched from live RAM.
- Lo-res, revision-0 hi-res and mixed text/graphics on either page, with a
  separate 560×192 colour view. Hi-res ignores D7, as the original board does.
- Original Integer BASIC graphics commands execute through the ROM; hi-res
  RAM and switches can be selected with signed-address `POKE` commands.
- Keyboard latch, mirrored reads, strobe acknowledgement and debugger-safe peeks.
- Speaker toggles on reads and writes, including NMOS read-modify-write dummy
  writes, with raw 48 kHz samples and observable toggle edges.
- Optional slot-6 Disk II: real P5/P6 PROMs, two bitstream drives and WOZ 2.
  DOS 3.3 boots and catalogs; writable work copies save and reload programs.
- A single 14.31818 MHz timebase, separate scanner/CPU RAM phases and the long
  cycle: 65 CPU periods / 912 crystal ticks per line, 262 lines per frame.

The revision-0 constructor does not synthesize a power-on RESET circuit. Call
`reset()` to press RESET. RAM and latch initialization is deterministic rather
than a simulation of analogue power-up randomness.

Cassette transport, game peripherals, optional ROM cards, save states and native host integration belong
to later milestones. Audio is a raw digital level, not a speaker/acoustic model.
Text flashing uses a nominal 2 Hz presentation oscillator; its analogue circuit
has not been measured. Colour uses a reference four-bit square filter and
nominal palette, not a measured analogue television model. Revision 0 retains
colour burst in text, so its colour view includes text fringes. The external test
character patterns have not been verified against an original 2513 mask ROM.

## Build and check

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The firmware tests are deliberately ignored in fixture-free CI. Explicitly asking
for them without the inputs fails; a green ordinary test run is not boot evidence.

```sh
export EMU198X_APPLE_II_ROM=/path/to/original-e000.rom
export EMU198X_APPLE_II_CHARS=/path/to/characters-64.bin
cargo test --test rom_boot -- --ignored --nocapture
```

`original-e000.rom` is exactly 8192 bytes, E000–FFFF in address order, with the
original Monitor, not the II+ Autostart Monitor. `characters-64.bin` is 512
bytes: 64 characters × 8 rows, character index `screen_code & 63`, bit 6 at
the left, bit 0 at the right, 1 for ink. This is a normalized bitmap input,
not an assertion about physical ROM pin/address layout. No ROM is embedded.

Run an interactive headless console:

```sh
cargo run --example boot -- "$EMU198X_APPLE_II_ROM" "$EMU198X_APPLE_II_CHARS"
```

It presses RESET, types Ctrl-B/Return and prints the text RAM. Each line from
stdin is typed through the keyboard latch; the ROM executes it. EOF exits.
An optional third argument exports the current monochrome raster as a new PPM
file; append `--colour` for the 560×192 colour view. An existing file is never
overwritten. The keyboard wait is bounded at one emulated second, allowing
ROM operations such as `GR` to finish. This console advances emulated
time while processing input, not in real time while stdin is idle.

See [verification](docs/verification.md), [hardware sources](docs/hardware.md)
and the [graphics plan](docs/plans/2026-11-04-revision-zero-graphics.md).

## Disk II

Install the separately published `emu198x-apple-disk-ii` controller with
`install_disk_ii`. `disk::mount` adapts a validated `format198x-apple-ii-woz`
image. Archive mounts are protected; `disk::export` returns updated bytes for a
new file. The libraries never open or overwrite a source path.

The headless example enters `C600G` through the original Monitor:

```sh
cargo run --release --example disk_boot -- "$EMU198X_APPLE_II_ROM" "$EMU198X_APPLE_II_CHARS" "$EMU198X_DISK_II_P5" "$EMU198X_DISK_II_P6" "$EMU198X_APPLE_II_DOS"
```

Append `--work-copy /new/path.woz` to create an explicitly writable copy. This
clears the protection tab on the memory copy and exports at EOF with overwrite
prevention. Each input line is typed through the keyboard. DOS `CATALOG` may
wait for a key between pages. This example advances time while handling input.

Writes normalize transition intervals to bit cells. Exact flux/splice timing,
copy-protected writes, acceleration and head settling remain unqualified.
Only 5.25-inch WOZ2 bitstreams are accepted; flux and 3.5-inch images fail
explicitly. See [disk qualification](docs/verification.md#disk-ii).
