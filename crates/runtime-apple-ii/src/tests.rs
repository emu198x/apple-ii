use super::*;
use emu198x_shell::*;
use format198x_apple_ii_woz::{Info, Track, Woz};
use serde_json::json;
use std::borrow::Cow;

fn runtime(program: &[u8], disk: bool) -> AppleIIRuntime {
    let mut rom = [0xea; 8192];
    rom[..program.len()].copy_from_slice(program);
    let end = 0xe000 + program.len() as u16;
    rom[program.len()..program.len() + 3].copy_from_slice(&[0x4c, end as u8, (end >> 8) as u8]);
    rom[0x1ffc..0x1ffe].copy_from_slice(&[0, 0xe0]);
    let mut fw = FirmwareSet::new();
    fw.push(FirmwareImage::new(profiles::FIRMWARE[0], &rom));
    fw.push(FirmwareImage::new(profiles::FIRMWARE[1], &[0; 512]));
    if disk {
        fw.push(FirmwareImage::new(profiles::FIRMWARE[2], &[0; 256]));
        fw.push(FirmwareImage::new(profiles::FIRMWARE[3], &[0; 256]));
    }
    AppleIIRuntime::from_firmware(&fw).expect("authored firmware")
}
fn key(name: &str, pressed: bool) -> InputEvent {
    InputEvent::Key {
        name: Cow::Owned(name.into()),
        pressed,
    }
}
#[derive(Default)]
struct Video(Vec<(u64, u32, u32, Vec<u8>)>);
impl FrameSink for Video {
    fn push_frame(&mut self, f: FramePacket<'_>) -> Result<(), MachineError> {
        self.0
            .push((f.timestamp.get(), f.width, f.height, f.pixels.to_vec()));
        Ok(())
    }
}
#[derive(Default)]
struct Audio(usize);
impl AudioSink for Audio {
    fn push_audio(&mut self, a: AudioPacket<'_>) -> Result<(), MachineError> {
        assert_eq!((a.sample_rate, a.channels), (48000, 1));
        self.0 += a.samples.len();
        Ok(())
    }
}
#[test]
fn television_geometry_uses_the_serializer_clock_without_depending_on_crop() {
    let m = runtime(&[], false);
    let display = m.display().expect("NTSC television");
    let emu198x_shell::display::Display::Television { pixel_clock_hz, .. } = display else {
        panic!("original board emits composite video");
    };
    assert_eq!(pixel_clock_hz, m.profile().clock.rate.numerator_hz as f64);
    let par = display.pixel_aspect_ratio(560, 192);
    assert!((0.42..0.44).contains(&par));
    assert_eq!(par, display.pixel_aspect_ratio(280, 192));
}

