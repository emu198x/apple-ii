//! Presentation decoder, separate from the board's serial PICTURE signal.
//!
//! Four-bit square filter and nominal palette adapted from MAME's
//! `src/mame/apple/apple2video.cpp`, composite colour mode 2, R. Belmont,
//! BSD-3-Clause. See THIRD_PARTY_NOTICES.md and docs/hardware.md.
//! This is a reference-matching digital presentation, not an analogue TV model.

pub(crate) const PALETTE: [u32; 16] = [
    0xff00_0000,
    0xffa7_0b40,
    0xff40_1cf7,
    0xffe6_28ff,
    0xff00_7440,
    0xff80_8080,
    0xff19_90ff,
    0xffbf_9cff,
    0xff40_6300,
    0xffe6_6f00,
    0xff80_8080,
    0xffff_8bbf,
    0xff19_d700,
    0xffbf_e308,
    0xff58_f4bf,
    0xffff_ffff,
];

/// Window bits, low to high: previous, current, next, next-after-next sample.
/// The horizontal position establishes the colour-reference phase; it must
/// continue across bytes and restart only at the next scanline.
pub(crate) fn index(window: u8, x: usize) -> usize {
    let shift = (x + 3) & 3;
    let nibble = u32::from(window & 15);
    (((nibble << shift) | (nibble >> (4 - shift))) & 15) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_phase_and_window_matches_the_reference_fixture() {
        let fixture = include_str!("../tests/fixtures/mame-square-filter.csv");
        let mut checked = 0;
        for row in fixture.lines().filter(|row| !row.starts_with('#')) {
            let fields: Vec<usize> = row
                .split(',')
                .map(|v| v.parse().expect("fixture integer"))
                .collect();
            assert_eq!(fields.len(), 3);
            assert_eq!(index(fields[0] as u8, fields[1]), fields[2], "{row}");
            checked += 1;
        }
        assert_eq!(
            checked,
            16 * 4,
            "the reference generator must produce every window and phase"
        );
    }
}
