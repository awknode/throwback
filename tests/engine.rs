//! ThrowBack engine checks: dry untouched, echo timing, every type and style
//! dies away, runaway feedback stays bounded, SAFE holds the ceiling,
//! freeze holds, oversampling adds no latency, CPU is reasonable.

use throw_back::*;

const SR: f32 = 48000.0;

fn noise(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
}

/// Runs `secs` of audio through `tb`, input from `sig(i) -> (l, r)`, in 512 blocks.
fn render(tb: &mut ThrowBack, secs: f32, sig: &mut dyn FnMut(usize) -> (f32, f32)) -> (Vec<f32>, Vec<f32>) {
    let n = (secs * SR) as usize;
    let (mut ol, mut or) = (Vec::with_capacity(n), Vec::with_capacity(n));
    let mut i = 0;
    while i < n {
        let m = 512.min(n - i);
        let mut l: Vec<f32> = (0..m).map(|k| sig(i + k).0).collect();
        let mut r: Vec<f32> = (0..m).map(|k| sig(i + k).1).collect();
        tb.run(&mut l, &mut r, None);
        ol.extend_from_slice(&l);
        or.extend_from_slice(&r);
        i += m;
    }
    (ol, or)
}

fn peak(v: &[f32]) -> f32 {
    v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
}
fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// A plain, clean lane: no drive, filters wide open, no modulation.
fn clean(b: &mut Blk) {
    b.kind = 0;
    b.fam = 0;
    b.ch = Default::default();
    b.style = 0;
    b.drive = 0.0;
    b.lc = 20.0;
    b.hc = 20000.0;
    b.slope = 0;
    b.res = 0.0;
    b.tilt = 0.0;
    b.ldrive = 0.0;
    b.diff = 0.0;
    b.wow = 0.0;
    b.flutter = 0.0;
    b.mtgt = 2; // pan target, depth 0 below
}

fn quiet_mod(tb: &ThrowBack) {
    let p = tb.params_arc();
    for l in [&p.a, &p.b] {
        l.modu.smoothed.reset(0.0);
    }
}

#[test]
fn dry_is_untouched_when_lanes_are_silent() {
    let mut tb = ThrowBack::for_test(SR, 120.0);
    let p = tb.params_arc();
    p.mix.smoothed.reset(0.0);
    let mut s = 3u32;
    let input: Vec<(f32, f32)> = (0..(SR as usize)).map(|_| (noise(&mut s) * 0.5, noise(&mut s) * 0.5)).collect();
    let (l, r) = render(&mut tb, 1.0, &mut |i| input[i]);
    for i in 0..l.len() {
        assert_eq!(l[i], input[i].0, "L sample {i} changed");
        assert_eq!(r[i], input[i].1, "R sample {i} changed");
    }
}

#[test]
fn clean_echo_lands_on_time() {
    // 1/8 dotted at 120 BPM = 0.375 s = 18000 samples.
    let mut tb = ThrowBack::for_test(SR, 120.0);
    tb.tweak = Some(Box::new(|i, b| {
        clean(b);
        if i == 1 {
            b.t = 1e9; // keep B out of the way: it is muted below
        }
    }));
    quiet_mod(&tb);
    let p = tb.params_arc();
    p.dry.smoothed.reset(-60.0);
    p.b.out.smoothed.reset(-60.0);
    p.a.out.smoothed.reset(0.0);
    p.a.fb.smoothed.reset(0.5);
    let (l, _) = render(&mut tb, 1.0, &mut |i| if i == 0 { (0.5, 0.5) } else { (0.0, 0.0) });
    let (pos, v) = l.iter().enumerate().fold((0, 0.0f32), |m, (i, x)| if x.abs() > m.1 { (i, x.abs()) } else { m });
    assert!((pos as i64 - 18000).abs() <= 1, "first echo at {pos}");
    assert!((v - 0.5).abs() < 0.02, "first echo level {v}");
    let second = l[35990..36010].iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!((second - 0.25).abs() < 0.02, "second echo {second} (fb 50%)");
}

#[test]
fn oversampling_adds_no_latency() {
    let first = |os: usize| {
        let mut tb = ThrowBack::for_test(SR, 120.0);
        tb.tweak = Some(Box::new(move |_, b| {
            clean(b);
            b.drive = 6.0;
            b.os_factor = os;
        }));
        quiet_mod(&tb);
        let p = tb.params_arc();
        p.dry.smoothed.reset(-60.0);
        p.b.out.smoothed.reset(-60.0);
        let (l, _) = render(&mut tb, 0.5, &mut |i| if i == 0 { (0.3, 0.3) } else { (0.0, 0.0) });
        l.iter().enumerate().fold((0, 0.0f32), |m, (i, x)| if x.abs() > m.1 { (i, x.abs()) } else { m }).0 as i64
    };
    let base = first(1);
    for os in [2, 4, 8] {
        let p = first(os);
        assert!((p - base).abs() <= 1, "os {os}x moves the echo: {p} vs {base}");
    }
}

