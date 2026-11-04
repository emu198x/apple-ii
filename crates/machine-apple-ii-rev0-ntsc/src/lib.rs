//! Original NTSC revision-0 Apple II: 48 KB, text/graphics, keyboard, speaker.
//!
//! One call to [`AppleII::tick`] advances the 14M crystal once. All CPU,
//! scanner, flash and audio timing derives from that clock. The CPU exposes
//! its bus at PHI0's rising edge; the access completes at its RAM latch phase.
//! No instructions are fast-forwarded and no firmware entry points are trapped.
//!
//! Reset is an explicit key action. An optional Disk II occupies slot 6. Cassette
//! transport and game peripherals are outside this slice. See the repository
//! README for the qualified surface and external-firmware test command.

mod colour;
pub mod disk;
mod timing;
mod video;

pub use timing::MASTER_HZ;
pub use video::{COLOUR_WIDTH, HEIGHT, WIDTH, text_address};

use emu198x_apple_disk_ii::DiskII;
use emu198x_mos_6502::M6502;
use std::error::Error;
use std::fmt;

/// Legacy framebuffer error retained for source compatibility. All original
/// board video modes now render, so [`AppleII::framebuffer`] returns `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphicsNotImplemented;

impl fmt::Display for GraphicsNotImplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("graphics output is outside the text foundation")
    }
}

impl Error for GraphicsNotImplemented {}

/// A visible CPU access. Debugger peeks never generate one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusAccess {
    pub address: u16,
    pub data: u8,
    pub read: bool,
}

/// Outputs produced on a single crystal edge. Callers own buffering, so an
/// unattended machine cannot grow an unbounded audio/trace queue.
#[derive(Default, Debug)]
pub struct TickOutput {
    pub bus: Option<BusAccess>,
    pub speaker_edge: bool,
    /// Raw bipolar speaker level averaged over each 48 kHz sample interval.
    /// No analogue speaker/filter response is claimed.
    pub audio_sample: Option<f32>,
    pub frame_complete: bool,
}

/// Original NTSC revision-0 board, with 48 KB fitted and no cards installed.
pub struct AppleII {
    cpu: M6502,
    disk_ii: Option<DiskII>,
    ram: Vec<u8>,
    rom: [u8; 8192],
    timing: timing::Timing,
    video: video::Video,
    master_clock: u64,
    instruction_boundaries: u64,
    frames: u64,
    keyboard: u8,
    speaker: bool,
    cassette_output: bool,
    annunciators: [bool; 4],
    floating_bus: u8,
    sample_phase: u64,
    sample_ticks: u32,
    speaker_high_ticks: u32,
}

impl AppleII {
    /// Install E000–FFFF ROM and 64 normalized 7×8 character bitmaps. Arrays
    /// enforce exact sizes; optional D000/D800 sockets are empty on this board.
    /// RAM starts deterministically zeroed, not as an analogue power-on model.
    /// Call [`Self::reset`] to press RESET (revision 0 has no power-on reset).
    pub fn new(rom: [u8; 8192], glyphs: [u8; 512]) -> Self {
        let mut cpu = M6502::new();
        // Park the unreset CPU; RESET releases it into the real vector sequence.
        cpu.rdy = false;
        Self {
            cpu,
            disk_ii: None,
            ram: vec![0; 0xc000],
            rom,
            timing: timing::Timing::default(),
            video: video::Video::new(glyphs),
            master_clock: 0,
            instruction_boundaries: 0,
            frames: 0,
            keyboard: 0,
            speaker: false,
            cassette_output: false,
            annunciators: [false; 4],
            floating_bus: 0,
            sample_phase: 0,
            sample_ticks: 0,
            speaker_high_ticks: 0,
        }
    }

    /// Press and release the RESET key. RAM, scanner phase and board latches
    /// survive; the 6502 runs its actual reset sequence on subsequent clocks.
    pub fn reset(&mut self) {
        if let Some(card) = self.disk_ii.as_mut() {
            card.reset();
        }
        self.cpu.rdy = true;
        self.cpu.reset();
    }

    /// Fit a Disk II card in slot 6, returning any previous card with its media.
    pub fn install_disk_ii(&mut self, card: DiskII) -> Option<DiskII> {
        self.disk_ii.replace(card)
    }
    pub fn disk_ii(&self) -> Option<&DiskII> {
        self.disk_ii.as_ref()
    }
    pub fn disk_ii_mut(&mut self) -> Option<&mut DiskII> {
        self.disk_ii.as_mut()
    }

