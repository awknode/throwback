//! ORIGINAL strip: what your own (dry) sound runs through when PARALLEL is off.
//! Same tone controls as a lane, no repeats: DRIVE -> LOW CUT -> HIGH CUT ->
//! WIDTH -> PAN -> OUTPUT. Zero latency, so the dry never moves in time.
//! At its defaults every stage is skipped, so INSERT sounds exactly like PARALLEL.

use crate::dsp::*;
use crate::params::*;

pub struct Orig {
    lc: [Svf; 2],
    hc: [Svf; 2],
    sr: f32,
    lc_hz: f32,
    hc_hz: f32,
}

/// LOW CUT at its bottom / HIGH CUT at its top = filter out of the path.
const LC_OFF: f32 = 20.5;
const HC_OFF: f32 = 19_900.0;

impl Orig {
    pub fn new(sr: f32) -> Self {
        Self { lc: Default::default(), hc: Default::default(), sr, lc_hz: -1.0, hc_hz: -1.0 }
    }

    pub fn clear(&mut self) {
        for f in self.lc.iter_mut().chain(self.hc.iter_mut()) {
            f.clear();
        }
    }

    /// Once per block: filter corners.
    pub fn setup(&mut self, p: &TbParams) {
        let (lc, hc) = (p.o_lc.value(), p.o_hc.value());
        if lc != self.lc_hz {
            self.lc_hz = lc;
            for f in self.lc.iter_mut() {
                f.set(lc, 0.707, self.sr);
            }
        }
        if hc != self.hc_hz {
            self.hc_hz = hc;
            for f in self.hc.iter_mut() {
                f.set(hc, 0.707, self.sr);
            }
        }
    }

    /// One stereo sample. Call every sample (keeps the smoothers moving) even
    /// when PARALLEL is on; the result is only used in INSERT.
    #[inline]
    pub fn tick(&mut self, p: &TbParams, l: f32, r: f32) -> (f32, f32) {
        let drive = p.o_drive.smoothed.next();
        let width = p.o_width.smoothed.next();
        let pan = p.o_pan.smoothed.next();
        let out = level_gain(p.o_out.smoothed.next());
        let mut x = [l, r];
        if drive > 0.01 {
            let g = 10f32.powf(drive / 20.0);
            let norm = g.powf(-0.75);
            for v in x.iter_mut() {
                *v = ftanh(*v * g) * norm;
            }
        }
        if self.lc_hz > LC_OFF {
            for ch in 0..2 {
                x[ch] = self.lc[ch].tick(x[ch]).hp;
            }
        }
        if self.hc_hz < HC_OFF {
            for ch in 0..2 {
                x[ch] = self.hc[ch].tick(x[ch]).lp;
            }
        }
        let (mut l, mut r) = (x[0], x[1]);
        if (width - 1.0).abs() > 1e-4 {
            let m = 0.5 * (l + r);
            let sd = 0.5 * (l - r) * width;
            l = m + sd;
            r = m - sd;
        }
        let pz = pan.clamp(-1.0, 1.0);
        let (gl, gr) = ((1.0 - pz).min(1.0), (1.0 + pz).min(1.0));
        (l * gl * out, r * gr * out)
    }
}
