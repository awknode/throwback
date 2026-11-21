//! Small DSP building blocks for ThrowBack: delay line, filters, diffuser,
//! pitch shifter, LFO, envelope follower, noise.

use std::f32::consts::PI;

/// Stereo-independent circular delay line with cubic (Hermite) reads.
pub struct Line {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
}

impl Line {
    pub fn new(max_samples: usize) -> Self {
        let n = (max_samples + 8).next_power_of_two();
        Self { buf: vec![0.0; n], mask: n - 1, w: 0 }
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
    }
    #[inline]
    pub fn write(&mut self, x: f32) {
        self.buf[self.w] = x;
        self.w = (self.w + 1) & self.mask;
    }
    /// Sample from `d` samples ago (d >= 1; the newest written sample is d = 1).
    #[inline]
    pub fn read(&self, d: f32) -> f32 {
        let d = d.clamp(1.0, (self.buf.len() - 4) as f32);
        let p = self.w as f32 - d;
        let fi = p.floor();
        let f = p - fi;
        let i = fi as isize;
        let m = self.mask as isize;
        let at = |k: isize| self.buf[((i + k) & m) as usize];
        let (y0, y1, y2, y3) = (at(-1), at(0), at(1), at(2));
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
        ((c3 * f + c2) * f + c1) * f + y1
    }
}

/// Topology-preserving state variable filter (Simper). Gives LP/BP/HP at once.
#[derive(Clone, Copy, Default)]
pub struct Svf {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
    ic1: f32,
    ic2: f32,
}

pub struct SvfOut {
    pub lp: f32,
    pub bp: f32,
    pub hp: f32,
}

impl Svf {
    pub fn set(&mut self, fc: f32, q: f32, sr: f32) {
        let fc = fc.clamp(10.0, sr * 0.49);
        let g = (PI * fc / sr).tan();
        self.k = 1.0 / q.max(0.05);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }
    #[inline]
    pub fn tick(&mut self, x: f32) -> SvfOut {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        if !self.ic1.is_finite() || !self.ic2.is_finite() {
            self.ic1 = 0.0;
            self.ic2 = 0.0;
        }
        SvfOut { lp: v2, bp: v1, hp: x - self.k * v1 - v2 }
    }
    pub fn clear(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }
}

/// One-pole low-pass.
#[derive(Clone, Copy, Default)]
pub struct OnePole {
    a: f32,
    z: f32,
}
impl OnePole {
    pub fn set(&mut self, fc: f32, sr: f32) {
        self.a = 1.0 - (-2.0 * PI * fc.clamp(1.0, sr * 0.49) / sr).exp();
    }
    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
    pub fn clear(&mut self) {
        self.z = 0.0;
    }
}

