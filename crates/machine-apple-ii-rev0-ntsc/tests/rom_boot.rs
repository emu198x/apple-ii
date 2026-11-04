use emu198x_machine_apple_ii_rev0_ntsc::AppleII;

fn advance(machine: &mut AppleII, ticks: usize) {
    for _ in 0..ticks {
        machine.tick();
    }
}

fn type_keys(machine: &mut AppleII, text: &[u8]) {
    for &key in text {
        assert!(machine.key(key), "previous key must have been consumed");
        let mut consumed = false;
        for _ in 0..1_000_000 {
            machine.tick();
            if !machine.key_pending() {
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
