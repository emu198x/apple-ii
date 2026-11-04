//! Master oscillator and shared-memory timing. See docs/hardware.md.
//!
//! Equations follow the vendored MiSTer timing_generator.vhd's RAS/AX/CAS/Q3/
//! PHI0 network, with only the original-II signals. Scanner counters are the
//! discrete original-II sequence, including H's duplicated low-six-bit zero.

/// Crystal periods per second (14M).
pub const MASTER_HZ: u64 = 14_318_180;

#[derive(Default)]
pub(crate) struct Events {
    pub video_latch: bool,
    pub cpu_start: bool,
    pub cpu_latch: bool,
    pub scan_advance: bool,
}

pub(crate) struct Timing {
    color_ref: bool,
    clk7: bool,
    ras: bool,
    ax: bool,
    cas: bool,
    q3: bool,
    phi0: bool,
    pub h: u8,
    pub v: u16,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            color_ref: false,
            clk7: false,
            ras: false,
            ax: false,
            cas: false,
            q3: false,
            phi0: false,
            h: 0,
            // Start at the first visible line; this is an arbitrary oscillator
            // phase, not a claim about analogue power-on latch values.
            v: 0x100,
        }
    }
}

impl Timing {
    pub fn tick(&mut self) -> Events {
        let Self {
            color_ref: c,
            clk7: k,
            ras: r,
            ax: a,
            cas: s,
            q3: q,
            phi0: p,
            h,
            ..
        } = *self;
        let odd = h & 1 != 0;
        let ras = !(q || (!r && !a) || (!r && c && odd && p) || (!r && !k && odd && p));
        let ax = !(!r && q || !a && q);
        let cas = !(!a || (!s && !r));
        let q3 = !((!a && !p && !k) || (!a && p && k) || (!q && !r));
        let phi0 = !((p && r && !q) || (!p && !r) || (!p && q));
        let latch = a && !s && r && !q;
        let events = Events {
            video_latch: latch && !p,
            cpu_start: !p && phi0,
            cpu_latch: latch && p,
            scan_advance: r && !p && !q,
        };
        self.color_ref = k ^ c;
        self.clk7 = !k;
        self.ras = ras;
        self.ax = ax;
        self.cas = cas;
        self.q3 = q3;
        self.phi0 = phi0;
        events
    }

    pub fn advance_scanner(&mut self) -> bool {
        let mut new_frame = false;
        if self.h == 127 {
            self.v = if self.v == 511 { 250 } else { self.v + 1 };
            new_frame = self.v == 256;
        }
        self.h = if self.h == 0 { 64 } else { (self.h + 1) & 127 };
        new_frame
    }

    /// The DRAM scanner runs through blanking as well as visible memory.
    /// TTL adder mapping: Sather ch.5; MiSTer VIDEO_ADDRESS; MAME
    /// a2_video_device::scanner_address. Original-II HBL asserts A12.
    pub fn address(&self, page2: bool, hires: bool) -> u16 {
        let h = u16::from(self.h & 63);
        let v = self.v;
        let sum = (13 + (h >> 3) + ((v >> 7) & 1) * 10 + ((v >> 6) & 1) * 5) & 15;
        let low = (h & 7) | (sum << 3) | (((v >> 3) & 7) << 7);
        if hires {
            low | ((v & 7) << 10) | if page2 { 0x4000 } else { 0x2000 }
        } else {
            low | if page2 { 0x0800 } else { 0x0400 } | if self.h < 88 { 0x1000 } else { 0 }
        }
    }

    pub fn visible_cell(&self) -> Option<(usize, usize)> {
        if self.h >= 88 && (256..448).contains(&self.v) {
            Some((usize::from(self.h - 88), usize::from(self.v - 256)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_network_has_65_cpu_cycles_and_912_ticks_per_line() {
        let mut timing = Timing::default();
        let mut starts = Vec::new();
        let mut line_edges = Vec::new();
        let mut frame_edges = Vec::new();
        for tick in 0..500_000 {
            let events = timing.tick();
            if events.cpu_start {
                starts.push(tick);
            }
            if events.scan_advance {
                if timing.h == 127 {
                    line_edges.push(tick);
                }
                if timing.advance_scanner() {
                    frame_edges.push(tick);
                }
            }
        }
        assert!(line_edges.len() > 500);
        assert!(line_edges.windows(2).all(|w| w[1] - w[0] == 912));
        assert_eq!(frame_edges.len(), 2);
        assert_eq!(frame_edges[1] - frame_edges[0], 912 * 262);
        for window in line_edges.windows(2).take(10) {
            let cycles: Vec<_> = starts
                .iter()
                .copied()
                .filter(|&t| t >= window[0] && t < window[1])
                .collect();
            assert_eq!(cycles.len(), 65);
            let lengths: Vec<_> = cycles.windows(2).map(|w| w[1] - w[0]).collect();
            assert!(lengths.iter().all(|&period| period == 14 || period == 16));
            assert_eq!(lengths.iter().filter(|&&period| period == 16).count(), 1);
        }
    }

    #[test]
    fn scanner_matches_text_coordinates_and_scans_blank_memory() {
        let mut timing = Timing::default();
        let mut visible = 0;
        for _ in 0..65 * 262 {
            if let Some((x, y)) = timing.visible_cell() {
                visible += 1;
                for page2 in [false, true] {
                    assert_eq!(
                        Some(timing.address(page2, false)),
                        crate::text_address(page2, x, y / 8)
                    );
                }
            } else if timing.h < 88 {
                assert_ne!(
                    timing.address(false, false) & 0x1000,
                    0,
                    "original-II HBL sets A12, unlike the IIe"
                );
            }
            timing.advance_scanner();
        }
        assert_eq!(visible, 40 * 192);
        assert_eq!((timing.h, timing.v), (0, 256));
    }

    #[test]
    fn each_video_latch_precedes_its_cpu_latch() {
        let mut timing = Timing::default();
        let mut video_tick = None;
        let mut cpu_accesses = 0;
        for tick in 0..5000 {
            let events = timing.tick();
            if events.video_latch {
                assert!(video_tick.replace(tick).is_none());
            }
            if events.cpu_latch {
                let earlier = video_tick.take().expect("video owns the first RAM phase");
                assert!(matches!(tick - earlier, 6 | 7 | 9));
                cpu_accesses += 1;
            }
            if events.scan_advance {
                timing.advance_scanner();
            }
        }
        assert!(cpu_accesses > 300);
    }
}