#[test]
fn native_frames_exact_ticks_and_audio_share_one_clock() {
    let mut m = runtime(&[], false);
    let mut video = Video::default();
    let mut audio = Audio::default();
    let mut trace = NullTraceSink;
    let mut host = HostIo {
        input_events: &[],
        frame_sink: &mut video,
        audio_sink: &mut audio,
        trace_sink: &mut trace,
    };
    m.run_ticks(0, &mut host).expect("zero");
    m.run_until(MachineTime::new(0), &mut host).expect("past");
    m.run_ticks(37, &mut host).expect("partial");
    assert_eq!(m.time().get(), 37);
    m.run_frame(&mut host).expect("field");
    assert_eq!(m.machine().frame_count(), 1);
    let first = m.time().get();
    m.run_frame(&mut host).expect("next field");
    assert_eq!(m.time().get() - first, profiles::FRAME_TICKS);
    m.run_ticks(profiles::FRAME_TICKS + 50, &mut host)
        .expect("field and partial");
    assert!(m.run_ticks(u64::MAX, &mut host).is_err());
    assert_eq!(video.0.first().expect("partial").0, 37);
    let last = video.0.last().expect("frame");
    assert_eq!((last.0, last.1, last.2), (m.time().get(), 560, 192));
    assert_eq!(last.3.len(), 560 * 192 * 4);
    assert!(last.3.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    assert_eq!(
        audio.0 as u64,
        m.time().get() * 48000 / emu198x_machine_apple_ii_rev0_ntsc::MASTER_HZ
    );
}
#[test]
fn firmware_errors_are_explicit_and_save_states_are_not_advertised() {
    let mut fw = FirmwareSet::new();
    assert!(AppleIIRuntime::from_firmware(&fw).is_err());
    fw.push(FirmwareImage::new(profiles::FIRMWARE[0], &[0; 8191]));
    fw.push(FirmwareImage::new(profiles::FIRMWARE[1], &[0; 512]));
    assert!(matches!(
        AppleIIRuntime::from_firmware(&fw),
        Err(MachineError::InvalidFirmware { .. })
    ));
    let mut fw = FirmwareSet::new();
    fw.push(FirmwareImage::new(profiles::FIRMWARE[0], &[0; 8192]));
    fw.push(FirmwareImage::new(profiles::FIRMWARE[1], &[0; 512]));
    fw.push(FirmwareImage::new(profiles::FIRMWARE[2], &[0; 256]));
    assert!(matches!(
        AppleIIRuntime::from_firmware(&fw),
        Err(MachineError::MissingFirmware { .. })
    ));
    let mut m = runtime(&[], false);
    assert_eq!(m.capabilities(), m.profile().capabilities);
    assert!(m.snapshot().is_err());
    assert!(m.restore(&[]).is_err());
    assert!(m.debug_target().is_some() && m.debug_target_mut().is_some());
}
#[test]
fn debug_steps_stop_after_completed_stores_and_halts_are_bounded() {
    let mut m = runtime(&[0xa9, 0x42, 0x8d, 0, 2, 0x02], false);
    for _ in 0..3 {
        assert!(m.step_instruction() > 0);
    }
    assert_eq!(m.peek(0x200), 0x42);
    assert_eq!(m.pc(), 0xe005);
    assert!(m.disassemble(0xe000).expect("decode").0.contains("LDA"));
    assert!(m.disassemble(65536).is_none());
    assert_eq!(m.peek(65536), 255);
    m.poke(65536, 0);
    m.poke(0x300, 9);
    m.poke(0xe000, 0);
    m.poke(0xc030, 0);
    assert_eq!(m.peek(0x300), 9);
    assert_eq!(m.peek(0xe000), 0xa9);
    assert!(!m.machine().speaker_level());
    m.step_instruction_no_resync();
    let time = m.time();
    let count = m.instruction_boundary_count();
    assert_eq!(m.step_instruction(), 0);
    assert_eq!(m.time(), time);
    assert_eq!(m.instruction_boundary_count(), count);
    assert_eq!(m.cpu_state()["halted"], true);
    m.resync();
    m.reset(ResetKind::Soft);
    assert_eq!(m.time(), time);
    assert_eq!(m.peek(0x300), 9);
    assert!(m.step_instruction() > 0);
    m.reset(ResetKind::Hard);
    assert_eq!(m.peek(0x300), 9);
}
#[test]
fn keyboard_mapping_validates_chords_and_backpressure_without_hidden_ticks() {
    let mut m = runtime(&[], false);
    let keyboard = m.keyboard_target().expect("keyboard");
    assert!(!keyboard.key_names_hint().is_empty());
    assert_eq!(keyboard.key_timing().default_hold_frames, 1);
    for c in [
        'a', 'A', '9', ' ', '\n', '\r', '\x1b', '\x08', '\x02', '!', '~',
    ] {
        let keys = keyboard.keys_for_char(c).expect("typeable");
        assert!(keys.iter().all(|k| keyboard.key_name_is_valid(k)));
    }
    assert!(keyboard.keys_for_char('é').is_none());
    assert!(!keyboard.key_name_is_valid("missing"));
    for name in ["return", "esc", "left", "right", "space", "z"] {
        crate::input::apply_input_event(&mut m.input, &key(name, true)).expect("key");
        crate::input::apply_input_event(&mut m.input, &key(name, false)).expect("release");
    }
    assert_eq!(
        m.input.queue.iter().copied().collect::<Vec<_>>(),
        [13, 27, 8, 21, 32, 90]
    );
    m.input.queue.clear();
    crate::input::apply_input_event(&mut m.input, &key("ctrl", true)).expect("ctrl");
    crate::input::apply_input_event(&mut m.input, &key("b", true)).expect("b");
    crate::input::apply_input_event(&mut m.input, &key("ctrl", false)).expect("release");
    assert_eq!(m.input.queue.pop_front(), Some(2));
    assert!(crate::input::apply_input_event(&mut m.input, &key("missing", true)).is_err());
    for _ in 0..256 {
        crate::input::apply_input_event(&mut m.input, &key("a", true)).expect("queue");
    }
    assert!(crate::input::apply_input_event(&mut m.input, &key("a", true)).is_err());
    assert_eq!(m.time().get(), 0);
    m.tick();
    assert_eq!(m.machine().peek(0xc000), 0xc1);
    assert_eq!(m.input.queue.len(), 255);
    m.tick();
    assert_eq!(m.input.queue.len(), 255);
    m.reset(ResetKind::Soft);
    assert!(m.input.queue.is_empty());
}
#[test]
fn query_catalogue_is_sorted_filtered_and_live() {
    let m = runtime(&[], false);
    let q = AppleIIQueries;
    let paths = q.query_paths(&m, None);
    assert!(paths.windows(2).all(|w| w[0] < w[1]));
    for path in paths {
        assert_eq!(
            q.query(&m, &path).expect("query").expect("known").path,
            path
        );
    }
    assert!(q.query(&m, "unknown").expect("unknown").is_none());
    assert_eq!(
        q.query_paths(&m, Some("cpu.")),
        ["cpu.master_clock", "cpu.pc"]
    );
    let m = runtime(&[], true);
    for path in q.query_paths(&m, Some("disk.")) {
        assert!(q.query(&m, &path).expect("query").is_some());
    }
    assert_eq!(
        q.query(&m, "disk.installed")
            .expect("query")
            .expect("known")
            .value,
        json!(true)
    );
}
fn woz() -> Vec<u8> {
    let mut map = [255; 160];
    map[0] = 0;
    Woz::from_tracks(
        Info::default(),
        map,
        &[Track {
            bits: vec![0x96; 32],
            bit_count: 256,
        }],
    )
    .expect("authored disk")
    .to_bytes()
}
fn medium<'a>(
    slot: &'static str,
    kind: MediaKind,
    bytes: &'a [u8],
    writable: bool,
) -> MediaSet<'a> {
    let mut set = MediaSet::new();
    set.push(MediaImage::new(slot, kind, bytes).writable(writable));
    set
}
#[test]
fn mounting_export_and_ejection_preserve_sources_and_reject_unsafe_operations() {
    let bytes = woz();
    let before = bytes.clone();
    let mut m = runtime(&[], false);
    assert!(m.load_media(&MediaSet::new()).is_ok());
    assert!(
        m.load_media(&medium("floppy-0", MediaKind::Disk, &bytes, false))
            .is_err()
    );
    let mut m = runtime(&[], true);
    for (slot, kind, data) in [
        ("missing", MediaKind::Disk, bytes.as_slice()),
        ("floppy-0", MediaKind::Cartridge, bytes.as_slice()),
        ("floppy-0", MediaKind::Disk, &b"invalid"[..]),
    ] {
        assert!(m.load_media(&medium(slot, kind, data, false)).is_err());
    }
    assert!(m.export_media("floppy-0", "woz").is_err());
    assert!(m.export_media("bad", "woz").is_err());
    m.load_media(&medium("floppy-0", MediaKind::Disk, &bytes, false))
        .expect("archive");
    assert!(
        m.machine()
            .disk_ii()
            .expect("card")
            .disk(0)
            .expect("disk")
            .write_protected()
    );
    assert_eq!(m.export_media("floppy-0", "woz").expect("export"), bytes);
    assert!(m.export_media("floppy-0", "dsk").is_err());
    m.load_media(&medium("floppy-1", MediaKind::Disk, &bytes, true))
        .expect("work copy");
    assert!(
        !m.machine()
            .disk_ii()
            .expect("card")
            .disk(1)
            .expect("disk")
            .write_protected()
    );
    let exported = m.export_media("floppy-1", "woz").expect("work export");
    assert!(
        !Woz::parse(&exported)
            .expect("valid export")
            .info()
            .write_protected
    );
    assert_eq!(bytes, before);
    m.machine.disk_ii_mut().expect("card").select(9, None);
    assert!(m.export_media("floppy-0", "woz").is_err());
    assert!(m.eject_media("floppy-0").is_err());
    assert!(
        m.load_media(&medium("floppy-0", MediaKind::Disk, &bytes, false))
            .is_err()
    );
    m.reset(ResetKind::Soft);
    assert!(m.machine().disk_ii().expect("card").disk(1).is_some());
    m.eject_media("floppy-0").expect("eject");
    assert!(m.images[0].is_none());
    assert!(m.eject_media("bad").is_err());
}
#[test]
fn shared_script_executes_ticks_queries_and_bounded_negative_waits() {
    let mut session = HeadlessSession::new_with_query_provider(
        runtime(&[], false),
        profiles::FRAME_TICKS,
        AppleIIQueries,
    );
    let script: HeadlessScript = serde_json::from_value(json!([
        {"action":"run_ticks","ticks":123},
        {"action":"query","path":"cpu.master_clock"},
        {"action":"query_paths","prefix":"disk."}
    ]))
    .expect("script");
    let observations = script.execute_collect(&mut session).expect("exec");
    assert_eq!(session.machine().time().get(), 123);
    assert_eq!(observations.len(), 3);
    assert!(
        serde_json::to_string(&observations)
            .expect("JSON")
            .contains("123")
    );
    let script:HeadlessScript=serde_json::from_value(json!([{ "action":"wait_for_query_contains","path":"screen.text","needle":"IMPOSSIBLE","max_frames":1}])).expect("script");
    assert!(script.execute(&mut session).is_err());
}

