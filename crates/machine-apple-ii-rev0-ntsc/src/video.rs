//! Digital monochrome text, not a composite-video decoder.

/// Visible text raster width in 7 MHz dots.
pub const WIDTH: usize = 280;
/// Visible raster height.
pub const HEIGHT: usize = 192;

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
    pub glyphs: [u8; 512],
    pub text: bool,
    pub mixed: bool,
    pub page2: bool,
    pub hires: bool,
    // One live scanner fetch drives seven serial dots, one per 7M period.
    shift: u8,
    position: Option<(usize, usize)>,
    dot_clock: u8,
    dot: usize,
}

impl Video {
    pub fn new(glyphs: [u8; 512]) -> Self {
        Self {
            framebuffer: vec![0xff00_0000; WIDTH * HEIGHT],
            glyphs,
            // A deterministic initial latch state, selectable by software;
            // revision 0 has no power-on reset circuit for these latches.
            text: true,
            mixed: false,
            page2: false,
            hires: false,
            shift: 0,
            position: None,
            dot_clock: 0,
            dot: 0,
        }
    }

    pub fn latch(&mut self, code: u8, cell: Option<(usize, usize)>, flash: bool) {
        self.position = cell.filter(|&(_, y)| self.text || (self.mixed && y >= 160));
        self.dot = 0;
        self.dot_clock = 0;
        let row = cell.map_or(0, |(_, y)| y & 7);
        let glyph = self.glyphs[usize::from(code & 63) * 8 + row];
        let inverse = code & 0xc0 == 0 || (code & 0xc0 == 0x40 && flash);
        // Input is normalized visual bitmap: bit 6 is the leftmost dot,
        // 1 is ink. Accepting a caller's patterns avoids embedding firmware.
        self.shift = if inverse { glyph ^ 0x7f } else { glyph };
    }

    pub fn tick(&mut self) {
        if self.dot_clock == 0 && self.dot < 7 {
            if let Some((x, y)) = self.position {
                self.framebuffer[y * WIDTH + x * 7 + self.dot] = if self.shift & 0x40 != 0 {
                    0xffff_ffff
                } else {
                    0xff00_0000
                };
            }
            self.shift <<= 1;
            self.dot += 1;
        }
        self.dot_clock ^= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