/// Every echo type in every style: a noise burst's tail dies away, nothing blows up.
#[test]
fn every_type_and_style_dies_away() {
    for kind in 0..20 {
        for style in 0..5 {
            let mut tb = ThrowBack::for_test(SR, 140.0);
            tb.force_kind = [Some(kind), Some(kind)];
            tb.tweak = Some(Box::new(move |_, b| {
                b.style = style;
                b.t = b.t.min(0.4 * SR);
            }));
            let p = tb.params_arc();
            // RHYTHM loops once per bar (1.7 s here), so it gets less feedback
            // to fade in the same 13 s.
            let fb = if style == 4 { 0.45 } else if kind == 7 { 0.55 } else { 0.8 }; // MULTI-HEAD loops at 3x the time
            p.a.fb.smoothed.reset(fb);
            p.b.fb.smoothed.reset(fb);
            let mut s = 9u32 + kind as u32;
            let burst = (0.25 * SR) as usize;
            let (l, r) = render(&mut tb, 14.0, &mut |i| if i < burst { (noise(&mut s) * 0.7, noise(&mut s) * 0.7) } else { (0.0, 0.0) });
            assert!(l.iter().chain(r.iter()).all(|x| x.is_finite()), "type {kind} style {style}: NaN");
            assert!(peak(&l).max(peak(&r)) <= 0.9886, "type {kind} style {style}: over the ceiling");
            let tail = peak(&l[(13.0 * SR) as usize..]).max(peak(&r[(13.0 * SR) as usize..]));
            // hiss / clock noise types fade with the loop too, so -60 is a fair floor
            assert!(db(tail) < -60.0, "type {kind} style {style}: tail still {:.1} dB after 13 s", db(tail));
        }
    }
}

/// 110% feedback on every type: it grows but stays bounded (SAFE off).
#[test]
fn runaway_feedback_stays_bounded() {
    for kind in 0..20 {
        let mut tb = ThrowBack::for_test(SR, 120.0);
        tb.force_kind = [Some(kind), Some(kind)];
        let p = tb.params_arc();
        p.a.fb.smoothed.reset(1.1);
        p.b.fb.smoothed.reset(1.1);
        let mut s = 5u32;
        let (l, r) = render(&mut tb, 8.0, &mut |i| if i < 4800 { (noise(&mut s), noise(&mut s)) } else { (0.0, 0.0) });
        assert!(l.iter().chain(r.iter()).all(|x| x.is_finite()), "type {kind}: NaN");
        assert!(peak(&l).max(peak(&r)) <= 0.9886, "type {kind}: SAFE let it through");
    }
}

#[test]
fn defaults_never_go_over_zero_db() {
    let mut tb = ThrowBack::for_test(SR, 150.0);
    let mut s = 1u32;
    // full-scale drums-ish: noise hits every beat
    let beat = (SR * 60.0 / 150.0) as usize;
    let (l, r) = render(&mut tb, 10.0, &mut |i| {
        let e = (-((i % beat) as f32) / 2000.0).exp();
        (noise(&mut s) * e, noise(&mut s) * e)
    });
    assert!(peak(&l).max(peak(&r)) <= 0.9886);
}

#[test]
fn freeze_holds_the_repeats() {
    let frozen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let f2 = frozen.clone();
    let mut tb = ThrowBack::for_test(SR, 120.0);
    tb.tweak = Some(Box::new(move |_, b| {
        clean(b);
        b.freeze = f2.load(std::sync::atomic::Ordering::Relaxed);
    }));
    quiet_mod(&tb);
    let p = tb.params_arc();
    p.dry.smoothed.reset(-60.0);
    p.b.out.smoothed.reset(-60.0);
    p.a.fb.smoothed.reset(0.0);
    let mut s = 4u32;
    // Fill the line, then freeze: new input is ignored, the loop holds.
    let _ = render(&mut tb, 0.5, &mut |_| (noise(&mut s) * 0.3, noise(&mut s) * 0.3));
    frozen.store(true, std::sync::atomic::Ordering::Relaxed);
    let (l, _) = render(&mut tb, 6.0, &mut |_| (noise(&mut s) * 0.3, 0.0));
    let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
    let early = rms(&l[(0.5 * SR) as usize..(1.5 * SR) as usize]);
    let late = rms(&l[(4.5 * SR) as usize..(5.5 * SR) as usize]);
    assert!(early > 1e-3, "nothing frozen");
    assert!((db(late) - db(early)).abs() < 1.0, "freeze drifted {:.2} dB", db(late) - db(early));
}

#[test]
fn ping_pong_alternates_sides() {
    let mut tb = ThrowBack::for_test(SR, 120.0);
    tb.tweak = Some(Box::new(|_, b| {
        clean(b);
        b.style = 2;
    }));
    quiet_mod(&tb);
    let p = tb.params_arc();
    p.dry.smoothed.reset(-60.0);
    p.b.out.smoothed.reset(-60.0);
    p.a.out.smoothed.reset(0.0);
    p.a.fb.smoothed.reset(0.5);
    p.a.width.smoothed.reset(1.0);
    let (l, r) = render(&mut tb, 1.0, &mut |i| if i == 0 { (0.5, 0.5) } else { (0.0, 0.0) });
    let w = |v: &[f32], at: usize| v[at - 5..at + 5].iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(w(&l, 18000) > 0.3 && w(&r, 18000) < 0.01, "first repeat L only: {} {}", w(&l, 18000), w(&r, 18000));
    assert!(w(&r, 36000) > 0.15 && w(&l, 36000) < 0.01, "second repeat R only: {} {}", w(&l, 36000), w(&r, 36000));
}

