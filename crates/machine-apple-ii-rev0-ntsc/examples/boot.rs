//! Headless interactive console; the ROM handles every typed command.
use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, HEIGHT, WIDTH};
use std::io::{self, BufRead, Write};

fn advance(machine: &mut AppleII, ticks: usize) {
    for _ in 0..ticks {
        machine.tick();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let rom = std::fs::read(args.next().ok_or("usage: boot ROM CHARS [output.ppm]")?)?;
    let chars = std::fs::read(args.next().ok_or("usage: boot ROM CHARS [output.ppm]")?)?;
    let mut machine = AppleII::new(
        rom.try_into()
            .map_err(|_| "ROM must be exactly 8192 bytes")?,
        chars
            .try_into()
            .map_err(|_| "character patterns must be exactly 512 bytes")?,
    );
    machine.reset();
    advance(&mut machine, 4_000_000);
    for key in [2, 13] {
        send(&mut machine, key)?;
    }
    print_screen(&machine);
    for line in io::stdin().lock().lines() {
        for key in line?.bytes().chain([13]) {
            send(&mut machine, key.to_ascii_uppercase())?;
        }
        advance(&mut machine, 500_000);
        print_screen(&machine);
    }
    if let Some(path) = args.next() {
        // create_new prevents overwriting any loaded source or existing file.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        write!(file, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
        for &pixel in machine.framebuffer()? {
            file.write_all(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])?;
        }
    }
    Ok(())
}

fn send(machine: &mut AppleII, key: u8) -> Result<(), Box<dyn std::error::Error>> {
    if !machine.key(key) {
        return Err("keyboard latch occupied or non-ASCII input".into());
    }
    for _ in 0..1_000_000 {
        machine.tick();
        if !machine.key_pending() {
            advance(machine, 1_000_000);
            return Ok(());
        }
    }
    Err("firmware did not acknowledge key within the bounded wait".into())
}

fn print_screen(machine: &AppleII) {
    println!("{}", machine.text_rows().join("\n"));
}
