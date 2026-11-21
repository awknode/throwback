//! One ThrowBack lane: a stereo delay with its own echo type, style, filter,
//! modulation, pitch, diffusion, ducking and output.
//!
//! Signal flow (per channel):
//!   in -> [pitch ONCE] -> DRIVE (oversampled) -> [diffuse BEFORE] -> line
//!   line --fb read--> loop chain (filter if IN THE LOOP, character, loop drive,
//!                     pitch EVERY REPEAT, diffuse IN THE LOOP) -> x FEEDBACK -> line
//!   line --out read (type heads: multi-head, chorus, reverse, cloud, glitch,
//!                    rhythm taps)--> out chain (same stages once more) -> duck
//!                    -> width / pan -> OUTPUT
//! Running the out read through its own copy of the chain means repeat n has
//! been through the chain exactly n times, whether the out read and the
//! feedback read sit at the same place or not (feel, swing, offsets, heads).

use crate::dsp::*;
use crate::oversample::{latency, Oversampler, MAX_FACTOR};
use crate::params::*;

pub const MAX_SECONDS: f32 = 10.5;

/// Everything worked out once per block from the lane's params.
#[derive(Clone, Copy, Default)]
pub struct Blk {
    pub kind: i32,
    pub fam: usize,
    pub style: i32,
    /// Base delay time in samples.
    pub t: f32,
    /// One 16th note in samples (rhythm style and glitch grid).
    pub s16: f32,
    pub ratio: f32,
    pub off: f32,
    pub swing: f32,
    pub feel: f32,
    pub tchange: i32,
    pub slide_coef: f32,
    pub fade_step: f32,
    pub freeze: bool,
    pub inloop: bool,
    pub slope: i32,
    pub lc: f32,
    pub hc: f32,
    pub res: f32,
    pub tilt: f32,
    pub ldrive: f32,
    pub xfb: f32,
    pub drive: f32,
    pub diff: f32,
    pub dpos_loop: bool,
    pub dsize: f32,
    pub dstages: usize,
    pub dmotion: f32,
    pub dspread: f32,
    pub pitch_st: f32,
    pub pmode: i32,
    pub pmix: f32,
    pub pgrain: f32,
    pub pspread: f32,
    pub mshape: i32,
    pub mtgt: i32,
    pub mrate: f32,
    pub mphase: f32,
    pub menv: f32,
    pub mdrift: f32,
    pub hears: i32,
    pub damt: f32,
    pub dthr: f32,
    pub steps: [f32; 16],
    pub wow: f32,
    pub flutter: f32,
    pub stutter: f32,
    pub revwin: f32,
    pub cgrain: f32,
    pub cdensity: f32,
    pub cspread: f32,
    pub detune: f32,
    pub ch: CharSet,
    pub os_factor: usize,
    /// PITCH is the mod target.
    pub pmod_on: bool,
    /// One trip round the loop, in samples.
    pub loop_len: f32,
    /// per-sample fall of the loop gate's peak meter (-20 dB per trip round the loop)
    pub wpk_dec: f32,
}

/// Character settings (the type's sound, one pass).
#[derive(Clone, Copy, Default)]
pub struct CharSet {
    pub crush_q: f32,
    pub hold_rate: f32,
    pub bw: f32,
    pub dnoise: f32,
    pub radio: bool,
    pub radio_sat: f32,
    pub tape: bool,
    pub tsat: f32,
    pub bump: f32,
    pub hiss: f32,
    pub tape_lp: f32,
    pub dropouts: bool,
    pub bbd: bool,
    pub bbd_lp: f32,
    pub comp: f32,
    pub clock: f32,
    pub clock_hz: f32,
    pub bsat: f32,
    pub dub: bool,
    pub dub_fc: f32,
    pub dub_q: f32,
    pub fecho: bool,
}

