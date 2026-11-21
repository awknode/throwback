use throw_back::*;
const SR: f32 = 48000.0;
fn go(name: &str, f: fn(usize, &mut Blk), off_b: bool) { go2(name, f, off_b, false) }
fn go2(name: &str, f: fn(usize, &mut Blk), off_b: bool, silent_after: bool) {
    let mut tb = ThrowBack::for_test(SR, 120.0);
    tb.tweak = Some(Box::new(f));
    let p = tb.params_arc();
    if off_b { p.b.out.smoothed.reset(-60.0); }
    let mut seed = 2u32;
    let n = (20.0 * SR) as usize;
    let mut l = vec![0.0f32; 512]; let mut r = vec![0.0f32; 512];
    let t0 = std::time::Instant::now();
    let mut i = 0;
    while i < n {
        for k in 0..512 { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); l[k] = if silent_after && i > 48000 { 0.0 } else { ((seed >> 8) as f32 / 16777216.0 - 0.5) * 0.6 }; r[k] = l[k]; }
        tb.run(&mut l, &mut r, None);
        i += 512;
    }
    println!("PROF {name:<28} {:.2}%", t0.elapsed().as_secs_f32() * 5.0);
}
#[test]
fn prof() {
    go("defaults", |_, _| {}, false);
    go2("defaults, 1 s audio then silence", |_, _| {}, false, true);
    go("no diffusion", |_, b| b.diff = 0.0, false);
    go("no diffusion motion", |_, b| b.dmotion = 0.0, false);
    go("no mod/wow", |_, b| { b.wow = 0.0; b.flutter = 0.0; b.mtgt = 2; }, false);
    go("no drive", |_, b| b.drive = 0.0, false);
    go("no char", |_, b| b.ch = Default::default(), false);
    go("crossfade off (slide)", |_, b| b.tchange = 0, false);
    go("all of the above off", |_, b| { b.diff = 0.0; b.wow = 0.0; b.flutter = 0.0; b.mtgt = 2; b.drive = 0.0; b.ch = Default::default(); b.tchange = 0; b.slope = 0; }, false);
}
