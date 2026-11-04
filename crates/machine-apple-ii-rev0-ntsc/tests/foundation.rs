use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, HEIGHT, MASTER_HZ, WIDTH, text_address};

fn board(program: &[u8]) -> AppleII {
    let mut rom = [0xea; 8192];
    rom[..program.len()].copy_from_slice(program);
    let end = 0xe000u16 + program.len() as u16;
    rom[program.len()..program.len() + 3].copy_from_slice(&[0x4c, end as u8, (end >> 8) as u8]);
    rom[0x1ffc..0x1ffe].copy_from_slice(&[0x00, 0xe0]);
    let mut glyphs = [0; 512];
    glyphs[8..16].fill(0x22); // A's controlled pattern, distinct left/right dots.
    let mut board = AppleII::new(rom, glyphs);
    board.reset();
    board
}

fn advance(board: &mut AppleII, ticks: usize) {
    for _ in 0..ticks {
        board.tick();
    }
}

#[test]
fn reset_fetches_vector_executes_rom_and_cannot_overwrite_it() {
    let mut machine = board(&[
        0xa9, 0x42, 0x8d, 0x00, 0x02, // LDA #$42; STA $0200
        0xa9, 0x00, 0x8d, 0x00, 0xe0, // attempted ROM write
    ]);
    let mut reads = Vec::new();
    for _ in 0..2000 {
        if let Some(bus) = machine.tick().bus
            && bus.read
        {
            reads.push(bus.address);
        }
    }
    assert!(reads.windows(2).any(|w| w == [0xfffc, 0xfffd]));
    assert_eq!(machine.peek(0x0200), 0x42);
    assert_eq!(machine.peek(0xe000), 0xa9);
    let clock = machine.master_clock();
    machine.reset();
    assert_eq!(machine.master_clock(), clock);
    assert_eq!(machine.peek(0x0200), 0x42);
}

#[test]
fn keyboard_latch_mirrors_and_clear_strobe_keep_character_bits() {
    let mut machine = board(&[
        0xad, 0x0f, 0xc0, 0x8d, 0x00, 0x02, // mirror reads pending character
        0xad, 0x1f, 0xc0, // mirror clears strobe
        0xad, 0x00, 0xc0, 0x8d, 0x01, 0x02,
    ]);
    assert!(machine.key(b'A'));
    assert!(!machine.key(b'B'));
    assert!(!machine.key(128));
    for _ in 0..4 {
        assert_eq!(machine.peek(0xc01f), 0);
    }
    assert!(
        machine.key_pending(),
        "debugger peeks must not acknowledge keys"
    );
    advance(&mut machine, 3000);
    assert_eq!(machine.peek(0x0200), 0xc1);
    assert_eq!(machine.peek(0x0201), 0x41);
    assert!(!machine.key_pending());
    assert!(machine.key(b'B'));
}

#[test]
fn keyboard_writes_are_ignored_but_strobe_clear_writes_work() {
    let mut machine = board(&[
        0xa9, 0x00, 0x8d, 0x00, 0xc0, 0xad, 0x00, 0xc0, 0x8d, 0x00, 0x02, 0x8d, 0x10, 0xc0,
    ]);
    assert!(machine.key(b'Z'));
    advance(&mut machine, 2000);
    assert_eq!(machine.peek(0x0200), 0xda);
    assert_eq!(machine.peek(0xc000), b'Z');
}

#[test]
fn speaker_observes_read_and_both_nmos_rmw_writes() {
    let mut machine = board(&[0xee, 0x3f, 0xc0]); // INC mirrored speaker: read, dummy write, final write
    let mut accesses = Vec::new();
    let mut edges = 0;
    for _ in 0..2000 {
        let output = machine.tick();
        edges += usize::from(output.speaker_edge);
        if let Some(bus) = output.bus
            && bus.address == 0xc03f
        {
            accesses.push(bus);
        }
    }
    assert_eq!(accesses.len(), 3);
    assert!(accesses[0].read);
    assert!(!accesses[1].read && !accesses[2].read);
    assert_eq!(accesses[1].data, accesses[0].data);
    assert_eq!(accesses[2].data, accesses[0].data.wrapping_add(1));
    assert_eq!(edges, accesses.len());
    assert!(machine.speaker_level());
}