impl Blk {
    pub fn from(p: &LaneParams, bpm: f32, sr: f32, os: i32) -> Self {
        Self::build(p, bpm, sr, os, None)
    }
    /// `kind` overrides the ECHO TYPE param (tests).
    pub fn build(p: &LaneParams, bpm: f32, sr: f32, os: i32, kind: Option<i32>) -> Self {
        let kind = kind.unwrap_or(p.kind.value()).clamp(0, 19);
        let fam = fam(kind);
        let beat = 60.0 / bpm.clamp(20.0, 999.0) * sr;
        let t = if p.sync.value() { NOTE_BEATS[p.note.value().clamp(0, 18) as usize] * beat } else { p.ms.value() * 0.001 * sr };
        let max = MAX_SECONDS * sr - 64.0;
        let t = t.clamp(8.0, if kind == T_MULTI { max / 3.0 } else { max / 2.0 });
        let s16 = if p.sync.value() { beat / 4.0 } else { t / 4.0 };
        let glide = p.glide.value() * 0.001 * sr;
        let tchange = p.tchange.value();
        let fade_len = match tchange {
            1 => (glide * 0.5).max(0.01 * sr),
            _ => 0.004 * sr,
        };
        let mut pitch = p.pitch.value() as f32 + p.fine.value() / 100.0;
        let mut pmode = p.pmode.value();
        if kind == T_SHIMMER {
            pmode = 1;
            if p.pitch.value() == 0 {
                pitch += 12.0;
            }
        }
        let dark = p.dark.value();
        let age = p.age.value();
        let tsec = t / sr;
        let mut ch = CharSet::default();
        match fam {
            0 => {
                let mut bits = p.bits.value();
                let mut srk = p.srate.value();
                let mut bw = p.bw.value();
                match kind {
                    1 => {
                        bits = bits.min(12.0);
                        bw = bw.min(12000.0);
                    }
                    2 => {
                        bits = bits.min(8.0);
                        srk = srk.min(11.0);
                        bw = bw.min(5000.0);
                    }
                    3 => {
                        ch.radio = true;
                        ch.radio_sat = 0.5;
                    }
                    _ => {}
                }
                ch.crush_q = if bits < 23.5 { 2f32.powf(bits - 1.0) } else { 0.0 };
                ch.hold_rate = if srk < 47.5 { (srk * 1000.0 / sr).min(1.0) } else { 0.0 };
                ch.bw = if bw < 19500.0 { bw } else { 0.0 };
                ch.dnoise = p.dnoise.value() * 0.006;
            }
            1 => {
                ch.tape = true;
                ch.tsat = p.tsat.value() * if kind == 8 { 1.3 } else { 1.0 };
                ch.bump = p.bump.value();
                ch.hiss = p.hiss.value() * 0.004 * if kind == 6 { 2.0 } else { 1.0 };
                ch.tape_lp = match kind {
                    6 => 5500.0,
                    7 => 8000.0,
                    8 => 3800.0,
                    9 => 10000.0,
                    _ => 9000.0,
                };
                ch.dropouts = kind == 6;
            }
            2 => match kind {
                10 | 11 => {
                    let dirty = kind == 11;
                    ch.bbd = true;
                    ch.bbd_lp = (6000.0 * (0.25 / tsec.max(0.01)).sqrt()).clamp(2000.0, 9000.0) * (1.0 - 0.5 * dark) * (1.0 - 0.3 * age) * if dirty { 0.8 } else { 1.0 };
                    ch.comp = p.comp.value() * if dirty { 1.5 } else { 1.0 };
                    ch.clock = p.clock.value() * 0.0015 * if dirty { 3.0 } else { 1.0 } * (1.0 + age);
                    ch.clock_hz = 9000.0 - 3000.0 * age;
                    ch.bsat = if dirty { 0.6 } else { 0.2 } + 0.3 * age;
                }
                12 => {
                    ch.bw = 12000.0;
                }
                13 => {
                    ch.dub = true;
                    ch.dub_fc = 400.0 * 2f32.powf(3.3 * (1.0 - dark));
                    ch.dub_q = 2.0 + 4.0 * p.res.value();
                    ch.tsat = 0.3;
                }
                _ => {
                    ch.fecho = true;
                }
            },
            _ => {}
        }
        let tdiff = TYPES[kind as usize].3;
        let wscale = match kind {
            6 => 2.0,
            8 => 1.5,
            _ => 1.0,
        };
        Self {
            kind,
            fam,
            style: p.style.value(),
            t,
            s16,
            ratio: p.ratio.value(),
            off: p.offset.value() * 0.001 * sr,
            swing: p.groove.value() * 0.25,
            feel: p.feel.value() * 0.001 * sr,
            tchange,
            slide_coef: 1.0 - (-1.0 / glide.max(0.005 * sr)).exp(),
            fade_step: 1.0 / fade_len.max(1.0),
            freeze: p.freeze.value(),
            inloop: p.inloop.value(),
            slope: p.slope.value(),
            lc: p.lc.value(),
            hc: p.hc.value(),
            res: p.res.value(),
            tilt: p.tilt.value(),
            ldrive: p.ldrive.value(),
            xfb: p.xfb.value(),
            drive: p.drive.value(),
            diff: (p.diff.value() + tdiff).clamp(0.0, 1.0),
            dpos_loop: p.dpos.value() == 1,
            dsize: p.dsize.value() * 0.001,
            dstages: p.dstages.value() as usize,
            dmotion: p.dmotion.value(),
            dspread: p.dspread.value(),
            pitch_st: pitch,
            pmode,
            pmix: p.pmix.value(),
            pgrain: p.pgrain.value() * 0.001 * sr,
            pspread: p.pspread.value() / 100.0,
            mshape: p.mshape.value(),
            mtgt: p.mtgt.value(),
            mrate: p.mrate.value(),
            mphase: p.mphase.value() / 360.0,
            menv: p.menv.value(),
            mdrift: p.mdrift.value(),
            hears: p.hears.value(),
            damt: p.damt.value(),
            dthr: p.dthr.value(),
            steps: std::array::from_fn(|k| STEP_GAIN[p.steps[k].v.value().clamp(0, 3) as usize]),
            wow: if fam == 1 { p.wow.value() * wscale } else { 0.0 },
            flutter: if fam == 1 { p.flutter.value() * if kind == 6 { 1.5 } else { 1.0 } } else { 0.0 },
            stutter: p.stutter.value(),
            revwin: p.revwin.value() * 0.001 * sr,
            cgrain: p.cgrain.value() * 0.001 * sr,
            cdensity: p.density.value(),
            cspread: p.cspread.value(),
            detune: p.detune.value() / 100.0,
            ch,
            os_factor: crate::oversample::factor_for(os),
            pmod_on: p.mtgt.value() == 3 && p.modu.value() > 0.001,
            loop_len: 0.0,
            wpk_dec: 0.0,
        }
        .with_loop_len()
    }
    fn with_loop_len(mut self) -> Self {
        self.loop_len = if self.kind == T_MULTI {
            3.0 * self.t
        } else if self.style == S_RHYTHM {
            16.0 * self.s16
        } else if self.style == S_DUAL || self.style == S_RATIO {
            self.t.max(self.t * self.ratio + self.off.max(0.0))
        } else {
            self.t
        };
        self.wpk_dec = 0.1f32.powf(1.0 / self.loop_len.max(1.0));
        self
    }
    /// Pitch stage runs on every pass (inside the loop).
    pub fn pitch_loop(&self) -> bool {
        (self.pmode == 1 && (self.pitch_st.abs() > 0.001 || self.pmod_on)) || self.kind == 16
    }
    pub fn pitch_once(&self) -> bool {
        self.pmode != 1 && self.kind != 16 && (self.pitch_st.abs() > 0.001 || self.pmod_on)
    }
}

// ------------------------------------------------------------- the filter

#[derive(Default)]
struct Filt {
    hp1: OnePole,
    lp1: OnePole,
    hp: [Svf; 2],
    lp: [Svf; 2],
    tilt: OnePole,
    gl: f32,
    gh: f32,
    slope: i32,
    use_hp: bool,
    use_lp: bool,
    norm: f32,
}

/// Gain (dB) of one pass of the lane filter at `f`, including its peak scaling (for the screens).
pub fn filt_db(f: f32, b: &Blk) -> f32 {
    let (hp, lp) = (b.lc > 20.5, b.hc < 19900.0);
    let norm = 1.0 / filt_peak(b.lc, b.hc, b.slope, b.res, b.tilt, hp, lp).max(1.0);
    20.0 * (filt_mag(f, b.lc, b.hc, b.slope, b.res, b.tilt, hp, lp) * norm).max(1e-6).log10()
}

fn filt_peak(lc: f32, hc: f32, slope: i32, res: f32, tilt: f32, hp: bool, lp: bool) -> f32 {
    (0..64).map(|i| filt_mag(20.0 * 1000f32.powf(i as f32 / 63.0), lc, hc, slope, res, tilt, hp, lp)).fold(0.0, f32::max)
}

