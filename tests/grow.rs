use throw_back::*;
const SR: f32 = 48000.0;
#[test]
fn grow_scan() {
    for route in [0, 2] {
    for kind in 0..20 {
        for fb in [0.9f32, 0.98, 1.0] {
            let mut tb = ThrowBack::for_test(SR, 120.0);
            tb.force_kind = [Some(kind), Some(kind)];
            tb.force_route = Some(route);
            let p = tb.params_arc();
            p.a.fb.smoothed.reset(fb);
            p.b.fb.smoothed.reset(fb);
            p.safe.value();
            let mut seed = 3u32;
            let mut rms = vec![];
            let mut l = vec![0.0f32; 4800]; let mut r = vec![0.0f32; 4800];
            for blk in 0..(40 * 10) {
                for k in 0..4800 { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); let x = if blk < 3 { ((seed >> 8) as f32 / 16777216.0 - 0.5) } else { 0.0 }; l[k] = x; r[k] = x; }
                tb.run(&mut l, &mut r, None);
                let e = l.iter().chain(r.iter()).map(|x| x * x).sum::<f32>() / 9600.0;
                rms.push(10.0 * e.max(1e-20).log10());
            }
            // 1-second windows after the input stops
            let sec: Vec<f32> = rms.chunks(10).map(|c| c.iter().cloned().fold(-200.0, f32::max)).collect();
            let first = sec[1];
            let later = sec[3..].iter().cloned().fold(-200.0, f32::max);
            if later > first + 1.0 {
                panic!("GROWS route {route} type {kind:2} {} fb {fb}: 1 s {first:.1} dB -> later max {later:.1} dB (end {:.1})", TYPES[kind as usize].0, sec.last().unwrap());
            }
        }
    }
    }
}
