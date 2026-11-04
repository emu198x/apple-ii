# Verify the merged Apple II window

This verifies the agreed shared-UI design; it adds no dependencies or new
machine behaviour. Preserve the unrelated untracked files in the suite/docs.

1. Recheck the merged `crates/emu198x-apple-ii/src/ui.rs` adapter, shared input
   routing and `emu198x-shell/src/audio.rs` output path. Run the existing binary,
   shared UI and explicit original-ROM process tests against published runtime
   0.1.1, with normal and headless feature configurations.
2. Extend `crates/emu198x-apple-ii/src/ui/tests.rs` with an authored, audible
   speaker loop. Check the captured signal's frequency, range and sample count
   against elapsed master-clock time, rather than only nonconstant samples.
   Check that releasing target Ctrl restores ordinary letters.
3. Rebuild and open the real native window using the recorded original ROM,
   glyphs, P5/P6 and DOS startup image. Use Monitor to install a short authored
   speaker program in RAM; its delay loop produces an approximately 1 kHz tone.
   Check presentation and shared controls. Request a hands-on report of sound
   and physical left-Option+B only once the test window is ready. Do not infer
   either result from a successful device open or an aggregate Alt flag.
4. Record results and exact limits in `docs/verification.md`; update test-count
   pages through `scripts/status/render_status.py` if tests change. Run fmt,
   Clippy and relevant tests, commit and push verified changes in the owning
   repositories. Stop the test window and remove task builds, bundles and logs
   before finishing; keep any hands-on check explicitly pending if unanswered.

Hands-on findings to resolve before completion:

- Physical left Option+B works; `PRINT 6*7` visibly produces `42`.
- The uncleared RAM display was confusing. Verify `CALL -936` clears it and
  document the cold-start screen and revision-0 text colour fringing.
- Audible crackling was reported. Temporary callback counters reproduce audio
  queue underruns in the unoptimised debug build. Compare an optimised build
  with the same fixtures and callback counter before changing buffering.
- Remove all keyboard/audio diagnostics before committing. Keep the measured
  limitation explicit if smooth audible playback is not confirmed.

Verification outcome: physical Option+B and BASIC arithmetic pass. ROM screen
clear removes the confusing startup glyphs. Audible playback fails in debug
and optimised builds; measured host queue underruns remain an open defect.
The temporary keyboard and callback traces are removed from source.

Proposed follow-up, awaiting agreement under flagship RULES.md §31: retain
approximately 50 ms of audio in the existing shared native output queue before
starting/resuming consumption. Keep the existing bounded maximum, re-prime on
clear/reset or starvation, and derive thresholds from output sample rate and
channel count. Exact files: `crates/emu198x-shell/src/audio.rs` and its tests.
Test ordered stereo output under delayed packet delivery, initial buffering,
reset/clear, sustained starvation, recovery and bounded overflow. Show
that delayed-delivery regression fails with immediate consumption restored.
Repeat the native tone/listening check in an optimised build before claiming
the crackling is fixed. A larger configurable buffer is the alternative.

A repeat build hit `No space left on device (os error 28)`. Task-owned release
and incremental caches were removed after stopping the test window; unrelated
active builds were preserved. Rerun checks after recovering space.
