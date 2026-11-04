//! Revision-0 PICTURE serialization, with monochrome and colour presentations.

use crate::colour;

/// Visible monochrome raster width in 7 MHz dots.
pub const WIDTH: usize = 280;
/// Visible raster height.
pub const HEIGHT: usize = 192;

/// Colour raster width: one sample per 14M tick, twice the 7M dot width.
pub const COLOUR_WIDTH: usize = WIDTH * 2;

/// Address of a character in either 40-column text page.
/// Returns None for coordinates outside 40 × 24.
pub fn text_address(page2: bool, column: usize, row: usize) -> Option<u16> {
    if column >= 40 || row >= 24 {
        return None;
    }
    Some(
        (if page2 { 0x0800 } else { 0x0400 }) + ((row & 7) * 128 + (row >> 3) * 40 + column) as u16,
    )
}

pub(crate) struct Video {
    pub framebuffer: Vec<u32>,
    pub colour_framebuffer: Vec<u32>,
    pub glyphs: [u8; 512],
    pub text: bool,
    pub mixed: bool,
    pub page2: bool,
    pub hires: bool,
    // One live fetch drives fourteen serial PICTURE samples, LSB first.
    shift: u16,
    position: Option<(usize, usize)>,
    sample: usize,
    // Last three PICTURE samples, oldest in bit 0. The colour decoder needs
    // two future samples, obtained from the live shifter rather than RAM.
    history: u8,
    active_line: Option<usize>,
    blank_samples: usize,
    monochrome_first: u8,
}

impl Video {
    pub fn new(glyphs: [u8; 512]) -> Self {
        Self {
            framebuffer: vec![0xff00_0000; WIDTH * HEIGHT],
            colour_framebuffer: vec![0xff00_0000; COLOUR_WIDTH * HEIGHT],
            glyphs,
            // A deterministic initial latch state, selectable by software;
            // revision 0 has no power-on reset circuit for these latches.
            text: true,
            mixed: false,
            page2: false,
            hires: false,
            shift: 0,
            position: None,
            sample: 0,
            history: 0,
            active_line: None,
            blank_samples: 0,
            monochrome_first: 0,
        }
    }

    pub fn latch(&mut self, code: u8, cell: Option<(usize, usize)>, flash: bool) {
        self.position = cell;
        self.sample = 0;
        let Some((column, y)) = cell else {
            return;
        };
        if column == 0 {
            self.history = 0;
            self.active_line = Some(y);
            self.blank_samples = 0;
        }
        if self.text || (self.mixed && y >= 160) {
            let glyph = self.glyphs[usize::from(code & 63) * 8 + (y & 7)];
            let inverse = code & 0xc0 == 0 || (code & 0xc0 == 0x40 && flash);
            let pattern = if inverse { glyph ^ 0x7f } else { glyph };
            // Caller-supplied text patterns put the leftmost dot in bit 6.
            self.shift = double_dots(pattern.reverse_bits() >> 1);
        } else if self.hires {
            // Revision 0 has no D7 delay flip-flop. Only D0–D6 reach PICTURE.
            self.shift = double_dots(code & 0x7f);
        } else {
            let nibble = if y & 4 == 0 { code & 15 } else { code >> 4 };
            self.shift = 0;
            for tick in 0..14 {
                // Original dual four-bit circulation: odd bytes select Q2,
                // compensating for 3.5 colour-reference periods per byte.
                let bit = (tick + (column & 1) * 2) & 3;
                self.shift |= u16::from((nibble >> bit) & 1) << tick;
            }
        }
    }

    pub fn tick(&mut self) {
        if let Some((column, y)) = self.position {
            if self.sample < 14 {
                let bit = (self.shift & 1) as u8;
                let x = column * 14 + self.sample;
                self.decode_sample(bit, x, y);
                if x & 1 == 0 {
                    self.monochrome_first = bit;
                } else {
                    let luma = u32::from(self.monochrome_first + bit) * 255 / 2;
                    self.framebuffer[y * WIDTH + x / 2] = 0xff00_0000 | (luma * 0x0001_0101);
                }
                self.shift >>= 1;
                self.sample += 1;
            }
        } else if let Some(y) = self.active_line {
            // Actual blanking samples provide the right-edge decoder context.
            self.decode_sample(0, COLOUR_WIDTH + self.blank_samples, y);
            self.blank_samples += 1;
            if self.blank_samples == 2 {
                self.active_line = None;
            }
        }
    }

    fn decode_sample(&mut self, bit: u8, x: usize, y: usize) {
        let window = self.history | (bit << 3);
        if let Some(pixel) = x.checked_sub(2).filter(|&pixel| pixel < COLOUR_WIDTH) {
            self.colour_framebuffer[y * COLOUR_WIDTH + pixel] =
                colour::PALETTE[colour::index(window, pixel)];
        }
        self.history = (window >> 1) & 7;
    }
}

