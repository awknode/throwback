//! What the screens draw: where each repeat lands and how loud, worked out
//! from the same settings the engine uses (Blk), so the picture matches the sound.

use crate::lane::{filt_db, Blk};
use crate::params::*;

const SR: f32 = 48000.0;

#[derive(Clone, Copy)]
pub struct Echo {
    /// seconds after the hit
    pub t: f32,
    /// 0 = L, 1 = R
    pub ch: usize,
    pub db: f32,
    /// repeat number (1 = first)
    pub n: u32,
    /// smear width in seconds (diffusion)
    pub w: f32,
    pub rev: bool,
    /// pitch of this repeat in semitones
    pub st: f32,
}

pub struct Train {
    pub echoes: Vec<Echo>,
    pub b: Blk,
    /// base time (s)
    pub t: f32,
    /// time between passes through the loop (s)
    pub loop_t: f32,
    /// level change per pass (dB)
    pub per: f32,
    pub first: f32,
    pub on: bool,
}

impl Train {
    pub fn rt60(&self) -> f32 {
        if self.b.freeze {
            f32::INFINITY
        } else if self.per < -0.01 {
            60.0 / -self.per * self.loop_t
        } else {
            f32::INFINITY
        }
    }
    pub fn runaway(&self) -> bool {
        !self.b.freeze && self.per >= -0.01
    }
}

fn gdb(g: f32) -> f32 {
    if g <= 1e-5 {
        f32::NEG_INFINITY
    } else {
        20.0 * g.log10()
    }
}

/// Mid-band loss of one filter pass (weighted around 1.2 kHz, like the ear).
pub fn filt_loss(b: &Blk) -> f32 {
    let (mut s, mut sw) = (0.0f32, 0.0f32);
    for i in 0..48 {
        let f = 20.0 * 1000f32.powf((i as f32 + 0.5) / 48.0);
        let w = (-((f / 1200.0).log2().powi(2)) / 8.0).exp();
        s += w * 10f32.powf(filt_db(f, b) / 10.0);
        sw += w;
    }
    10.0 * (s / sw).log10()
}

pub fn train(p: &LaneParams, bpm: f32, os: i32) -> Train {
    let b = Blk::from(p, bpm, SR, os);
    let t = b.t / SR;
    let s16 = b.s16 / SR;
    let fb = if b.freeze { 1.0 } else if b.kind == T_SLAP { p.fb.value() * 0.35 } else { p.fb.value() };
    let floss = filt_loss(&b);
    let tloss = TYPES[b.kind as usize].2;
    let per = if b.freeze { 0.0 } else { gdb(fb) + tloss + if b.inloop { floss } else { 0.0 } };
    let send = level_gain(p.send.value());
    let out = level_gain(p.out.value());
    let first = gdb(send) + gdb(out) + floss + tloss;
    let pan = p.pan.value();
    let (pl, pr) = (gdb((1.0 - pan).min(1.0)), gdb((1.0 + pan).min(1.0)));
    let width = p.width.value();
    let spread_db = gdb(1.0 - width.min(1.0));
    let loop_t = if b.kind == T_MULTI {
        3.0 * t
    } else if b.style == S_RHYTHM {
        16.0 * s16
    } else {
        t
    };
    let mut tr = Train { echoes: Vec::new(), b, t, loop_t, per, first, on: p.on.value() };
    if !first.is_finite() {
        return tr;
    }
    let mut seed = 99u32 + b.kind as u32 * 7;
    let mut rnd = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / 16777216.0
    };
    let st_n = |n: u32, ch: usize| -> f32 {
        match b.pmode {
            0 => b.pitch_st,
            1 => b.pitch_st * n as f32,
            _ => {
                if ch == 0 {
                    b.pitch_st
                } else {
                    -b.pitch_st
                }
            }
        }
    };
    let smear = |n: u32| if b.diff < 0.01 { 0.0 } else { b.diff * b.dsize * 1.5 * if b.dpos_loop { (n as f32).sqrt() } else { 1.0 } };
    let feel = b.feel / SR;
    let off = b.off / SR;
    let rev = b.kind == T_REVERSE;
    let runaway = !b.freeze && per >= 0.0;
    let mut add = |t: f32, ch: usize, db: f32, n: u32, list: &mut Vec<Echo>| {
        if !(db > -60.0) || t < 0.0 || t > 16.0 {
            return;
        }
        let db = if runaway { db.min(0.0) } else { db };
        list.push(Echo { t, ch, db, n, w: smear(n), rev, st: st_n(n, ch) });
    };
    let mut list = Vec::new();
    let max_n = 400u32;
    let mut acc_ratio = 0.0f32; // RATIO style running time
    for n in 1..=max_n {
        let lv = first + (n - 1) as f32 * per;
        if lv < -60.0 {
            break;
        }
        if b.kind == T_MULTI {
            let base = (n - 1) as f32 * 3.0 * t;
            if base > 16.0 {
                break;
            }
            for (h, hd) in [(1.0f32, 0.0f32), (2.0, -3.0), (3.0, -6.0)] {
                add(base + h * t, 0, lv + hd + pl, n, &mut list);
                add(base + h * t, 1, lv + hd + pr, n, &mut list);
            }
            continue;
        }
        let jit = if b.kind == T_GLITCH { ((rnd() - 0.5) * 2.0).round() * s16 } else { 0.0 };
        match b.style {
            S_DUAL => {
                let tl = n as f32 * t + feel + jit;
                let trr = n as f32 * (t * b.ratio + off) + feel + jit;
                if tl > 16.0 && trr > 16.0 {
                    break;
                }
                add(tl, 0, lv + pl, n, &mut list);
                add(trr, 1, lv + pr, n, &mut list);
            }
            S_PING | S_RATIO => {
                let ch = ((n - 1) % 2) as usize;
                let (dl, dr) = if b.style == S_PING { (t, t) } else { (t, t * b.ratio) };
                acc_ratio += if ch == 0 { dl } else { dr };
                let mut tt = acc_ratio + feel + jit;
                if ch == 1 {
                    tt += b.swing * if b.style == S_PING { t } else { dr } + off;
                }
                if tt > 16.0 {
                    break;
                }
                let side = if ch == 0 { pl } else { pr };
                add(tt, ch, lv + side, n, &mut list);
                if spread_db.is_finite() {
                    add(tt, 1 - ch, lv + spread_db + if ch == 0 { pr } else { pl }, n, &mut list);
                }
            }
            S_RHYTHM => {
                let bar = 16.0 * s16;
                let base = (n - 1) as f32 * bar;
                if base > 16.0 {
                    break;
                }
                for k in 0..16 {
                    let g = b.steps[k];
                    if g <= 0.0 {
                        continue;
                    }
                    let mut tt = base + (k as f32 + 1.0) * s16 + feel;
                    if k % 2 == 0 {
                        tt += b.swing * 2.0 * s16;
                    }
                    let ch = k % 2;
                    add(tt, ch, lv + gdb(g) + if ch == 0 { pl } else { pr }, n, &mut list);
                    if spread_db.is_finite() {
                        add(tt, 1 - ch, lv + gdb(g) + spread_db, n, &mut list);
                    }
                }
            }
            _ => {
                let tt = n as f32 * t + feel + jit;
                if tt > 16.0 {
                    break;
                }
                add(tt, 0, lv + pl, n, &mut list);
                add(tt, 1, lv + pr, n, &mut list);
            }
        }
        if b.kind == T_SLAP && n >= 2 {
            break;
        }
    }
    tr.echoes = list;
    tr
}

