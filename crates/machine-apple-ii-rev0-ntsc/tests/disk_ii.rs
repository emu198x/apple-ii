use emu198x_apple_disk_ii::{Disk, DiskII, Track};
use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, MASTER_HZ, disk};
use format198x_apple_ii_woz::{Info, Track as WozTrack, Woz};

fn board(program: &[u8]) -> AppleII {
    let mut rom = [0xea; 8192];
    rom[..program.len()].copy_from_slice(program);
    let end = 0xe000 + program.len() as u16;
    rom[program.len()..program.len() + 3].copy_from_slice(&[0x4c, end as u8, (end >> 8) as u8]);
    rom[0x1ffc..0x1ffe].copy_from_slice(&[0, 0xe0]);
    let mut m = AppleII::new(rom, [0; 512]);
    m.reset();
    m
}
fn card() -> DiskII {
    DiskII::new([0x6a; 256], [0x0b; 256], MASTER_HZ as u32).expect("clock")
}
#[test]
fn cpu_selects_slot_six_and_reads_the_latch_after_real_sequencer_edges() {
    let mut m = board(&[
        0xad, 0xe9, 0xc0, // motor
        0xad, 0xeb, 0xc0, // drive 2
        0xa9, 0x42, 0x8d, 0xed, 0xc0, // write the held bus
        0xad, 0xec, 0xc0, 0x8d, 0, 2, // load at even address
        0xad, 0, 0xc6, 0x8d, 1, 2, // bootstrap PROM
        0x8d, 0, 0xc6, // attempted PROM write
    ]);
    assert!(m.install_disk_ii(card()).is_none());
    assert_eq!(m.peek(0xc600), 0x6a);
    let before = m.disk_ii().expect("card").status();
    for addr in 0xc0e0..=0xc0ef {
        m.peek(addr);
    }
    assert_eq!(m.disk_ii().expect("card").status(), before);
    for _ in 0..3000 {
        m.tick();
    }
    assert_eq!(m.peek(0x200), 0x42);
    assert_eq!(m.peek(0x201), 0x6a);
    assert_eq!(m.peek(0xc600), 0x6a);
    let status = m.disk_ii().expect("card").status();
    assert!(status.motor_on);
    assert_eq!(status.selected_drive, 1);
    assert!(status.bit_reads[1] > 0);
    assert_eq!(status.bit_reads[0], 0);
    m.reset();
    assert!(!m.disk_ii().expect("card").status().motor_on);
    assert_eq!(m.peek(0x200), 0x42);
    assert!(m.install_disk_ii(card()).is_some());
}
#[test]
fn empty_slot_floats_and_read_modify_write_exposes_both_disk_writes() {
    let mut m = board(&[0xee, 0xed, 0xc0]);
    for addr in [0xc0e0, 0xc0ed, 0xc600] {
        assert_eq!(m.peek(addr), 0);
    }
    m.install_disk_ii(card());
    m.disk_ii_mut().expect("card").select(9, None);
    let mut accesses = Vec::new();
    for _ in 0..2000 {
        if let Some(bus) = m.tick().bus
            && bus.address == 0xc0ed
        {
            accesses.push(bus);
        }
    }
    assert_eq!(accesses.len(), 3);
    assert!(accesses[0].read);
    assert!(!accesses[1].read && !accesses[2].read);
    assert_eq!(accesses[2].data, accesses[1].data.wrapping_add(1));
    assert!(m.disk_ii().expect("card").status().q6);
}
#[test]
fn adapter_keeps_shared_maps_source_metadata_and_write_protection() {
    let mut map = [255; 160];
    map[..3].fill(0);
    let image = Woz::from_tracks(
        Info::default(),
        map,
        &[WozTrack {
            bits: vec![0x80, 0],
            bit_count: 9,
        }],
    )
    .expect("image");
    let before = image.to_bytes();
    let work = disk::mount(&image, true).expect("mount");
    assert!(work.write_protected());
    assert_eq!(work.track_map(), &map);
    assert_eq!(disk::export(&image, &work).expect("export"), before);
    assert_eq!(image.to_bytes(), before);
    let wrong = Disk::new([255; 160], vec![], 32, false).expect("empty");
    assert!(disk::export(&image, &wrong).is_err());
    let wrong =
        Disk::new(map, vec![Track::new(vec![0], 8).expect("track")], 32, false).expect("disk");
    assert!(disk::export(&image, &wrong).is_err());
    let mut copy = image.clone();
    copy.set_write_protected(false);
    assert!(!disk::mount(&copy, true).expect("copy").write_protected());
    assert!(image.info().write_protected);
}
