# Apple II family

Read the umbrella PRINCIPLES.md first. This repository follows the flagship's
RULES.md and knowledge/decisions/, especially cpu-bus-interface.md,
each-machine-has-its-own-machine-crate.md and workspace-shape.md. Current plans
live in docs/plans/; consult 2026-11-04-revision-zero-graphics.md alongside the
foundation plan.

One machine crate per board, published with the emu198x- prefix. This first
slice is original NTSC revision 0 with 48 KB fitted. Do not put II+, IIe or
other revisions behind a model branch in this crate. Extract common hardware
when a second board makes the sharing concrete.

Hardware facts come from the umbrella reference/ library; emulator source is
implementation precedent. See docs/hardware.md for the actual consulted paths.
Never commit firmware. Never write back to a loaded firmware or media path.

Before pushing, run cargo fmt --all --check, cargo clippy --workspace
--all-targets -- -D warnings, and cargo test --workspace. Explicit firmware
tests are ignored in fixture-free CI but must fail if requested without inputs.
Record boot claims with the ROM identities and observed program output.