/// Peaking biquad (RBJ), used for the tape head bump.
#[derive(Clone, Copy, Default)]
pub struct Peak {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}
impl Peak {
    pub fn set(&mut self, fc: f32, q: f32, db: f32, sr: f32) {
        let a = 10f32.powf(db / 40.0);
        let w = 2.0 * PI * fc / sr;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q);
        let a0 = 1.0 + al / a;
        self.b0 = (1.0 + al * a) / a0;
        self.b1 = -2.0 * c / a0;
        self.b2 = (1.0 - al * a) / a0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - al / a) / a0;
    }
    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
    pub fn clear(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Cheap fast noise (xorshift), -1..1.
#[derive(Clone, Copy)]
pub struct Noise(pub u32);
impl Noise {
    #[inline]
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    #[inline]
    pub fn uni(&mut self) -> f32 {
        self.next() * 0.5 + 0.5
    }
}

/// Fast tanh (Padé 3/2, exact 1.0 from |x| = 3 up). Smooth, ~10x cheaper than tanh().
#[inline]
pub fn ftanh(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

/// Soft safety clip used inside feedback loops: linear up to 1, then rounds
/// off so even 110% feedback can't run away to infinity.
#[inline]
pub fn loop_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 1.0 {
        x
    } else {
        x.signum() * (1.0 + ftanh(a - 1.0))
    }
}

/// Saturation with unity small-signal gain: rounds off peaks only.
#[inline]
pub fn sat(x: f32, amt: f32) -> f32 {
    if amt <= 1e-4 {
        return x;
    }
    let a = 1.0 + 4.0 * amt;
    ftanh(x * a) / a
}

// ------------------------------------------------------------------ diffuser

const MAX_STAGES: usize = 12;
/// Spread of allpass lengths (fractions of SIZE), mutually prime-ish.
const AP_RATIO: [f32; MAX_STAGES] = [1.0, 0.771, 0.618, 0.533, 0.437, 0.379, 0.311, 0.263, 0.217, 0.181, 0.149, 0.127];

struct Allpass {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
}
impl Allpass {
    /// `di` whole samples + `f` fraction (f = 0 keeps it lossless).
    #[inline]
    fn tick(&mut self, x: f32, di: usize, f: f32, g: f32) -> f32 {
        let i = self.w.wrapping_sub(di + 1) & self.mask;
        let a = self.buf[i];
        let z = if f > 0.0 { a + (self.buf[(i + 1) & self.mask] - a) * (1.0 - f) } else { self.buf[(i + 1) & self.mask] };
        let v = x + g * z;
        self.buf[self.w] = v;
        self.w = (self.w + 1) & self.mask;
        z - g * v
    }
}

/// Chain of up to 12 allpasses per channel. SIZE sets the longest one, SPREAD
/// detunes the right channel's lengths, MOTION wobbles them slowly.
pub struct Diffuser {
    ap: [Vec<Allpass>; 2],
    ph: f32,
    sr: f32,
    /// per channel, per stage: whole samples and fraction (refreshed every 32 samples)
    len: [[(usize, f32); MAX_STAGES]; 2],
    tick_n: u32,
}
impl Diffuser {
    pub fn new(sr: f32) -> Self {
        let n = ((0.26 * sr) as usize + 8).next_power_of_two();
        let mk = || (0..MAX_STAGES).map(|_| Allpass { buf: vec![0.0; n], mask: n - 1, w: 0 }).collect();
        Self { ap: [mk(), mk()], ph: 0.0, sr, len: [[(1, 0.0); MAX_STAGES]; 2], tick_n: 0 }
    }
    pub fn clear(&mut self) {
        for ch in self.ap.iter_mut() {
            for a in ch.iter_mut() {
                a.buf.iter_mut().for_each(|v| *v = 0.0);
            }
        }
    }
    /// amount 0..1, size_s seconds, stages 2..12, motion 0..1, spread 0..1.
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32, amount: f32, size_s: f32, stages: usize, motion: f32, spread: f32) -> (f32, f32) {
        if amount <= 0.005 {
            return (l, r);
        }
        let stages = stages.clamp(2, MAX_STAGES);
        if self.tick_n % 32 == 0 {
            self.ph = (self.ph + 32.0 * 0.37 / self.sr) % 1.0;
            let base = size_s * self.sr;
            let maxd = (self.ap[0][0].mask - 4) as f32;
            for ch in 0..2 {
                for k in 0..stages {
                    let sp = if ch == 1 { 1.0 + spread * 0.13 * if k % 2 == 0 { 1.0 } else { -0.8 } } else { 1.0 };
                    let wob = if motion > 0.01 { 1.0 + motion * 0.015 * (2.0 * PI * (self.ph + k as f32 * 0.19 + ch as f32 * 0.5)).sin() } else { 1.0 };
                    let d = (base * AP_RATIO[k] * sp * wob).clamp(1.0, maxd);
                    // Whole-sample delays keep the allpass lossless; only MOTION needs fractions.
                    self.len[ch][k] = if motion > 0.01 { (d as usize, d.fract()) } else { (d.round() as usize, 0.0) };
                }
            }
        }
        self.tick_n = self.tick_n.wrapping_add(1);
        let g = (amount * 0.72).min(0.72);
        let mut x = [l, r];
        for (ch, xs) in x.iter_mut().enumerate() {
            let mut v = *xs;
            let lens = &self.len[ch];
            for (k, a) in self.ap[ch].iter_mut().take(stages).enumerate() {
                let (di, f) = lens[k];
                v = a.tick(v, di.max(1), f, g);
            }
            *xs = v;
        }
        (x[0], x[1])
    }
}

// ------------------------------------------------------------------- pitch

/// Two-grain pitch shifter with Hann windows (unity gain, so it is safe inside
/// a feedback loop). Grain length is adjustable 10..200 ms.
pub struct Pitch {
    buf: Vec<f32>,
    w: usize,
    phase: f32,
}
impl Pitch {
    pub fn new(sr: f32) -> Self {
        Self { buf: vec![0.0; ((0.21 * sr) as usize + 8).next_power_of_two()], w: 0, phase: 0.0 }
    }
    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
    }
    #[inline]
    fn read(&self, d: f32) -> f32 {
        let n = self.buf.len();
        let p = self.w as f32 + n as f32 - d;
        let i = p as usize;
        let f = p - i as f32;
        let a = self.buf[i % n];
        let b = self.buf[(i + 1) % n];
        a + (b - a) * f
    }
    /// `win` = grain length in samples. Average extra delay is win / 2.
    #[inline]
    pub fn tick(&mut self, x: f32, ratio: f32, win: f32) -> f32 {
        let n = self.buf.len();
        self.buf[self.w] = x;
        self.w = (self.w + 1) % n;
        let win = win.clamp(16.0, (n - 4) as f32);
        self.phase = (self.phase + (1.0 - ratio) / win).rem_euclid(1.0);
        let p2 = (self.phase + 0.5) % 1.0;
        let g1 = (PI * self.phase).sin().powi(2);
        let g2 = (PI * p2).sin().powi(2);
        self.read(self.phase * win + 1.0) * g1 + self.read(p2 * win + 1.0) * g2
    }
}