#[test]
fn audio_sample_count_and_levels_follow_elapsed_master_time() {
    let mut silent = board(&[]);
    let mut high = board(&[0xad, 0x30, 0xc0]);
    let ticks = 100_000;
    let mut count = 0;
    let mut last = 0.0;
    for _ in 0..ticks {
        if let Some(sample) = silent.tick().audio_sample {
            assert_eq!(sample, -0.25);
            count += 1;
        }
        if let Some(sample) = high.tick().audio_sample {
            last = sample;
        }
    }
    assert_eq!(count, ticks * 48_000 / MASTER_HZ);
    assert_eq!(last, 0.25);
}

#[test]
fn text_folds_are_unique_and_match_known_row_starts() {
    let starts = [
        0x400, 0x480, 0x500, 0x580, 0x600, 0x680, 0x700, 0x780, 0x428, 0x4a8, 0x528, 0x5a8, 0x628,
        0x6a8, 0x728, 0x7a8, 0x450, 0x4d0, 0x550, 0x5d0, 0x650, 0x6d0, 0x750, 0x7d0,
    ];
    let mut addresses = std::collections::HashSet::new();
    for (row, start) in starts.into_iter().enumerate() {
        assert_eq!(text_address(false, 0, row), Some(start));
        for col in 0..40 {
            let address = text_address(false, col, row).expect("text address");
            assert!(addresses.insert(address));
            assert_eq!(text_address(true, col, row), Some(address + 0x400));
        }
    }
    assert_eq!(addresses.len(), 960);
    assert_eq!(text_address(false, 40, 0), None);
    assert_eq!(text_address(false, 0, 24), None);
}

#[test]
fn text_pixels_follow_rom_glyphs_inverse_codes_and_page_selection() {
    let mut normal = board(&[0xa9, 0xc1, 0x8d, 0x00, 0x04]);
    let mut inverse = board(&[0xa9, 0x01, 0x8d, 0x00, 0x04]);
    let mut page2 = board(&[0xa9, 0xc1, 0x8d, 0x00, 0x08, 0xad, 0x55, 0xc0]);
    for machine in [&mut normal, &mut inverse, &mut page2] {
        advance(machine, 500_000);
    }
    let a = normal.framebuffer().expect("text");
    let b = inverse.framebuffer().expect("text");
    let c = page2.framebuffer().expect("text");
    assert_eq!(a.len(), WIDTH * HEIGHT);
    assert_eq!(
        &a[..7],
        &[
            0xff000000, 0xffffffff, 0xff000000, 0xff000000, 0xff000000, 0xffffffff, 0xff000000
        ]
    );
    for y in 0..8 {
        for x in 0..7 {
            let index = y * WIDTH + x;
            assert_eq!(a[index] ^ b[index], 0x00ff_ffff);
            assert_eq!(a[index], c[index]);
        }
    }
    assert!(normal.text_rows()[0].starts_with('A'));
    assert!(page2.text_rows()[0].starts_with('A'));
}

#[test]
fn graphics_requests_fail_explicitly_until_text_is_selected() {
    let mut machine = board(&[0xad, 0x50, 0xc0]);
    advance(&mut machine, 2000);
    assert!(machine.framebuffer().is_err());
    machine.reset();
    advance(&mut machine, 2000);
    assert!(
        machine.framebuffer().is_err(),
        "reset must not reset board video latches"
    );
}

#[test]
fn empty_slots_and_empty_rom_sockets_read_live_scanner_data() {
    // Default RAM is zero. A missing ROM must float, not supply the ROM filler.
    let mut machine = board(&[
        0xad, 0x00, 0xd0, 0x8d, 0x00, 0x02, 0xad, 0x00, 0xc6, 0x8d, 0x01, 0x02,
    ]);
    advance(&mut machine, 2000);
    assert_eq!(machine.peek(0x0200), 0);
    assert_eq!(machine.peek(0x0201), 0);
}
