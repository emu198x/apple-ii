use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, MASTER_HZ};

fn advance(machine: &mut AppleII, ticks: usize) {
    for _ in 0..ticks {
        machine.tick();
    }
}

fn type_keys(machine: &mut AppleII, text: &[u8]) {
    for &key in text {
        assert!(machine.key(key), "previous key must have been consumed");
        let mut consumed = false;
        for elapsed in 0..MASTER_HZ {
            machine.tick();
            if !machine.key_pending() {
                if elapsed >= 1_000_000 {
                    println!("Key {key:#04x} acknowledged after {elapsed} master ticks");
                }
                consumed = true;
                break;
            }
        }
        assert!(
            consumed,
            "firmware did not acknowledge key {key:#04x}; PC={:04x}, halted={}\n{}",
            machine.cpu().regs.pc,
            machine.cpu().halted,
            machine.text_rows().join("\n")
        );
        advance(machine, 1_000_000);
    }
}

#[test]
#[ignore = "external original E000 ROM and 64-glyph bitmap; see README"]
fn original_monitor_enters_integer_basic_and_runs_programs() {
    let rom_path = std::env::var("EMU198X_APPLE_II_ROM").expect("set EMU198X_APPLE_II_ROM");
    let chars_path = std::env::var("EMU198X_APPLE_II_CHARS").expect("set EMU198X_APPLE_II_CHARS");
    let rom: [u8; 8192] = std::fs::read(rom_path)
        .expect("read original ROM")
        .try_into()
        .expect("8192 ROM bytes");
    let chars: [u8; 512] = std::fs::read(chars_path)
        .expect("read glyph patterns")
        .try_into()
        .expect("512 glyph bytes");
    let mut machine = AppleII::new(rom, chars);
    machine.reset();
    advance(&mut machine, 4_000_000);
    let monitor = machine.text_rows().join("\n");
    println!("Monitor:\n{monitor}");
    assert!(monitor.contains('*'), "original Monitor prompt");
    type_keys(&mut machine, b"\x02\r");
    assert!(
        machine.text_rows().join("\n").contains('>'),
        "Integer BASIC prompt"
    );
    type_keys(&mut machine, b"PRINT 6*7\r");
    let screen = machine.text_rows().join("\n");
    println!("Arithmetic:\n{screen}");
    assert!(machine.text_rows().iter().any(|row| row.trim() == "42"));
    type_keys(&mut machine, b"10 PRINT 7*8\r20 END\rRUN\rLIST\r");
    let rows = machine.text_rows();
    println!("Program:\n{}", rows.join("\n"));
    assert!(rows.iter().any(|row| row.trim() == "56"));
    assert!(rows.iter().any(|row| row.contains("10 PRINT 7*8")));
    advance(&mut machine, 500_000);
    let pixels = machine.framebuffer().expect("BASIC selected text");
    assert!(pixels.contains(&0xffffffff));
    assert!(pixels.contains(&0xff000000));
    assert!(!machine.cpu().halted);
}

#[test]
#[ignore = "external original E000 ROM and 64-glyph bitmap; see README"]
fn integer_basic_draws_lores_and_switches_hires_through_real_io() {
    let rom: [u8; 8192] =
        std::fs::read(std::env::var("EMU198X_APPLE_II_ROM").expect("set EMU198X_APPLE_II_ROM"))
            .expect("read original ROM")
            .try_into()
            .expect("8192 ROM bytes");
    let chars: [u8; 512] =
        std::fs::read(std::env::var("EMU198X_APPLE_II_CHARS").expect("set EMU198X_APPLE_II_CHARS"))
            .expect("read glyph patterns")
            .try_into()
            .expect("512 glyph bytes");
    let mut machine = AppleII::new(rom, chars);
    machine.reset();
    advance(&mut machine, 4_000_000);
    type_keys(
        &mut machine,
        b"\x02\rGR\rCOLOR=15\rPLOT 2,2\rHLIN 4,8 AT 4\rVLIN 6,10 AT 10\r",
    );
    advance(&mut machine, 500_000);
    assert_eq!(machine.peek(0x482) & 15, 15, "PLOT 2,2");
    assert_eq!(machine.peek(0x504) & 15, 15, "HLIN 4,8 AT 4");
    assert_eq!(machine.peek(0x58a) & 15, 15, "VLIN 6,10 AT 10");
    let width = emu198x_machine_apple_ii_rev0_ntsc::COLOUR_WIDTH;
    assert_eq!(
        machine.colour_framebuffer()[8 * width + 2 * 14 + 6],
        0xffffffff
    );
    println!("Lo-res: PLOT, HLIN and VLIN updated RAM and the live colour raster.");
    // Original Integer BASIC has no HGR command. POKE selects board switches.
    type_keys(
        &mut machine,
        b"POKE 8192,85\rPOKE 8193,42\rPOKE -16297,0\rPOKE -16304,0\rPOKE -16302,0\r",
    );
    advance(&mut machine, 500_000);
    println!(
        "Hi-res command console:\n{}",
        machine.text_rows().join("\n")
    );
    let mono = machine.framebuffer().expect("hires");
    assert_eq!(
        &mono[..7],
        &[
            0xffffffff, 0xff000000, 0xffffffff, 0xff000000, 0xffffffff, 0xff000000, 0xffffffff
        ]
    );
    assert_eq!(machine.peek(8193), 42);
    let first = machine.colour_framebuffer()[2..12].to_vec();
    type_keys(&mut machine, b"POKE 8192,213\r");
    advance(&mut machine, 500_000);
    assert_eq!(
        &machine.colour_framebuffer()[2..12],
        first,
        "D7 ignored on rev0"
    );
    println!("Hi-res: alternating dots rendered; setting D7 leaves revision-0 output unchanged.");
    type_keys(&mut machine, b"TEXT\rPRINT 9*9\r");
    assert!(machine.text_rows().iter().any(|row| row.trim() == "81"));
    println!("TEXT restored the console; PRINT 9*9 produced 81.");
    assert!(!machine.cpu().halted);
}