    /// Present one encoded 7-bit keyboard character and assert its strobe.
    /// Returns false while a previous character is pending or for non-7-bit
    /// data. The host schedules key repeat; this is the encoder's output latch.
    pub fn key(&mut self, ascii: u8) -> bool {
        if ascii >= 128 || self.keyboard & 128 != 0 {
            return false;
        }
        self.keyboard = ascii | 128;
        true
    }

    pub fn key_pending(&self) -> bool {
        self.keyboard & 128 != 0
    }
    pub fn master_clock(&self) -> u64 {
        self.master_clock
    }
    pub fn frame_count(&self) -> u64 {
        self.frames
    }
    pub fn cpu(&self) -> &M6502 {
        &self.cpu
    }

    /// Debugger write to fitted RAM only. ROM, I/O and card space are ignored.
    /// Does not advance time or select any hardware switch.
    pub fn poke(&mut self, address: u16, value: u8) {
        if let Some(byte) = self.ram.get_mut(usize::from(address)) {
            *byte = value;
        }
    }

    /// Completed CPU boundaries observed after their bus access has latched.
    /// Includes completion of RESET; remains monotonic across RESET key presses.
    pub fn instruction_boundary_count(&self) -> u64 {
        self.instruction_boundaries
    }
    pub fn speaker_level(&self) -> bool {
        self.speaker
    }

    /// No side effects: safe for debugger inspection and screen extraction.
    pub fn peek(&self, address: u16) -> u8 {
        match address {
            0x0000..=0xbfff => self.ram[usize::from(address)],
            0xc000..=0xc00f => self.keyboard,
            0xc600..=0xc6ff if self.disk_ii.is_some() => self
                .disk_ii
                .as_ref()
                .map_or(self.floating_bus, |card| card.rom(address as u8)),
            0xc0e0..=0xc0ef => self
                .disk_ii
                .as_ref()
                .and_then(|card| card.data(address as u8))
                .unwrap_or(self.floating_bus),
            0xe000..=0xffff => self.rom[usize::from(address - 0xe000)],
            _ => self.floating_bus,
        }
    }

    /// Return the 280×192 monochrome raster in any video mode. Each pixel
    /// averages two 14M PICTURE samples; text and hi-res dots remain sharp.
    /// The Result return type is retained for source compatibility.
    pub fn framebuffer(&self) -> Result<&[u32], GraphicsNotImplemented> {
        Ok(&self.video.framebuffer)
    }

    /// Return the 560×192 ARGB32 colour presentation of the live PICTURE
    /// signal, using the reference four-bit square filter and nominal palette.
    /// Revision 0 has colour burst in text as well as graphics.
    pub fn colour_framebuffer(&self) -> &[u32] {
        &self.video.colour_framebuffer
    }

    /// Current text RAM, decoded into 24 rows of 40 printable characters.
    /// This inspection does not consume keyboard strobes or advance the beam.
    pub fn text_rows(&self) -> Vec<String> {
        (0..24)
            .map(|row| {
                (0..40)
                    .map(|col| {
                        let address = text_address(self.video.page2, col, row)
                            .expect("valid text coordinates");
                        let code = self.peek(address) & 63;
                        char::from(if code < 32 { code + 64 } else { code })
                    })
                    .collect()
            })
            .collect()
    }