// --------------------------------------------------------------------- LFO

/// LFO shapes: 0 sine, 1 triangle, 2 random (smoothed), 3 tape wow, 4 envelope.
pub struct Lfo {
    pub ph: f32,
    rnd: [f32; 2],
    rnd_t: [f32; 2],
    last_k: [i64; 2],
    noise: Noise,
    drift: f32,
}
impl Lfo {
    pub fn new(seed: u32) -> Self {
        Self { ph: 0.0, rnd: [0.0; 2], rnd_t: [0.0; 2], last_k: [-1; 2], noise: Noise(seed | 1), drift: 0.0 }
    }
    /// Advances by `hz` and returns (L, R) in -1..1. `env` is 0..1 for shape 4.
    #[inline]
    pub fn tick(&mut self, shape: i32, hz: f32, stereo: f32, drift: f32, env: f32, sr: f32) -> (f32, f32) {
        self.drift += (self.noise.next() * 0.02 - self.drift * 0.0005) * drift;
        self.drift = self.drift.clamp(-0.5, 0.5);
        self.ph = (self.ph + hz * (1.0 + self.drift) / sr).rem_euclid(1.0);
        let mut out = [0.0f32; 2];
        for (ch, o) in out.iter_mut().enumerate() {
            let p = (self.ph + if ch == 1 { stereo } else { 0.0 }).rem_euclid(1.0);
            *o = match shape {
                0 => (2.0 * PI * p).sin(),
                1 => 1.0 - 4.0 * (p - 0.5).abs(),
                2 => {
                    let k = (p * 8.0) as i64;
                    if k != self.last_k[ch] {
                        self.last_k[ch] = k;
                        self.rnd_t[ch] = self.noise.next() * 0.9;
                    }
                    self.rnd[ch] += (self.rnd_t[ch] - self.rnd[ch]) * (40.0 / sr).min(1.0);
                    self.rnd[ch]
                }
                3 => 0.7 * (2.0 * PI * p).sin() + 0.3 * (2.0 * PI * p * 7.3).sin(),
                _ => env * 2.0 - 1.0,
            };
        }
        (out[0], out[1])
    }
}

/// Peak envelope follower with attack/release in ms.
#[derive(Clone, Copy, Default)]
pub struct Follower {
    pub env: f32,
    att: f32,
    rel: f32,
}
impl Follower {
    pub fn set(&mut self, att_ms: f32, rel_ms: f32, sr: f32) {
        self.att = 1.0 - (-1.0 / (att_ms.max(0.05) * 0.001 * sr)).exp();
        self.rel = 1.0 - (-1.0 / (rel_ms.max(1.0) * 0.001 * sr)).exp();
    }
    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let c = if a > self.env { self.att } else { self.rel };
        self.env += c * (a - self.env);
        self.env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_reads_integer_delay_exactly() {
        let mut l = Line::new(100);
        for i in 0..50 {
            l.write(i as f32);
        }
        assert!((l.read(1.0) - 49.0).abs() < 1e-4);
        assert!((l.read(10.0) - 40.0).abs() < 1e-4);
        assert!((l.read(10.5) - 39.5).abs() < 1e-3);
    }

    #[test]
    fn pitch_shifter_unity_gain_on_sine() {
        let sr = 48000.0;
        let mut p = Pitch::new(sr);
        let (mut ein, mut eout) = (0.0f64, 0.0f64);
        for i in 0..96000 {
            let x = (2.0 * PI * 220.0 * i as f32 / sr).sin() * 0.5;
            let y = p.tick(x, 2.0f32.powf(7.0 / 12.0), 0.06 * sr);
            if i > 10000 {
                ein += (x * x) as f64;
                eout += (y * y) as f64;
            }
        }
        let db = 10.0 * (eout / ein).log10();
        // Two grains of unrelated phase lose a little; it must never gain (loop safety).
        assert!(db < 0.5 && db > -4.0, "shifter gain {db:.2} dB");
    }

    #[test]
    fn diffuser_is_energy_neutral() {
        let sr = 48000.0;
        let mut d = Diffuser::new(sr);
        let mut e = 0.0f64;
        for i in 0..96000 {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let (l, r) = d.tick(x, x, 0.8, 0.05, 8, 0.0, 0.5);
            e += (l * l + r * r) as f64 * 0.5;
        }
        assert!((e - 1.0).abs() < 0.05, "allpass chain energy {e:.3}");
    }
}
