# Original Apple II foundation: evidence boundaries

## Primary library

Hardware facts are owned by the umbrella's primary library, not this repository.
The consulted material lives under `198x/reference/by-system/apple-ii/`:

- `understanding-the-apple-ii.docling/understanding-the-apple-ii.md` — Jim
  Sather, *Understanding the Apple II*: ch.3 clock/scanner timing, ch.5 RAM
  address generation, ch.7 I/O/keyboard, ch.8 text/video, Appendix G board
  revisions. The short sibling `.md` is an OCR routing stub; the canonical
  Docling extract contains the actual book.
- `apple-ii-reference.md` — the library's reference hub: original-II memory
  map, text-page folds, keyboard latch/strobe and speaker.
- `apple-ii-video-teaching.md` — source-cited scanner and revision distinctions,
  especially §6–7 and §9.1. Its long-cycle explanation corrects the hub's
  nominal-clock arithmetic: 64×14 + 16 = 912 ticks per line; the average CPU
  clock is approximately 1.02048 MHz, not a constant 14M/14 clock.

Nothing here claims that a IIe MMU/IOU or an Autostart ROM exists on revision 0.
The original board has empty D000/D800 sockets without Programmer's Aid, four
2 KB ROMs at E000–FFFF, and a keyboard RESET action. The original ROM itself
is the execution witness for the Monitor → Ctrl-B/Return → BASIC path.

## Implementation precedent

The following were read before writing timing or bus code:

- Vendored `198x/emulators/apple-ii/Apple-II_MiSTer/rtl/timing_generator.vhd`
  and `rtl/apple2.vhd`, snapshot commit
  `cb575062c3bacfed08d6ee7e3a9551117c62508c`. György Szombathelyi's IIe
  timing reconstruction credits Stephen A. Edwards's original II+ work and
  Sather's schematics. `timing.rs` expresses the shared RAS/AX/CAS/Q3/PHI0
  gate network in Rust. IIe-only switches and the HAL video controls are not
  imported. The visible scanner is the original-II mapping.
- Vendored `198x/emulators/multi-system/mame/src/mame/apple/apple2.cpp`:
  original-II memory/I/O mirrors, keyboard acknowledgement, speaker read/write
  toggles and the `ROM_START(apple2)` checksums.
- The adjacent `apple2video.cpp`, `scanner_address`: duplicated horizontal
  zero, vertical preset and original-II-only A12 during text blanking.

These are implementation precedents, not replacements for the primary sources.
The clock equations and address mapping have reference-derived invariant tests;
there has not yet been a full execution trace comparison against running MAME,
MiSTer or an original board. Passing Integer BASIC is not Disk II or graphics
qualification.

The three external archive paths named by flagship RULES.md rule 25 were checked
and are absent on this machine. No archived machine or CPU code was ported. The
CPU is the existing published NMOS core, `emu198x-mos-6502` 0.6.1. Its silicon
qualification does not by itself qualify this board's wiring.

## Deliberately bounded approximations

The framebuffer is normalized monochrome text, not analogue composite video.
The nominal 2 Hz text flasher is scheduled from elapsed master-clock time; the
real analogue oscillator's tolerance and phase have not been measured. Audio
averages the digital speaker latch over actual elapsed crystal ticks, then
emits a 48 kHz raw level. It models neither the loudspeaker response nor an
audio device. Input glyphs are logical bitmaps supplied by the caller, not an
embedded or authenticated 2513 dump.

## Local firmware identity

Fixtures live outside this repository at `Emu198x/roms/apple-ii/`. They were
extracted from [Apple ][js](https://github.com/whscullin/apple2js/tree/ee0aed25f73c69d0245e86a2a5fccb3324c3056c),
revision `ee0aed25f73c69d0245e86a2a5fccb3324c3056c`:

- `js/roms/system/original.ts`: keep the final 8192 bytes of the 12288-byte
  array. Discard the optional D000/D800 contents. The four 2048-byte CRC32s
  are `c0a4ad3b`, `a99c2cf6`, `62230d38`, `020a86d0`, matching MAME's
  recorded original Integer BASIC and Monitor identities.
- `js/roms/character/apple2_char.ts`: keep the first 512 bytes of the
  2048-byte array, representing the 64 base glyphs in normalized visual order.
  This is a usable test bitmap, not authenticated revision-0 glyph evidence.
  MAME explicitly marks its own original-II character ROM substitute BAD_DUMP.

SHA-256, original source URLs and sizes are retained in
[`fixtures.json`](fixtures.json). ROM bytes are neither committed nor fetched
by CI. Size checks alone do not authenticate firmware.