#[test]
#[ignore = "external original ROM and character bitmap; see docs/verification.md"]
fn original_rom_types_basic_through_the_shared_executor() {
    let rom =
        std::fs::read(std::env::var("EMU198X_APPLE_II_ROM").expect("set ROM path")).expect("ROM");
    let chars = std::fs::read(std::env::var("EMU198X_APPLE_II_CHARS").expect("set glyph path"))
        .expect("glyphs");
    let mut fw = FirmwareSet::new();
    fw.push(FirmwareImage::new(profiles::FIRMWARE[0], &rom));
    fw.push(FirmwareImage::new(profiles::FIRMWARE[1], &chars));
    let m = AppleIIRuntime::from_firmware(&fw).expect("firmware");
    let mut session =
        HeadlessSession::new_with_query_provider(m, profiles::FRAME_TICKS, AppleIIQueries);
    let script: HeadlessScript = serde_json::from_value(json!([
        {"action":"wait_for_boot","max_frames":120},
        {"action":"type_string","text":"\u{2}\n10 PRINT 6*7\n20 END\nRUN\n"},
        {"action":"wait_for_query_contains","path":"screen.text","needle":"42","max_frames":120}
    ]))
    .expect("script");
    script
        .execute(&mut session)
        .expect("BASIC prints 42 through shared executor");
    assert!(
        !session
            .machine()
            .machine()
            .text_rows()
            .join("\n")
            .contains(" ERR")
    );
    assert!(
        session
            .machine()
            .machine()
            .text_rows()
            .iter()
            .any(|r| r.trim() == "42")
    );
}
