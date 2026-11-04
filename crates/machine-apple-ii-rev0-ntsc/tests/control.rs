use emu198x_machine_apple_ii_rev0_ntsc::AppleII;
#[test]
fn debugger_writes_only_ram_and_boundaries_include_completed_bus_writes() {
    let mut rom = [0xea; 8192];
    rom[..8].copy_from_slice(&[0xa9, 0x42, 0x8d, 0, 2, 0x4c, 5, 0xe0]);
    rom[0x1ffc..0x1ffe].copy_from_slice(&[0, 0xe0]);
    let mut m = AppleII::new(rom, [0; 512]);
    m.poke(0xbfff, 9);
    m.poke(0xe000, 0);
    m.poke(0xc030, 0);
    assert_eq!(m.peek(0xbfff), 9);
    assert_eq!(m.peek(0xe000), 0xa9);
    assert!(!m.speaker_level());
    assert_eq!(m.master_clock(), 0);
    for _ in 0..1000 {
        m.tick();
    }
    assert_eq!(
        m.instruction_boundary_count(),
        0,
        "parked CPU makes no progress"
    );
    m.reset();
    let mut prior = 0;
    let mut saw_store = false;
    let mut completed_store = false;
    for _ in 0..2000 {
        let out = m.tick();
        if let Some(bus) = out.bus
            && !bus.read
            && bus.address == 0x200
        {
            assert_eq!(m.peek(0x200), 0x42);
            assert_eq!(m.instruction_boundary_count(), prior);
            saw_store = true;
        }
        if saw_store && m.instruction_boundary_count() > prior {
            assert_eq!(m.peek(0x200), 0x42);
            assert_eq!(m.cpu().regs.pc, 0xe005);
            completed_store = true;
            break;
        }
        prior = m.instruction_boundary_count();
    }
    assert!(saw_store && completed_store);
    prior = m.instruction_boundary_count();
    m.reset();
    assert_eq!(m.instruction_boundary_count(), prior);
}