/// Magnitude of the filter chain at `f` (analog prototypes).
#[allow(clippy::too_many_arguments)]
fn filt_mag(f: f32, lc: f32, hc: f32, slope: i32, res: f32, tilt: f32, hp: bool, lp: bool) -> f32 {
    let sec = |f: f32, fc: f32, q: f32, high: bool| -> f32 {
        let w = f / fc;
        let num = if high { w * w } else { 1.0 };
        num / ((1.0 - w * w).powi(2) + (w / q).powi(2)).sqrt()
    };
    let one = |f: f32, fc: f32, high: bool| -> f32 {
        let w = f / fc;
        if high {
            w / (1.0 + w * w).sqrt()
        } else {
            1.0 / (1.0 + w * w).sqrt()
        }
    };
    let (gl, gh) = (10f32.powf(-tilt / 40.0), 10f32.powf(tilt / 40.0));
    {
        let mut m = 1.0;
        match slope {
            0 => {
                if hp {
                    m *= one(f, lc, true);
                }
                if lp {
                    m *= one(f, hc, false);
                }
            }
            1 => {
                let q = 0.707 + res * 3.0;
                if hp {
                    m *= sec(f, lc, q, true);
                }
                if lp {
                    m *= sec(f, hc, q, false);
                }
            }
            _ => {
                let q2 = 1.307 + res * 3.0;
                if hp {
                    m *= sec(f, lc, 0.541, true) * sec(f, lc, q2, true);
                }
                if lp {
                    m *= sec(f, hc, 0.541, false) * sec(f, hc, q2, false);
                }
            }
        }
        // tilt: one-pole split at 1 kHz
        let w = f / 1000.0;
        let (re, im) = ((gl + gh * w * w) / (1.0 + w * w), (gh - gl) * w / (1.0 + w * w));
        m *= (re * re + im * im).sqrt();
        m
    }
}

impl Filt {
    fn set(&mut self, lc: f32, hc: f32, slope: i32, res: f32, tilt: f32, sr: f32) {
        self.slope = slope;
        self.use_hp = lc > 20.5;
        self.use_lp = hc < 19900.0;
        // Resonance / tilt may peak above 0 dB: scale down so a repeat never comes
        // back louder than FEEDBACK lets it (no endless ringing at sane settings).
        self.norm = 1.0 / filt_peak(lc, hc, slope, res, tilt, self.use_hp, self.use_lp).max(1.0);
        let q = 0.707 + res * 3.0;
        match slope {
            0 => {
                self.hp1.set(lc, sr);
                self.lp1.set(hc, sr);
            }
            1 => {
                self.hp[0].set(lc, q, sr);
                self.lp[0].set(hc, q, sr);
            }
            _ => {
                self.hp[0].set(lc, 0.541, sr);
                self.hp[1].set(lc, 1.307 + res * 3.0, sr);
                self.lp[0].set(hc, 0.541, sr);
                self.lp[1].set(hc, 1.307 + res * 3.0, sr);
            }
        }
        self.tilt.set(1000.0, sr);
        self.gl = 10f32.powf(-tilt / 40.0);
        self.gh = 10f32.powf(tilt / 40.0);
    }
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let mut y = x;
        match self.slope {
            0 => {
                if self.use_hp {
                    y = self.hp1.hp(y);
                }
                if self.use_lp {
                    y = self.lp1.lp(y);
                }
            }
            1 => {
                if self.use_hp {
                    y = self.hp[0].tick(y).hp;
                }
                if self.use_lp {
                    y = self.lp[0].tick(y).lp;
                }
            }
            _ => {
                if self.use_hp {
                    y = self.hp[0].tick(y).hp;
                    y = self.hp[1].tick(y).hp;
                }
                if self.use_lp {
                    y = self.lp[0].tick(y).lp;
                    y = self.lp[1].tick(y).lp;
                }
            }
        }
        if (self.gl - 1.0).abs() > 1e-4 {
            let lo = self.tilt.lp(y);
            y = lo * self.gl + (y - lo) * self.gh;
        }
        y * self.norm
    }
    fn clear(&mut self) {
        self.hp1.clear();
        self.lp1.clear();
        self.hp.iter_mut().chain(self.lp.iter_mut()).for_each(Svf::clear);
        self.tilt.clear();
    }
}

// --------------------------------------------------------- the character

