//! Headless interactive console; the ROM handles every typed command.
use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, COLOUR_WIDTH, HEIGHT, MASTER_HZ, WIDTH};
use std::io::{self, BufRead, Write};

fn advance(machine: &mut AppleII, ticks: usize) {
    for _ in 0..ticks {
        machine.tick();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let rom = std::fs::read(
        args.next()
            .ok_or("usage: boot ROM CHARS [output.ppm [--colour]]")?,
    )?;
    let chars = std::fs::read(
        args.next()
            .ok_or("usage: boot ROM CHARS [output.ppm [--colour]]")?,
    )?;
    let output = args.next();
    let colour = match args.next().as_deref() {
        None => false,
        Some("--colour") => true,
        Some(_) => return Err("expected --colour after output path".into()),
    };
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
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
    if let Some(path) = output {
        // create_new prevents overwriting any loaded source or existing file.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        let (width, pixels) = if colour {
            (COLOUR_WIDTH, machine.colour_framebuffer())
        } else {
            (WIDTH, machine.framebuffer()?)
        };
        write!(file, "P6\n{width} {HEIGHT}\n255\n")?;
        for &pixel in pixels {
            file.write_all(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])?;
        }
    }
    Ok(())
}

fn send(machine: &mut AppleII, key: u8) -> Result<(), Box<dyn std::error::Error>> {
    if !machine.key(key) {
        return Err("keyboard latch occupied or non-ASCII input".into());
    }
    // GR clears the screen through ROM plotting loops and exceeds 70 ms.
    // Allow up to one emulated second for a command to accept the next key.
    for _ in 0..MASTER_HZ {
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