    /// One oscillator edge, including both RAM bus owners at their own phases.
    pub fn tick(&mut self) -> TickOutput {
        let mut output = TickOutput::default();
        let events = self.timing.tick();
        if events.video_latch {
            let text_line = self.video.text || (self.video.mixed && self.timing.v & 0xa0 == 0xa0);
            let address = self
                .timing
                .address(self.video.page2, self.video.hires && !text_line);
            self.floating_bus = self.ram[usize::from(address)];
            // The real flash oscillator is analogue (~2 Hz, not crystal
            // divided). This deterministic 2 Hz presentation is unqualified
            // as an analogue oscillator model; see docs/hardware.md.
            let flash = (self.master_clock / (MASTER_HZ / 4)) & 1 != 0;
            self.video
                .latch(self.floating_bus, self.timing.visible_cell(), flash);
        }
        if events.scan_advance && self.timing.advance_scanner() {
            self.frames += 1;
            output.frame_complete = true;
        }
        if events.cpu_start {
            self.cpu.tick();
            if self.cpu.rdy
                && (0xc0e0..=0xc0ef).contains(&self.cpu.addr)
                && let Some(card) = self.disk_ii.as_mut()
            {
                card.select(self.cpu.addr as u8, (!self.cpu.rw).then_some(self.cpu.data));
            }
        }
        if let Some(card) = self.disk_ii.as_mut() {
            card.tick(events.disk_clock);
        }
        if events.cpu_latch && self.cpu.rdy {
            let address = self.cpu.addr;
            let read = self.cpu.rw;
            let data = if read {
                self.peek(address)
            } else {
                self.cpu.data
            };
            let old_speaker = self.speaker;
            self.access(address, read, data);
            output.speaker_edge = old_speaker != self.speaker;
            if read {
                self.cpu.data_in = data;
            }
            output.bus = Some(BusAccess {
                address,
                data,
                read,
            });
            if self.cpu.instruction_complete() && !self.cpu.halted {
                self.instruction_boundaries = self.instruction_boundaries.saturating_add(1);
            }
        }
        self.video.tick();
        self.sample_ticks += 1;
        self.speaker_high_ticks += u32::from(self.speaker);
        self.sample_phase += 48_000;
        if self.sample_phase >= MASTER_HZ {
            self.sample_phase -= MASTER_HZ;
            output.audio_sample =
                Some(self.speaker_high_ticks as f32 / self.sample_ticks as f32 * 0.5 - 0.25);
            self.sample_ticks = 0;
            self.speaker_high_ticks = 0;
        }
        self.master_clock += 1;
        output
    }

    fn access(&mut self, address: u16, read: bool, data: u8) {
        match address {
            0x0000..=0xbfff if !read => self.ram[usize::from(address)] = data,
            0xc010..=0xc01f => self.keyboard &= 127,
            0xc020..=0xc02f => self.cassette_output = !self.cassette_output,
            0xc030..=0xc03f => self.speaker = !self.speaker,
            0xc050 => self.video.text = false,
            0xc051 => self.video.text = true,
            0xc052 => self.video.mixed = false,
            0xc053 => self.video.mixed = true,
            0xc054 => self.video.page2 = false,
            0xc055 => self.video.page2 = true,
            0xc056 => self.video.hires = false,
            0xc057 => self.video.hires = true,
            0xc058..=0xc05f => {
                self.annunciators[usize::from((address - 0xc058) / 2)] = address & 1 != 0
            }
            _ => {} // ROM, absent sockets/cards and unconnected outputs.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphics_ram_writes_do_not_repaint_already_scanned_pixels() {
        for hires in [false, true] {
            let mut machine = AppleII::new([0; 8192], [0; 512]);
            machine.access(0xc050, true, 0);
            if hires {
                machine.access(0xc057, true, 0);
            }
            let address = if hires { 0x2000 } else { 0x400 };
            machine.access(address, false, 0xff);
            let mut fetched = false;
            for _ in 0..2000 {
                let cell = machine.timing.visible_cell();
                machine.tick();
                if cell == Some((0, 0)) && machine.timing.h == 89 {
                    fetched = true;
                    break;
                }
            }
            assert!(fetched);
            machine.access(address, false, 0);
            for _ in 0..14 {
                machine.tick();
            }
            assert_eq!(machine.colour_framebuffer()[6], 0xffffffff);
            assert_eq!(machine.framebuffer().expect("graphics")[3], 0xffffffff);
            for _ in 0..240_000 {
                machine.tick();
            }
            assert_eq!(machine.colour_framebuffer()[6], 0xff000000);
        }
    }

    #[test]
    fn scanner_holds_fetched_byte_until_next_pass_despite_later_ram_write() {
        let mut glyphs = [0; 512];
        glyphs[8..16].fill(0x7f);
        let mut machine = AppleII::new([0; 8192], glyphs);
        machine.access(0x0400, false, 0xc1);
        let mut fetched = false;
        for _ in 0..2000 {
            let address = machine.timing.address(false, false);
            let cell = machine.timing.visible_cell();
            machine.tick();
            if address == 0x0400 && cell == Some((0, 0)) && machine.timing.h == 89 {
                fetched = true;
                break;
            }
        }
        assert!(fetched);
        machine.access(0x0400, false, 0xa0); // space, after this scan's fetch
        for _ in 0..14 {
            machine.tick();
        }
        assert!(
            machine.framebuffer().expect("text")[..7]
                .iter()
                .all(|&p| p == 0xffffffff)
        );
        for _ in 0..240_000 {
            machine.tick();
        }
        assert!(
            machine.framebuffer().expect("text")[..7]
                .iter()
                .all(|&p| p == 0xff000000)
        );
    }
}