struct Char {
    held: f32,
    acc: f32,
    bw: Svf,
    rhp: Svf,
    rlp: Svf,
    tlp: [OnePole; 2],
    bump: Peak,
    blp: [Svf; 2],
    cenv: Follower,
    nenv: Follower,
    dub: Svf,
    fe: Svf,
    noise: Noise,
    clk_ph: f32,
    drop_g: f32,
    drop_left: i32,
    drop_wait: i32,
    dub_norm: f32,
}
impl Char {
    fn new(seed: u32) -> Self {
        Self {
            held: 0.0,
            acc: 1.0,
            bw: Svf::default(),
            rhp: Svf::default(),
            rlp: Svf::default(),
            tlp: [OnePole::default(); 2],
            bump: Peak::default(),
            blp: [Svf::default(); 2],
            cenv: Follower::default(),
            nenv: Follower::default(),
            dub: Svf::default(),
            fe: Svf::default(),
            noise: Noise(seed | 1),
            clk_ph: 0.0,
            drop_g: 1.0,
            drop_left: 0,
            drop_wait: 48000,
            dub_norm: 1.0,
        }
    }
    fn set(&mut self, c: &CharSet, sr: f32) {
        if c.bw > 0.0 {
            self.bw.set(c.bw, 0.707, sr);
        }
        if c.radio {
            self.rhp.set(300.0, 0.8, sr);
            self.rlp.set(3000.0, 0.9, sr);
        }
        if c.tape {
            self.tlp[0].set(c.tape_lp, sr);
            self.tlp[1].set(c.tape_lp * 1.4, sr);
            self.bump.set(90.0, 0.8, c.bump, sr);
        }
        if c.bbd {
            self.blp[0].set(c.bbd_lp, 0.6, sr);
            self.blp[1].set(c.bbd_lp, 0.9, sr);
            self.cenv.set(5.0, 80.0, sr);
        }
        if c.dub {
            self.dub.set(c.dub_fc, c.dub_q, sr);
            // keep the resonance just above unity so long throws hang, not explode
            let q = c.dub_q;
            self.dub_norm = (1.0 / (q / (1.0 - 1.0 / (4.0 * q * q)).sqrt())) * (1.0 + 0.04 * (q - 2.0));
        }
        self.nenv.set(1.0, 300.0, sr);
    }
    fn clear(&mut self) {
        self.held = 0.0;
        self.bw.clear();
        self.rhp.clear();
        self.rlp.clear();
        self.tlp.iter_mut().for_each(OnePole::clear);
        self.bump.clear();
        self.blp.iter_mut().for_each(Svf::clear);
        self.dub.clear();
        self.fe.clear();
        self.cenv.env = 0.0;
    }
    /// One pass. `out` = the copy heard at the output (not fed back): only that
    /// one gets level make-up and the head bump, so the loop never gains level.
    #[inline]
    fn tick(&mut self, x: f32, c: &CharSet, sr: f32, out: bool) -> f32 {
        let mut y = x;
        // Hiss / noise ride the signal: when the repeats are gone, so is the noise.
        let ng = (self.nenv.tick(x) * 30.0).min(1.0);
        // DIGITAL
        if c.hold_rate > 0.0 {
            self.acc += c.hold_rate;
            if self.acc >= 1.0 {
                self.acc -= 1.0;
                self.held = y;
            }
            y = self.held;
        }
        if c.crush_q > 0.0 {
            y = (y * c.crush_q).round() / c.crush_q;
        }
        if c.bw > 0.0 {
            y = self.bw.tick(y).lp;
        }
        if c.dnoise > 0.0 {
            y += self.noise.next() * c.dnoise * ng;
        }
        if c.radio {
            y = self.rhp.tick(y).hp;
            y = self.rlp.tick(y).lp;
            y = sat(y * 1.6, c.radio_sat) / if out { 1.2 } else { 1.6 };
        }
        // TAPE
        if c.tape {
            y = sat(y, c.tsat);
            if c.bump > 0.01 && out {
                y = self.bump.tick(y);
            }
            y = self.tlp[0].lp(y);
            y = self.tlp[1].lp(y);
            if c.hiss > 0.0 {
                y += self.noise.next() * c.hiss * ng;
            }
            if c.dropouts {
                if self.drop_left > 0 {
                    self.drop_left -= 1;
                    self.drop_g += (0.35 - self.drop_g) * (200.0 / sr);
                } else {
                    self.drop_g += (1.0 - self.drop_g) * (100.0 / sr);
                    self.drop_wait -= 1;
                    if self.drop_wait <= 0 {
                        self.drop_wait = ((1.0 + 3.0 * self.noise.uni()) * sr) as i32;
                        self.drop_left = ((0.04 + 0.08 * self.noise.uni()) * sr) as i32;
                    }
                }
                y *= self.drop_g;
            }
        }
        // ANALOG
        if c.bbd {
            let e = self.cenv.tick(y);
            y = y / (1.0 + c.comp * 2.0 * e) * if out { 1.0 + c.comp * 0.3 } else { 1.0 };
            y = sat(y, c.bsat);
            y = self.blp[0].tick(y).lp;
            y = self.blp[1].tick(y).lp;
            if c.clock > 0.0 {
                self.clk_ph = (self.clk_ph + c.clock_hz / sr) % 1.0;
                y += (self.clk_ph * std::f32::consts::TAU).sin() * c.clock * ng * (0.7 + 0.3 * self.noise.next());
            }
        }
        if c.dub {
            y = sat(self.dub.tick(y).lp * self.dub_norm, 0.3);
        }
        y
    }
    /// FILTER ECHO: swept resonant low-pass, `sweep` 0..1.
    #[inline]
    fn fecho(&mut self, x: f32, sweep: f32, update: bool, sr: f32, out: bool) -> f32 {
        if update {
            self.fe.set(300.0 * 2f32.powf(5.0 * sweep.clamp(0.0, 1.0)), 1.6, sr);
        }
        // Q 1.6 peaks ~ +4.6 dB: the looped copy is scaled under unity, the heard copy less so.
        self.fe.tick(x).lp * if out { 0.85 } else { 0.58 }
    }
}

// ----------------------------------------------------------- time slewing

#[derive(Clone, Copy, Default)]
struct TimeSlew {
    a: f32,
    b: f32,
    x: f32,
    fresh: bool,
}
impl TimeSlew {
    /// Returns (time a, time b, weight of b).
    #[inline]
    fn tick(&mut self, target: f32, blk: &Blk) -> (f32, f32, f32) {
        if !self.fresh {
            self.fresh = true;
            self.a = target;
            self.b = target;
            self.x = 1.0;
        }
        if blk.tchange == 0 {
            self.a += (target - self.a) * blk.slide_coef;
            self.x = 1.0;
            return (self.a, self.a, 0.0);
        }
        if self.x >= 1.0 {
            if (target - self.a).abs() > 0.5 {
                self.b = target;
                self.x = 0.0;
            } else {
                return (self.a, self.a, 0.0);
            }
        }
        self.x += blk.fade_step;
        if self.x >= 1.0 {
            self.a = self.b;
            self.x = 1.0;
            return (self.a, self.a, 0.0);
        }
        let w = (self.x * std::f32::consts::FRAC_PI_2).sin().powi(2);
        (self.a, self.b, w)
    }
}

// ------------------------------------------------------------- the lane

#[derive(Clone, Copy, Default)]
struct Grain {
    age: f32,
    len: f32,
    off: f32,
    gl: f32,
    gr: f32,
}

pub struct LaneOut {
    pub l: f32,
    pub r: f32,
    /// The repeats before FEEDBACK / OUTPUT (what CROSS-FEED passes over).
    pub rep_l: f32,
    pub rep_r: f32,
}

pub struct Lane {
    sr: f32,
    line: [Line; 2],
    loop_f: [Filt; 2],
    out_f: [Filt; 2],
    loop_c: [Char; 2],
    out_c: [Char; 2],
    pre_p: [Pitch; 2],
    loop_p: [Pitch; 2],
    out_p: [Pitch; 2],
    diff_a: Diffuser,
    diff_b: Diffuser,
    os: [[Oversampler; 3]; 2],
    slew: TimeSlew,
    lfo: Lfo,
    wow_ph: f32,
    flut_ph: f32,
    chorus_ph: [f32; 3],
    rev_ph: f32,
    grains: [Grain; 8],
    grain_wait: f32,
    noise: Noise,
    glitch_pos: f32,
    glitch_mode: [i32; 2],
    env_in: Follower,
    duck: Follower,
    pub duck_db: f32,
    gate: f32,
    pub rep_prev: [f32; 2],
    upd: u32,
    sleeping: bool,
    /// samples since anything audible went into the delay line
    idle: u32,
    out_gc: GainCache,
    duck_g: f32,
    cr_n: u32,
    cr_prev: [f32; 4],
    cr_next: [f32; 4],
    /// CROSS-FEED amount (set by the plugin each block).
    pub cross: f32,
    /// loop gate: recent write peak, quiet time, and the gate gain
    wpk: f32,
    quiet_n: u32,
    lgate: f32,
    /// FREEZE crossfade 0..1 (30 ms) so going in / out of a hold never clicks.
    fz: f32,
}