#[test]
fn cpu_is_reasonable() {
    // Heaviest-ish settings on both lanes: cloud + chorus, diffusion in the loop, pitch, 8x drive.
    let mut tb = ThrowBack::for_test(SR, 120.0);
    tb.tweak = Some(Box::new(|i, b| {
        b.kind = if i == 0 { 19 } else { 12 };
        b.fam = (b.kind / 5) as usize;
        b.diff = 0.8;
        b.dpos_loop = true;
        b.dstages = 12;
        b.pitch_st = 7.0;
        b.pmode = 1;
        b.drive = 12.0;
        b.os_factor = 8;
        b.tchange = 1;
    }));
    let mut s = 2u32;
    let t0 = std::time::Instant::now();
    let _ = render(&mut tb, 10.0, &mut |_| (noise(&mut s) * 0.3, noise(&mut s) * 0.3));
    let el = t0.elapsed().as_secs_f32();
    println!("10 s of audio in {el:.2} s ({:.1}% of one core)", el * 10.0);
    assert!(el < 3.0, "too slow: {el:.2} s for 10 s");
}

/// CPU with the default settings (what you get when you load it).
#[test]
fn cpu_defaults() {
    let mut tb = ThrowBack::for_test(SR, 120.0);
    let mut s = 2u32;
    let t0 = std::time::Instant::now();
    let _ = render(&mut tb, 20.0, &mut |_| (noise(&mut s) * 0.3, noise(&mut s) * 0.3));
    let el = t0.elapsed().as_secs_f32();
    println!("DEFAULTS: 20 s of audio in {el:.3} s ({:.2}% of one core)", el * 5.0);
}

/// After the tail has died away (below -90 dB), a silent ThrowBack costs ~nothing
/// and puts out exact silence.
#[test]
fn sleeps_when_silent() {
    let mut tb = ThrowBack::for_test(SR, 120.0);
    let mut s = 3u32;
    let _ = render(&mut tb, 1.0, &mut |_| (noise(&mut s) * 0.5, noise(&mut s) * 0.5));
    let (l, _) = render(&mut tb, 40.0, &mut |_| (0.0, 0.0));
    let t0 = std::time::Instant::now();
    let (l2, r2) = render(&mut tb, 10.0, &mut |_| (0.0, 0.0));
    let el = t0.elapsed().as_secs_f32();
    println!("SLEEP: 10 s of silence in {el:.3} s ({:.2}% of one core); tail at 30-40 s peak {:.1} dB", el * 10.0, db(peak(&l[(30.0 * SR) as usize..])));
    assert!(l2.iter().chain(r2.iter()).all(|x| *x == 0.0));
    assert!(el < 0.05, "still working while silent: {el:.3} s");
}

/// Feedback turned right up after the music stops: nothing grows out of the
/// leftover tail (the loop gate clears it once it's under -80 dB).
#[test]
fn high_feedback_never_grows_from_silence() {
    for kind in [0, 5, 10, 13, 15, 18] {
        let mut tb = ThrowBack::for_test(SR, 120.0);
        tb.force_kind = [Some(kind), Some(kind)];
        let p = tb.params_arc();
        p.a.fb.smoothed.reset(0.5);
        p.b.fb.smoothed.reset(0.5);
        let mut s = 8u32;
        let _ = render(&mut tb, 0.5, &mut |_| (noise(&mut s) * 0.5, noise(&mut s) * 0.5));
        let _ = render(&mut tb, 20.0, &mut |_| (0.0, 0.0));
        p.a.fb.smoothed.reset(1.1);
        p.b.fb.smoothed.reset(1.1);
        let (l, r) = render(&mut tb, 60.0, &mut |_| (0.0, 0.0));
        let pk = peak(&l).max(peak(&r));
        assert!(db(pk) < -80.0, "type {kind}: grew to {:.1} dB out of silence", db(pk));
    }
}

/// CROSS-FEED with both lanes at high feedback still fades away.
#[test]
fn cross_feed_never_adds_up_over_100() {
    let mut tb = ThrowBack::for_test(SR, 120.0);
    tb.force_route = Some(2);
    let p = tb.params_arc();
    p.cross.smoothed.reset(0.6);
    p.a.fb.smoothed.reset(0.95);
    p.b.fb.smoothed.reset(0.95);
    let mut s = 8u32;
    let _ = render(&mut tb, 0.5, &mut |_| (noise(&mut s) * 0.5, noise(&mut s) * 0.5));
    let (l, r) = render(&mut tb, 90.0, &mut |_| (0.0, 0.0));
    let n = l.len();
    let pk = peak(&l[n - 48000..]).max(peak(&r[n - 48000..]));
    assert!(db(pk) < -60.0, "cross-feed tail still {:.1} dB after 90 s", db(pk));
}
