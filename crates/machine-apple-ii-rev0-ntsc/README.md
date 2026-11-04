# Original Apple II, NTSC revision 0

A crystal-driven original Apple II with 48 KB RAM, the published NMOS 6502,
text/lo-res/hi-res/mixed video, keyboard, speaker and optional slot-6 Disk II.
Firmware is caller supplied: 8192-byte E000 ROM and 512-byte normalized glyph
bitmap. No firmware or disk images ship in this crate. Call `reset()` before
execution, then `tick()` once per master crystal tick. Peeks and RAM-only
pokes never access I/O or advance time.

Original Monitor, Integer BASIC and DOS 3.3 workloads are verified against
external fixtures. This is an original-board implementation; later Apple II
boards are separate machines. Cassette, analogue fidelity and protected flux
media remain unqualified. See the repository's hardware and verification docs
for scope, provenance and limitations.