impl Lane {
    pub fn new(sr: f32, seed: u32) -> Self {
        let n = (MAX_SECONDS * sr) as usize + 64;
        let mk_os = || [Oversampler::new(2), Oversampler::new(4), Oversampler::new(8)];
        Self {
            sr,
            line: [Line::new(n), Line::new(n)],
            loop_f: Default::default(),
            out_f: Default::default(),
            loop_c: [Char::new(seed * 3 + 1), Char::new(seed * 5 + 7)],
            out_c: [Char::new(seed * 7 + 3), Char::new(seed * 11 + 5)],
            pre_p: [Pitch::new(sr), Pitch::new(sr)],
            loop_p: [Pitch::new(sr), Pitch::new(sr)],
            out_p: [Pitch::new(sr), Pitch::new(sr)],
            diff_a: Diffuser::new(sr),
            diff_b: Diffuser::new(sr),
            os: [mk_os(), mk_os()],
            slew: TimeSlew::default(),
            lfo: Lfo::new(seed * 13 + 1),
            wow_ph: 0.0,
            flut_ph: 0.0,
            chorus_ph: [0.0, 0.33, 0.66],
            rev_ph: 0.0,
            grains: [Grain::default(); 8],
            grain_wait: 0.0,
            noise: Noise(seed * 977 + 31),
            glitch_pos: 0.0,
            glitch_mode: [0; 2],
            env_in: Follower::default(),
            duck: Follower::default(),
            duck_db: 0.0,
            gate: 1.0,
            rep_prev: [0.0; 2],
            upd: 0,
            sleeping: false,
            idle: u32::MAX / 2,
            out_gc: GainCache::default(),
            duck_g: 1.0,
            cr_n: 0,
            cr_prev: [0.0; 4],
            cr_next: [0.0; 4],
            cross: 0.0,
            wpk: 0.0,
            quiet_n: 0,
            lgate: 1.0,
            fz: 0.0,
        }
    }

    pub fn clear(&mut self) {
        self.line.iter_mut().for_each(Line::clear);
        self.loop_f.iter_mut().for_each(Filt::clear);
        self.out_f.iter_mut().for_each(Filt::clear);
        self.loop_c.iter_mut().for_each(Char::clear);
        self.out_c.iter_mut().for_each(Char::clear);
        for p in self.pre_p.iter_mut().chain(self.loop_p.iter_mut()).chain(self.out_p.iter_mut()) {
            p.clear();
        }
        self.diff_a.clear();
        self.diff_b.clear();
        self.os.iter_mut().flatten().for_each(Oversampler::clear);
        self.rep_prev = [0.0; 2];
        self.grains = [Grain::default(); 8];
        self.slew.fresh = false;
    }

    /// Per-block setup (filters and character coefficients).
    pub fn setup(&mut self, b: &Blk, p: &LaneParams) {
        let sr = self.sr;
        for f in self.loop_f.iter_mut().chain(self.out_f.iter_mut()) {
            f.set(b.lc, b.hc, b.slope, b.res, b.tilt, sr);
        }
        for c in self.loop_c.iter_mut().chain(self.out_c.iter_mut()) {
            c.set(&b.ch, sr);
        }
        self.env_in.set(5.0, 150.0, sr);
        self.duck.set(p.datt.value(), p.drel.value(), sr);
    }

    #[inline]
    fn os_lat(b: &Blk) -> f32 {
        if b.drive > 0.01 && b.os_factor > 1 {
            latency(b.os_factor) as f32
        } else {
            0.0
        }
    }

    /// Plain read of both channels at delay `d` (samples) plus per-channel offset.
    #[inline]
    fn rd(&self, ch: usize, d: f32) -> f32 {
        self.line[ch].read(d.max(2.0))
    }

    /// (feedback read, out read) for both channels at base time `t`.
    #[inline]
    fn reads(&self, b: &Blk, t: f32, tm: [f32; 2], comp_f: f32, comp_o: f32) -> ([f32; 2], [f32; 2]) {
        let lat = Self::os_lat(b);
        let sh_f = |d: f32, ch: usize| d - lat - comp_f + tm[ch];
        let sh_o = |d: f32, ch: usize| d - lat - comp_o + tm[ch];
        // Feedback read positions and out read positions per style.
        let (df, dout) = if b.kind == T_MULTI {
            ([3.0 * t, 3.0 * t], [t, t])
        } else {
            match b.style {
                S_DUAL => {
                    let r = t * b.ratio + b.off;
                    ([t, r], [t + b.feel, r + b.feel])
                }
                S_PING => ([t, t], [t + b.feel, t + b.feel + b.swing * t + b.off]),
                S_RATIO => {
                    let r = t * b.ratio;
                    ([t, r], [t + b.feel, r + b.feel + b.swing * r + b.off])
                }
                S_RHYTHM => {
                    let bar = 16.0 * b.s16;
                    ([bar, bar], [0.0, 0.0])
                }
                _ => ([t, t], [t + b.feel, t + b.feel]),
            }
        };
        let fb = [self.rd(0, sh_f(df[0], 0)), self.rd(1, sh_f(df[1], 1))];
        let mut out = [0.0f32; 2];
        if b.style == S_RHYTHM && b.kind != T_MULTI {
            for k in 0..16 {
                let g = b.steps[k];
                if g <= 0.0 {
                    continue;
                }
                let mut d = (k as f32 + 1.0) * b.s16 + b.feel;
                if k % 2 == 0 {
                    d += b.swing * 2.0 * b.s16;
                }
                let v = 0.5 * (self.rd(0, sh_o(d, 0)) + self.rd(1, sh_o(d, 1))) * g;
                let c = k % 2;
                out[c] += v;
            }
            return (fb, out);
        }
        for ch in 0..2 {
            let d = sh_o(dout[ch], ch);
            out[ch] = match b.kind {
                T_MULTI => self.rd(ch, d) + 0.708 * self.rd(ch, d + t) + 0.5 * self.rd(ch, d + 2.0 * t),
                T_CHORUS => {
                    let mut s = 0.0;
                    for k in 0..3 {
                        let m = (0.002 + 0.0015 * k as f32) * self.sr * (1.0 + 0.5 * (std::f32::consts::TAU * (self.chorus_ph[k] + ch as f32 * 0.25)).sin());
                        s += self.rd(ch, d + m);
                    }
                    s * 0.577
                }
                T_REVERSE => {
                    let w = b.revwin.min(t.max(64.0));
                    let mut s = 0.0;
                    for g in 0..2 {
                        let ph = (self.rev_ph + g as f32 * 0.5) % 1.0;
                        let win = (std::f32::consts::PI * ph).sin().powi(2);
                        s += win * self.rd(ch, d - w + 2.0 * ph * w);
                    }
                    s
                }
                T_CLOUD => {
                    let mut s = 0.0;
                    for gr in self.grains.iter() {
                        if gr.len <= 0.0 {
                            continue;
                        }
                        let win = (std::f32::consts::PI * (gr.age / gr.len)).sin().powi(2);
                        s += win * self.rd(ch, d + gr.off) * if ch == 0 { gr.gl } else { gr.gr };
                    }
                    s * (0.45 / (0.4 + b.cdensity)).sqrt()
                }
                T_GLITCH => {
                    let extra = match self.glitch_mode[0] {
                        2 => b.s16,
                        3 => b.s16 * 0.5,
                        _ => 0.0,
                    };
                    let g = self.glitch_gain(b);
                    self.rd(ch, d + extra) * g
                }
                _ => self.rd(ch, d),
            };
        }
        (fb, out)
    }