/// Both lanes with the routing applied (serial: B's repeats ride A's; cross: each
/// lane's repeats also appear in the other one).
pub fn all(p: &TbParams, bpm: f32) -> [Train; 2] {
    let os = p.os.value();
    let mut a = train(&p.a, bpm, os);
    let mut b = train(&p.b, bpm, os);
    match p.route.value() {
        1 if !a.echoes.is_empty() && !b.echoes.is_empty() => {
            // B hears A's output instead of the input.
            let out_a = gdb(level_gain(p.a.out.value()));
            let _ = out_a;
            let mut comb = Vec::new();
            for ea in a.echoes.iter().filter(|e| e.db > -40.0).take(40) {
                for eb in b.echoes.iter() {
                    if eb.ch != ea.ch {
                        continue;
                    }
                    let db = ea.db + eb.db;
                    if db > -60.0 {
                        comb.push(Echo { t: ea.t + eb.t, db, n: ea.n + eb.n, ..*eb });
                    }
                }
            }
            comb.truncate(1500);
            b.echoes = comb;
        }
        2 => {
            let cx = p.cross.value();
            if cx > 0.0 {
                let cdb = gdb(cx);
                let oa = gdb(level_gain(p.a.out.value()));
                let ob = gdb(level_gain(p.b.out.value()));
                let xb: Vec<Echo> = a.echoes.iter().take(40).map(|e| Echo { t: e.t + b.t, db: e.db + cdb + ob - oa, ..*e }).filter(|e| e.db > -60.0).collect();
                let xa: Vec<Echo> = b.echoes.iter().take(40).map(|e| Echo { t: e.t + a.t, db: e.db + cdb + oa - ob, ..*e }).filter(|e| e.db > -60.0).collect();
                a.echoes.extend(xa);
                b.echoes.extend(xb);
            }
        }
        _ => {}
    }
    [a, b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lane_a_lands_on_dotted_eighths() {
        let p = TbParams::default();
        let tr = train(&p.a, 120.0, 0);
        // A = 1/8D ping-pong at 120 BPM: 0.375 s, first repeat on the left.
        let e = tr.echoes.iter().filter(|e| e.db > -59.0).min_by(|x, y| x.t.total_cmp(&y.t)).unwrap();
        assert!((e.t - 0.375).abs() < 1e-3 && e.ch == 0, "{} {}", e.t, e.ch);
        assert!(tr.rt60().is_finite());
    }
}
