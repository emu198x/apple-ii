use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, COLOUR_WIDTH, WIDTH};

fn write(program: &mut Vec<u8>, address: u16, value: u8) {
    program.extend([0xa9, value, 0x8d, address as u8, (address >> 8) as u8]);
}

fn machine(program: &[u8]) -> AppleII {
    let mut rom = [0xea; 8192];
    rom[..program.len()].copy_from_slice(program);
    let end = 0xe000 + program.len() as u16;
    rom[program.len()..program.len() + 3].copy_from_slice(&[0x4c, end as u8, (end >> 8) as u8]);
    rom[0x1ffc..0x1ffe].copy_from_slice(&[0, 0xe0]);
    let mut glyphs = [0; 512];
    glyphs[8..16].fill(0x40); // A: leftmost dot only.
    let mut machine = AppleII::new(rom, glyphs);
    machine.reset();
    advance(&mut machine);
    machine
}

fn advance(machine: &mut AppleII) {
    for _ in 0..500_000 {
        machine.tick();
    }
}

#[test]
fn cpu_selects_both_lores_pages_and_reset_keeps_the_video_latches() {
    for page2 in [false, true] {
        let mut program = Vec::new();
        // Leave the other page black; a wrong page selection must fail.
        let base = if page2 { 0x800 } else { 0x400 };
        write(&mut program, base, 0xf0); // upper black, lower white
        write(&mut program, base + 1, 0xf0);
        for switch in [0xc050, 0xc052, 0xc056, if page2 { 0xc055 } else { 0xc054 }] {
            write(&mut program, switch, 0);
        }
        let mut board = machine(&program);
        let pixels = board.colour_framebuffer();
        assert!(pixels[2..26].iter().all(|&p| p == 0xff000000));
        assert!(
            pixels[4 * COLOUR_WIDTH + 2..4 * COLOUR_WIDTH + 26]
                .iter()
                .all(|&p| p == 0xffffffff)
        );
        board.reset();
        advance(&mut board);
        assert_eq!(board.colour_framebuffer()[4 * COLOUR_WIDTH + 3], 0xffffffff);
        assert_eq!(
            board.framebuffer().expect("graphics")[4 * WIDTH + 3],
            0xffffffff
        );
    }
}

#[test]
fn cpu_selects_both_pages_in_both_graphics_modes_with_mixed_text_at_line_160() {
    for page2 in [false, true] {
        for mixed in [false, true] {
            for hires in [false, true] {
                let mut program = Vec::new();
                let text = if page2 { 0x800 } else { 0x400 };
                if hires {
                    let base = if page2 { 0x4000 } else { 0x2000 };
                    // Known independent folded addresses: lines 159 and 160.
                    write(&mut program, base + 0x1dd0, 0x7f);
                    write(&mut program, base + 0x250, 0x7f);
                    write(&mut program, text + 0x250, 0xc1);
                } else {
                    write(&mut program, text + 0x1d0, 0xff);
                    write(&mut program, text + 0x250, if mixed { 0xc1 } else { 0xff });
                }
                for switch in [
                    0xc050,
                    if hires { 0xc057 } else { 0xc056 },
                    if mixed { 0xc053 } else { 0xc052 },
                    if page2 { 0xc055 } else { 0xc054 },
                ] {
                    program.extend([0xad, switch as u8, (switch >> 8) as u8]);
                }
                let board = machine(&program);
                let pixels = board.framebuffer().expect("graphics");
                assert!(
                    pixels[159 * WIDTH..159 * WIDTH + 7]
                        .iter()
                        .all(|&p| p == 0xffffffff)
                );
                let bottom = &pixels[160 * WIDTH..160 * WIDTH + 7];
                assert_eq!(bottom[0], 0xffffffff);
                assert!(
                    bottom[1..]
                        .iter()
                        .all(|&p| p == if mixed { 0xff000000 } else { 0xffffffff })
                );
            }
        }
    }
}