    #[inline]
    fn glitch_gain(&self, b: &Blk) -> f32 {
        let fade = 0.0015 * self.sr;
        let mut g = if self.glitch_mode[0] == 1 { 0.0 } else { 1.0 };
        if g > 0.0 {
            // dip at boundaries only when the mode changes
            let to_end = b.s16 - self.glitch_pos;
            if self.glitch_mode[1] != self.glitch_mode[0] && to_end < fade {
                g *= (to_end / fade).clamp(0.0, 1.0);
            }
            if self.glitch_pos < fade {
                g *= (self.glitch_pos / fade).clamp(0.0, 1.0);
            }
        }
        g
    }

    fn advance_states(&mut self, b: &Blk) {
        let sr = self.sr;
        for (k, ph) in self.chorus_ph.iter_mut().enumerate() {
            *ph = (*ph + (0.6 + 0.23 * k as f32) / sr) % 1.0;
        }
        if b.kind == T_REVERSE {
            let w = b.revwin.min(b.t.max(64.0));
            self.rev_ph = (self.rev_ph + 1.0 / w) % 1.0;
        }
        if b.kind == T_CLOUD {
            for g in self.grains.iter_mut() {
                if g.len > 0.0 {
                    g.age += 1.0;
                    if g.age >= g.len {
                        g.len = 0.0;
                    }
                }
            }
            self.grain_wait -= 1.0;
            if self.grain_wait <= 0.0 {
                self.grain_wait = b.cgrain / (1.0 + b.cdensity * 7.0);
                if let Some(g) = self.grains.iter_mut().find(|g| g.len <= 0.0) {
                    let pan = (self.noise.next() * b.cspread).clamp(-1.0, 1.0);
                    let a = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                    *g = Grain { age: 0.0, len: b.cgrain * (0.7 + 0.6 * self.noise.uni()), off: self.noise.uni() * b.t * 0.5 * b.cspread, gl: a.cos() * 1.414, gr: a.sin() * 1.414 };
                }
            }
        }
        if b.kind == T_GLITCH {
            self.glitch_pos += 1.0;
            if self.glitch_pos >= b.s16 {
                self.glitch_pos -= b.s16;
                self.glitch_mode[0] = self.glitch_mode[1];
                let r = self.noise.uni();
                self.glitch_mode[1] = if r < b.stutter { 1 + (self.noise.uni() * 3.0) as i32 % 3 } else { 0 };
            }
        }
    }

