use emu198x_apple_disk_ii::DiskII;
use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, MASTER_HZ, disk};
use format198x_apple_ii_woz::Woz;
fn bytes<const N: usize>(name: &str) -> [u8; N] {
    std::fs::read(std::env::var(name).expect(name))
        .expect("read external fixture")
        .try_into()
        .expect("fixture size")
}
fn advance(m: &mut AppleII, ticks: u64) {
    for _ in 0..ticks {
        m.tick();
    }
}
fn type_keys(m: &mut AppleII, text: &[u8]) {
    for &key in text {
        assert!(m.key(key));
        let mut consumed = false;
        for _ in 0..MASTER_HZ * 30 {
            m.tick();
            if !m.key_pending() {
                consumed = true;
                break;
            }
        }
        assert!(
            consumed,
            "keyboard timeout PC={:04x}\n{}",
            m.cpu().regs.pc,
            m.text_rows().join("\n")
        );
        advance(m, 1_000_000);
    }
}
fn boot(image: &Woz, writable: bool) -> AppleII {
    let mut m = AppleII::new(
        bytes("EMU198X_APPLE_II_ROM"),
        bytes("EMU198X_APPLE_II_CHARS"),
    );
    let mut card = DiskII::new(
        bytes("EMU198X_DISK_II_P5"),
        bytes("EMU198X_DISK_II_P6"),
        MASTER_HZ as u32,
    )
    .expect("card");
    card.insert(0, disk::mount(image, writable).expect("mount"))
        .expect("drive");
    m.install_disk_ii(card);
    m.reset();
    advance(&mut m, 4_000_000);
    type_keys(&mut m, b"C600G\r");
    advance(&mut m, MASTER_HZ * 15);
    m
}
fn image(name: &str) -> Woz {
    Woz::parse(&std::fs::read(std::env::var(name).expect(name)).expect("read image"))
        .expect("valid WOZ2")
}
#[test]
#[ignore = "external ROMs and authored diagnostic WOZ; see verification.md"]
fn slot_boot_executes_an_authored_sector_and_prints_through_the_monitor() {
    let image = image("EMU198X_APPLE_II_DIAGNOSTIC");
    let m = boot(&image, false);
    assert_eq!(
        m.peek(0x300),
        0x42,
        "marker is written by the boot sector, never injected"
    );
    assert!(m.text_rows().join("\n").contains("DISK II BOOT OK"));
    assert!(m.disk_ii().expect("card").status().bit_reads[0] > 1000);
    assert_eq!(m.disk_ii().expect("card").status().bit_writes, [0; 2]);
}
#[test]
#[ignore = "external original ROMs and DOS 3.3 System Master WOZ; see verification.md"]
fn dos_reads_protects_archive_and_reloads_a_saved_work_copy_in_a_new_machine() {
    let source = image("EMU198X_APPLE_II_DOS");
    let before = source.to_bytes();
    let mut archive = boot(&source, false);
    assert!(archive.text_rows().join("\n").contains("DOS VERSION 3.3"));
    type_keys(&mut archive, b"10 PRINT 6*7\r20 END\rSAVE DISKTEST\r");
    advance(&mut archive, MASTER_HZ * 2);
    assert!(archive.text_rows().join("\n").contains("WRITE PROTECTED"));
    assert_eq!(archive.disk_ii().expect("card").status().bit_writes, [0; 2]);
    type_keys(&mut archive, b"CATALOG\r");
    advance(&mut archive, MASTER_HZ * 2);
    assert!(archive.text_rows().join("\n").contains("HELLO"));
    let mut work = source.clone();
    work.set_write_protected(false);
    let mut m = boot(&work, true);
    type_keys(&mut m, b"10 PRINT 6*7\r20 END\rSAVE DISKTEST\r");
    advance(&mut m, MASTER_HZ * 5);
    assert!(!m.text_rows().join("\n").contains("I/O ERROR"));
    assert!(m.disk_ii().expect("card").status().bit_writes[0] > 1000);
    let exported =
        disk::export(&work, m.disk_ii().and_then(|c| c.disk(0)).expect("disk")).expect("export");
    assert_ne!(exported, work.to_bytes());
    let saved = Woz::parse(&exported).expect("export checksum and descriptors");
    let mut fresh = boot(&saved, false);
    type_keys(&mut fresh, b"LOAD DISKTEST\rRUN\r");
    advance(&mut fresh, MASTER_HZ * 2);
    println!("Fresh machine:\n{}", fresh.text_rows().join("\n"));
    assert!(fresh.text_rows().iter().any(|row| row.trim() == "42"));
    assert_eq!(source.to_bytes(), before, "immutable archive");
}
