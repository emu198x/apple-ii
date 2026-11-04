//! Headless external-ROM disk boot; all bootstrap bytes are read by the CPU.
use emu198x_apple_disk_ii::DiskII;
use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, MASTER_HZ, disk};
use format198x_apple_ii_woz::Woz;
use std::io::{self, BufRead};
fn advance(m: &mut AppleII, ticks: u64) {
    for _ in 0..ticks {
        m.tick();
    }
}
fn send(m: &mut AppleII, key: u8) -> Result<(), Box<dyn std::error::Error>> {
    if !m.key(key) {
        return Err("keyboard occupied or non-ASCII input".into());
    }
    for _ in 0..MASTER_HZ * 30 {
        m.tick();
        if !m.key_pending() {
            advance(m, 1_000_000);
            return Ok(());
        }
    }
    Err(format!("keyboard not acknowledged, PC={:04x}", m.cpu().regs.pc).into())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() != 5 && !(a.len() == 7 && a[5] == "--work-copy") {
        return Err("usage: disk_boot ROM CHARS P5 P6 DISK.woz [--work-copy NEW.woz]".into());
    }
    let rom = std::fs::read(&a[0])?
        .try_into()
        .map_err(|_| "8192 ROM bytes required")?;
    let chars = std::fs::read(&a[1])?
        .try_into()
        .map_err(|_| "512 character bytes required")?;
    let p5 = std::fs::read(&a[2])?
        .try_into()
        .map_err(|_| "256 P5 bytes required")?;
    let p6 = std::fs::read(&a[3])?
        .try_into()
        .map_err(|_| "256 P6 bytes required")?;
    let mut image = Woz::parse(&std::fs::read(&a[4])?)?;
    if a.len() == 7 {
        image.set_write_protected(false);
    }
    let mut card = DiskII::new(p5, p6, MASTER_HZ as u32)?;
    card.insert(0, disk::mount(&image, a.len() == 7)?)?;
    let mut m = AppleII::new(rom, chars);
    m.install_disk_ii(card);
    m.reset();
    advance(&mut m, 4_000_000);
    for key in b"C600G\r" {
        send(&mut m, *key)?;
    }
    advance(&mut m, MASTER_HZ * 15);
    println!(
        "Boot PC={:04x} status={:?}\n{}",
        m.cpu().regs.pc,
        m.disk_ii().map(DiskII::status),
        m.text_rows().join("\n")
    );
    for line in io::stdin().lock().lines() {
        for key in line?.bytes().chain([13]) {
            send(&mut m, key.to_ascii_uppercase())?;
        }
        advance(&mut m, MASTER_HZ * 2);
        println!("{}", m.text_rows().join("\n"));
    }
    if a.len() == 7 {
        for _ in 0..MASTER_HZ * 30 {
            if m.disk_ii().is_none_or(|c| !c.status().motor_on) {
                break;
            }
            m.tick();
        }
        if m.disk_ii().is_some_and(|c| c.status().motor_on) {
            return Err("disk motor still active after 30 seconds; export deferred".into());
        }
        let data = disk::export(
            &image,
            m.disk_ii().and_then(|c| c.disk(0)).ok_or("drive empty")?,
        )?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&a[6])?;
        file.write_all(&data)?;
    }
    Ok(())
}