fn double_dots(pattern: u8) -> u16 {
    let mut result = 0;
    for bit in 0..7 {
        result |= (u16::from((pattern >> bit) & 1) * 3) << (bit * 2);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(video: &mut Video, code: u8, column: usize, y: usize) {
        video.latch(code, Some((column, y)), false);
        for _ in 0..14 {
            video.tick();
        }
    }

    #[test]
    fn streaming_decoder_matches_reference_across_bytes_and_right_blanking() {
        let mut video = Video::new([0; 512]);
        let words = [0x0003, 0x3000, 0x0003, 0x1555, 0x2aaa, 0x3333, 0, 0x3fff];
        for column in 0..40 {
            video.latch(0, Some((column, 0)), false);
            video.shift = words[column % words.len()];
            for _ in 0..14 {
                video.tick();
            }
        }
        video.latch(0, None, false);
        video.tick();
        video.tick();
        video.tick(); // idle blanking must stop after the two context samples
        let mut checked = 0;
        for row in include_str!("../tests/fixtures/mame-square-filter-row.csv")
            .lines()
            .filter(|r| !r.starts_with('#'))
        {
            let (x, index) = row.split_once(',').expect("fixture pair");
            let x: usize = x.parse().expect("x");
            let index: usize = index.parse().expect("index");
            assert_eq!(video.colour_framebuffer[x], colour::PALETTE[index], "x={x}");
            checked += 1;
        }
        assert_eq!(checked, COLOUR_WIDTH);
    }

    #[test]
    fn lores_circulates_all_patterns_with_continuous_phase_and_selects_nibbles() {
        let mut video = Video::new([0; 512]);
        video.text = false;
        for nibble in 0..16u8 {
            for y in [0, 4] {
                let code = if y == 0 { nibble } else { nibble << 4 };
                cell(&mut video, code, 0, y);
                cell(&mut video, code, 1, y);
                // Interior pixels exclude the black left edge and next byte.
                assert!(
                    video.colour_framebuffer[y * COLOUR_WIDTH + 2..y * COLOUR_WIDTH + 26]
                        .iter()
                        .all(|&pixel| pixel == colour::PALETTE[usize::from(nibble)]),
                    "nibble={nibble}, y={y}"
                );
            }
        }
    }

    #[test]
    fn hires_is_lsb_first_and_d7_never_delays_revision_zero_dots() {
        let mut video = Video::new([0; 512]);
        video.text = false;
        video.hires = true;
        for code in 0..128u8 {
            cell(&mut video, code, 0, 0);
            cell(&mut video, code | 128, 0, 1);
            assert_eq!(
                &video.framebuffer[..7],
                &video.framebuffer[WIDTH..WIDTH + 7]
            );
            assert_eq!(
                &video.colour_framebuffer[..12],
                &video.colour_framebuffer[COLOUR_WIDTH..COLOUR_WIDTH + 12]
            );
            for x in 0..7 {
                assert_eq!(
                    video.framebuffer[x],
                    if code & (1 << x) != 0 {
                        0xffffffff
                    } else {
                        0xff000000
                    }
                );
            }
        }
    }

    #[test]
    fn mode_changes_affect_the_next_fetch_without_reinterpreting_latched_bits() {
        let mut video = Video::new([0; 512]);
        video.text = false;
        video.hires = true;
        video.latch(1, Some((0, 0)), false);
        video.tick();
        video.text = true;
        video.tick();
        assert_eq!(video.framebuffer[0], 0xffffffff);
        cell(&mut video, 0x80, 1, 0);
        assert!(video.framebuffer[7..14].iter().all(|&p| p == 0xff000000));
    }

    #[test]
    fn flashing_inverts_with_oscillator_level_but_normal_text_does_not() {
        let mut glyphs = [0; 512];
        glyphs[8..16].fill(0x22);
        let mut video = Video::new(glyphs);
        let mut pixels = Vec::new();
        for (code, flash) in [(0x41, false), (0x41, true), (0xc1, false), (0xc1, true)] {
            video.latch(code, Some((0, 0)), flash);
            for _ in 0..14 {
                video.tick();
            }
            pixels.push(video.framebuffer[..7].to_vec());
        }
        assert_eq!(pixels[0], pixels[2]);
        assert_eq!(pixels[2], pixels[3]);
        for (normal, inverse) in pixels[0].iter().zip(&pixels[1]) {
            assert_eq!(normal ^ inverse, 0x00ff_ffff);
        }
    }
}