    /// One sample. `inp` = what the lane hears (after SEND), `xin` = CROSS-FEED
    /// from the other lane, `duck_src` = level the ducker listens to.
    /// `on` = lane switched on, `gate_on` = audible (on, not muted, solo rules).
    #[inline]
    pub fn tick(&mut self, b: &Blk, p: &LaneParams, inp: (f32, f32), xin: (f32, f32), duck_src: f32, on: bool, gate_on: bool) -> LaneOut {
        let sr = self.sr;
        // smoothed knobs (always advanced so they stay in step)
        let fb_amt = p.fb.smoothed.next();
        let out_g = self.out_gc.get(p.out.smoothed.next());
        let width = p.width.smoothed.next();
        let pan = p.pan.smoothed.next();
        let modu = p.modu.smoothed.next();

        let target_gate = if gate_on { 1.0 } else { 0.0 };
        self.gate += (target_gate - self.gate) * (1.0 / (0.01 * sr)).min(1.0);
        if !on && self.gate < 1e-4 {
            if !self.sleeping {
                self.sleeping = true;
                self.clear();
            }
            self.gate = 0.0;
            return LaneOut { l: 0.0, r: 0.0, rep_l: 0.0, rep_r: 0.0 };
        }
        self.sleeping = false;
        // Nothing coming in and the line has held only silence for longer than any
        // read reaches back: the lane is silent, so skip the work (CPU drops to ~0).
        let quiet_in = inp.0.abs() < 1e-7 && inp.1.abs() < 1e-7 && (self.cross < 1e-4 || (xin.0.abs() < 1e-7 && xin.1.abs() < 1e-7));
        if quiet_in && !b.freeze && self.fz == 0.0 {
            let reach = 3.0 * b.t.max(16.0 * b.s16) * b.ratio.max(1.0) + 2.0 * b.revwin + 0.8 * sr;
            if self.idle as f32 > reach {
                self.rep_prev = [0.0; 2];
                self.duck_db = 0.0;
                return LaneOut { l: 0.0, r: 0.0, rep_l: 0.0, rep_r: 0.0 };
            }
        }

        // ---- modulation
        let env = self.env_in.tick(0.5 * (inp.0.abs() + inp.1.abs()));
        let env01 = (env * (1.0 + 30.0 * b.menv)).min(1.0);
        // Modulation runs at 1/8 rate and is ramped in between (saves the sines).
        if self.cr_n % 8 == 0 {
            self.cr_prev = self.cr_next;
            let crs = sr / 8.0;
            let (ml, mr) = self.lfo.tick(b.mshape, b.mrate, b.mphase, b.mdrift, env01, crs);
            let mut tm = [0.0f32; 2];
            if b.mtgt == 0 {
                tm[0] += ml * modu * 0.008 * sr;
                tm[1] += mr * modu * 0.008 * sr;
            }
            if b.wow > 0.0 || b.flutter > 0.0 {
                self.wow_ph = (self.wow_ph + 0.55 / crs) % 1.0;
                self.flut_ph = (self.flut_ph + 6.5 / crs) % 1.0;
                let w = (std::f32::consts::TAU * self.wow_ph).sin() * b.wow * 0.003 * sr;
                let f = (std::f32::consts::TAU * self.flut_ph).sin() * b.flutter * 0.00025 * sr;
                tm[0] += w + f;
                tm[1] += w * 0.9 + f;
            }
            if b.ch.bbd {
                self.wow_ph = (self.wow_ph + 0.35 / crs) % 1.0;
                let w = (std::f32::consts::TAU * self.wow_ph).sin() * 0.0004 * sr;
                tm[0] += w;
                tm[1] -= w;
            }
            self.cr_next = [ml, mr, tm[0], tm[1]];
        }
        let fr = (self.cr_n % 8) as f32 / 8.0;
        self.cr_n = self.cr_n.wrapping_add(1);
        let cv = |i: usize| self.cr_prev[i] + (self.cr_next[i] - self.cr_prev[i]) * fr;
        let (ml, mr) = (cv(0), cv(1));
        let tm = [cv(2), cv(3)];
        // Filter target: move cutoffs (coefficients every 32 samples).
        self.upd = self.upd.wrapping_add(1);
        let upd = self.upd % 32 == 0;
        if b.mtgt == 1 && modu > 0.001 && upd {
            let k = 2f32.powf(2.0 * modu * ml);
            let kr = 2f32.powf(2.0 * modu * mr);
            for (i, f) in self.loop_f.iter_mut().chain(self.out_f.iter_mut()).enumerate() {
                let kk = if i % 2 == 0 { k } else { kr };
                f.set(b.lc * kk, b.hc * kk, b.slope, b.res, b.tilt, sr);
            }
        }
        let pitch_mod = if b.mtgt == 3 { modu } else { 0.0 };
        let pan_mod = if b.mtgt == 2 { modu } else { 0.0 };

        // ---- input stage (frozen: nothing new comes in). FREEZE fades in/out over
        // 30 ms: input closes, feedback goes to unity, the loop chain steps aside.
        let fz_t = if b.freeze { 1.0 } else { 0.0 };
        if self.fz != fz_t {
            let st = 1.0 / (0.03 * sr);
            self.fz = if fz_t > self.fz { (self.fz + st).min(1.0) } else { (self.fz - st).max(0.0) };
        }
        let fz = self.fz;
        let frozen = fz > 0.0;
        let mut x = [inp.0 * (1.0 - fz), inp.1 * (1.0 - fz)];
        if b.pitch_once() {
            let up_down = b.pmode == 2;
            for ch in 0..2 {
                let sgn = if up_down && ch == 1 { -1.0 } else { 1.0 };
                let det = if ch == 0 { b.pspread } else { -b.pspread };
                let st = sgn * b.pitch_st + det + pitch_mod * if ch == 0 { ml } else { mr };
                let y = self.pre_p[ch].tick(x[ch], 2f32.powf(st / 12.0), b.pgrain);
                x[ch] = x[ch] * (1.0 - b.pmix) + y * b.pmix;
            }
        }
        if b.drive > 0.01 {
            let g = 10f32.powf(b.drive / 20.0);
            let norm = g.powf(-0.75);
            let fi = match b.os_factor {
                2 => 0,
                4 => 1,
                8 => 2,
                _ => 3,
            };
            for (ch, xs) in x.iter_mut().enumerate() {
                if fi < 3 {
                    let os = &mut self.os[ch][fi];
                    let mut buf = [0.0f32; MAX_FACTOR];
                    os.up(*xs, &mut buf);
                    for v in buf.iter_mut().take(b.os_factor) {
                        *v = ftanh(*v * g) * norm;
                    }
                    *xs = os.down(&mut buf);
                } else {
                    *xs = ftanh(*xs * g) * norm;
                }
            }
        }
        if !b.dpos_loop && b.diff > 0.005 {
            let (l, r) = self.diff_a.tick(x[0], x[1], b.diff, b.dsize, b.dstages, b.dmotion, b.dspread);
            x = [l, r];
        }

        // ---- reads
        let pl = b.pitch_loop();
        let comp = if pl { b.pgrain * 0.5 } else { 0.0 };
        let (ta, tb, w) = self.slew.tick(b.t, b);
        let (mut rf, mut ro) = self.reads(b, ta, tm, comp, comp);
        if w > 0.0 {
            let (rf2, ro2) = self.reads(b, tb, tm, comp, comp);
            for ch in 0..2 {
                rf[ch] = rf[ch] * (1.0 - w) + rf2[ch] * w;
                ro[ch] = ro[ch] * (1.0 - w) + ro2[ch] * w;
            }
        }
        self.advance_states(b);
        let sweep = 0.5 + 0.5 * ml * (0.4 + 0.6 * modu);

        // ---- loop chain
        let ld = if b.ldrive > 0.01 { (10f32.powf(b.ldrive / 20.0) - 1.0) * 0.25 } else { 0.0 };
        let mut lp = rf;
        if fz < 1.0 {
            for ch in 0..2 {
                let mut y = lp[ch];
                if b.inloop {
                    y = self.loop_f[ch].tick(y);
                }
                y = self.loop_c[ch].tick(y, &b.ch, sr, false);
                if b.ch.fecho {
                    y = self.loop_c[ch].fecho(y, sweep, upd, sr, false);
                }
                y = sat(y, ld);
                if pl {
                    let st = self.pitch_for(b, ch, pitch_mod, ml, mr);
                    let s = self.loop_p[ch].tick(y, 2f32.powf(st / 12.0), b.pgrain);
                    y = y * (1.0 - b.pmix) + s * b.pmix;
                }
                lp[ch] = y;
            }
            if b.dpos_loop && b.diff > 0.005 {
                let (l, r) = self.diff_a.tick(lp[0], lp[1], b.diff, b.dsize, b.dstages, b.dmotion, b.dspread);
                lp = [l, r];
            }
            if fz > 0.0 {
                lp = [lp[0] + (rf[0] - lp[0]) * fz, lp[1] + (rf[1] - lp[1]) * fz];
            }
        }
        // ---- out chain (same stages, once)
        let mut op = ro;
        for ch in 0..2 {
            let mut y = op[ch];
            y = self.out_f[ch].tick(y);
            y = self.out_c[ch].tick(y, &b.ch, sr, true);
            if b.ch.fecho {
                y = self.out_c[ch].fecho(y, sweep, upd, sr, true);
            }
            y = sat(y, ld);
            if pl {
                let st = self.pitch_for(b, ch, pitch_mod, ml, mr);
                let s = self.out_p[ch].tick(y, 2f32.powf(st / 12.0), b.pgrain);
                y = y * (1.0 - b.pmix) + s * b.pmix;
            }
            op[ch] = y;
        }
        if b.dpos_loop && b.diff > 0.005 {
            let (l, r) = self.diff_b.tick(op[0], op[1], b.diff, b.dsize, b.dstages, b.dmotion, b.dspread);
            op = [l, r];
        }

        // ---- write back
        let fb0 = if b.kind == T_SLAP { fb_amt * 0.35 } else { fb_amt };
        let fbk = fb0 + (1.0 - fb0) * fz;
        let xf = b.xfb * 0.5 * (1.0 - fz);
        let mut fl = lp[0] * (1.0 - xf) + lp[1] * xf;
        let mut fr = lp[1] * (1.0 - xf) + lp[0] * xf;
        // CROSS-FEED shares the feedback with the other lane instead of adding to
        // it, so the two loops together are never louder than FEEDBACK says.
        let c = self.cross.clamp(0.0, 1.0) * (1.0 - fz);
        if c > 0.0 {
            fl = fl * (1.0 - c) + xin.0 * c;
            fr = fr * (1.0 - c) + xin.1 * c;
        }
        let cx = (0.0f32, 0.0f32);
        let pingish = (b.style == S_PING || b.style == S_RATIO) && b.kind != T_MULTI;
        let (wl, wr) = if pingish {
            (0.5 * (x[0] + x[1]) + fbk * fr + cx.0, fbk * fl + cx.1)
        } else {
            (x[0] + fbk * fl + cx.0, x[1] + fbk * fr + cx.1)
        };
        let (mut wl, mut wr) = if fz >= 1.0 { (wl, wr) } else if fz > 0.0 { (wl + (loop_clip(wl) - wl) * (1.0 - fz), wr + (loop_clip(wr) - wr) * (1.0 - fz)) } else { (loop_clip(wl), loop_clip(wr)) };
        // Loop gate: with nothing coming in, a loop that has fallen under -80 dB is
        // cleared, so high FEEDBACK can't grow sound out of silence. A runaway you
        // can hear keeps going; FREEZE is never touched.
        let dec = b.wpk_dec;
        self.wpk = (self.wpk * dec).max(wl.abs().max(wr.abs()));
        if quiet_in && !frozen {
            self.quiet_n = self.quiet_n.saturating_add(1);
        } else {
            self.quiet_n = 0;
            self.lgate = 1.0;
        }
        if self.quiet_n as f32 > 2.0 * b.loop_len && self.wpk < 1e-4 {
            self.lgate = (self.lgate - 1.0 / (0.02 * sr)).max(0.0);
        }
        if self.lgate < 1.0 {
            wl *= self.lgate;
            wr *= self.lgate;
        }
        self.line[0].write(if wl.is_finite() { wl } else { 0.0 });
        self.line[1].write(if wr.is_finite() { wr } else { 0.0 });
        if wl.abs() > 3e-5 || wr.abs() > 3e-5 || !quiet_in {
            self.idle = 0;
        } else {
            self.idle = self.idle.saturating_add(1);
        }
        self.rep_prev = lp;

        // ---- duck
        let e = self.duck.tick(duck_src);
        if self.upd % 8 == 0 {
            if b.damt > 0.01 {
                let edb = 20.0 * e.max(1e-6).log10();
                self.duck_db = -((edb - b.dthr).max(0.0) * 1.5).min(b.damt);
                self.duck_g = 10f32.powf(self.duck_db / 20.0);
            } else {
                self.duck_db = 0.0;
                self.duck_g = 1.0;
            }
        }
        let dg = self.duck_g;

        // ---- width / pan / out
        let (mut l, mut r) = (op[0] * dg, op[1] * dg);
        if pingish || b.style == S_RHYTHM {
            let s = 1.0 - width.min(1.0);
            let (l0, r0) = (l, r);
            l = l0 + s * r0;
            r = r0 + s * l0;
            if width > 1.0 {
                let m = 0.5 * (l + r);
                let sd = 0.5 * (l - r) * width;
                l = m + sd;
                r = m - sd;
            }
        } else {
            let m = 0.5 * (l + r);
            let sd = 0.5 * (l - r) * width;
            l = m + sd;
            r = m - sd;
        }
        let pz = (pan + pan_mod * ml).clamp(-1.0, 1.0);
        // Balance: the far side fades out, the near side stays at 0 dB.
        let (gl, gr) = ((1.0 - pz).min(1.0), (1.0 + pz).min(1.0));
        let inv = if p.inv.value() { -1.0 } else { 1.0 };
        let g = out_g * self.gate * inv;
        LaneOut { l: l * gl * g, r: r * gr * g, rep_l: lp[0], rep_r: lp[1] }
    }

    #[inline]
    fn pitch_for(&self, b: &Blk, ch: usize, pm: f32, ml: f32, mr: f32) -> f32 {
        let det = if ch == 0 { b.pspread } else { -b.pspread };
        if b.kind == 16 {
            // GHOST: a little detune each pass, opposite per side.
            let d = b.detune * if ch == 0 { 1.0 } else { -1.0 };
            return b.pitch_st + d + det + pm * if ch == 0 { ml } else { mr };
        }
        b.pitch_st + det + pm * if ch == 0 { ml } else { mr }
    }
}
