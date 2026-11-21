//! ThrowBack — awk studios. Two-lane parallel delay; the dry is never processed.
//!
//! out = DRY (level only, never touched) + MIX x (lane A + lane B), then SAFE.
//! ROUTING: PARALLEL (both lanes hear the input), SERIAL A›B (B hears A's
//! output), CROSS-FEED (each lane's repeats also feed the other's loop).
//! Header MIX only scales the lanes, so the dry level never moves.
//! Power = host bypass (20 ms ramp). Latency 0: oversampling only runs on the
//! DRIVE stage, and its delay is taken out of the delay time.

use nih_plug::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

mod awk_ui;
pub mod dsp;
mod editor;
pub mod lane;
pub mod model;
mod oversample;
pub mod params;
mod presets;

pub use lane::{Blk, Lane};
pub use params::*;

/// What the editor reads.
pub struct Shared {
    pub meters: awk_ui::Meters,
    /// Lane output peaks (dB) and ducking gain (dB).
    pub lane_db: [AtomicU32; 2],
    pub duck_db: [AtomicU32; 2],
    pub bpm: AtomicU32,
}
impl Default for Shared {
    fn default() -> Self {
        let z = || AtomicU32::new((-100f32).to_bits());
        Self { meters: awk_ui::Meters::default(), lane_db: [z(), z()], duck_db: [AtomicU32::new(0), AtomicU32::new(0)], bpm: AtomicU32::new(120f32.to_bits()) }
    }
}
impl Shared {
    pub fn get(a: &AtomicU32) -> f32 {
        f32::from_bits(a.load(Ordering::Relaxed))
    }
}

pub struct ThrowBack {
    params: Arc<TbParams>,
    shared: Arc<Shared>,
    sr: f32,
    bpm: f32,
    lanes: Option<[Lane; 2]>,
    byp: awk_ui::BypassRamp,
    ceil: awk_ui::Ceiling,
    meter: awk_ui::PeakFollower,
    lane_pk: [f32; 2],
    duck_min: [f32; 2],
    gcache: [GainCache; 3],
    /// Tests: change the per-block lane settings after they are worked out.
    pub tweak: Option<Box<dyn Fn(usize, &mut Blk) + Send>>,
    /// Tests: force each lane's echo type.
    pub force_kind: [Option<i32>; 2],
    /// Tests: force the ROUTING.
    pub force_route: Option<i32>,
}

impl Default for ThrowBack {
    fn default() -> Self {
        Self {
            params: Arc::new(TbParams::default()),
            shared: Arc::new(Shared::default()),
            sr: 48000.0,
            bpm: 120.0,
            lanes: None,
            byp: awk_ui::BypassRamp::default(),
            ceil: awk_ui::Ceiling::default(),
            meter: awk_ui::PeakFollower::default(),
            lane_pk: [0.0; 2],
            duck_min: [0.0; 2],
            gcache: [GainCache::default(); 3],
            tweak: None,
            force_kind: [None; 2],
            force_route: None,
        }
    }
}

/// Lane audibility with solo rules: (switched on, heard).
pub fn audible(p: &TbParams, i: usize) -> (bool, bool) {
    let l = p.lane(i);
    let any_solo = p.a.solo.value() || p.b.solo.value();
    let on = l.on.value();
    (on, on && !l.mute.value() && (!any_solo || l.solo.value()))
}

impl ThrowBack {
    /// Runs the whole plugin on plain slices (used by `process` and the tests).
    pub fn run(&mut self, l: &mut [f32], r: &mut [f32], sc: Option<(&[f32], &[f32])>) {
        let sr = self.sr;
        let p = self.params.clone();
        let os = p.os.value();
        let mut blk = [Blk::build(&p.a, self.bpm, sr, os, self.force_kind[0]), Blk::build(&p.b, self.bpm, sr, os, self.force_kind[1])];
        if let Some(t) = &self.tweak {
            t(0, &mut blk[0]);
            t(1, &mut blk[1]);
        }
        let lanes = self.lanes.get_or_insert_with(|| [Lane::new(sr, 1), Lane::new(sr, 2)]);
        lanes[0].setup(&blk[0], &p.a);
        lanes[1].setup(&blk[1], &p.b);
        let route = self.force_route.unwrap_or(p.route.value());
        let aud = [audible(&p, 0), audible(&p, 1)];
        let bypassed = p.bypass.value();
        let dry_on = p.dry_on.value();
        let safe = p.safe.value();
        let mut last = [(0.0f32, 0.0f32); 2];
        for i in 0..l.len() {
            let (il, ir) = (l[i], r[i]);
            let mix = p.mix.smoothed.next();
            let dry_g = self.gcache[0].get(p.dry.smoothed.next()) * if dry_on { 1.0 } else { 0.0 };
            let cross = p.cross.smoothed.next() * if route == 2 { 1.0 } else { 0.0 };
            let send = [self.gcache[1].get(p.a.send.smoothed.next()), self.gcache[2].get(p.b.send.smoothed.next())];
            let (sl, sr_) = sc.map(|(a, b)| (a[i], b[i])).unwrap_or((0.0, 0.0));
            let src = |h: i32, other: (f32, f32)| match h {
                1 => sl.abs().max(sr_.abs()),
                2 => other.0.abs().max(other.1.abs()),
                _ => il.abs().max(ir.abs()),
            };
            let on_a = aud[0].0;
            let on_b = aud[1].0;
            let in_a = if on_a { (il * send[0], ir * send[0]) } else { (0.0, 0.0) };
            lanes[0].cross = cross;
            lanes[1].cross = cross;
            let xa = (lanes[1].rep_prev[0], lanes[1].rep_prev[1]);
            let xb = (lanes[0].rep_prev[0], lanes[0].rep_prev[1]);
            let a = lanes[0].tick(&blk[0], &p.a, in_a, xa, src(blk[0].hears, last[1]), on_a, aud[0].1);
            let in_b = if !on_b {
                (0.0, 0.0)
            } else if route == 1 {
                (a.l * send[1], a.r * send[1])
            } else {
                (il * send[1], ir * send[1])
            };
            let b = lanes[1].tick(&blk[1], &p.b, in_b, xb, src(blk[1].hears, last[0]), on_b, aud[1].1);
            last = [(a.l, a.r), (b.l, b.r)];
            self.lane_pk[0] = self.lane_pk[0].max(a.l.abs().max(a.r.abs()));
            self.lane_pk[1] = self.lane_pk[1].max(b.l.abs().max(b.r.abs()));
            let wet = self.byp.next(bypassed);
            let ol = il * dry_g + (a.l + b.l) * mix;
            let or = ir * dry_g + (a.r + b.r) * mix;
            // bypass crossfades to the untouched input
            let ol = il + (ol - il) * wet;
            let or = ir + (or - ir) * wet;
            let (ol, or) = self.ceil.tick(ol, or, safe);
            self.meter.feed(il, ir, ol, or);
            l[i] = ol;
            r[i] = or;
        }
        for k in 0..2 {
            self.duck_min[k] = self.duck_min[k].min(lanes[k].duck_db);
        }
    }

    fn publish(&mut self) {
        let sh = &self.shared;
        self.meter.store(&sh.meters);
        self.ceil.store(&sh.meters);
        for k in 0..2 {
            sh.lane_db[k].store((20.0 * self.lane_pk[k].max(1e-6).log10()).to_bits(), Ordering::Relaxed);
            sh.duck_db[k].store(self.duck_min[k].to_bits(), Ordering::Relaxed);
            self.lane_pk[k] = 0.0;
            self.duck_min[k] = 0.0;
        }
        sh.bpm.store(self.bpm.to_bits(), Ordering::Relaxed);
    }

    /// For tests: a ready-to-run instance at `sr` and `bpm`.
    pub fn for_test(sr: f32, bpm: f32) -> Self {
        let mut t = Self::default();
        t.sr = sr;
        t.bpm = bpm;
        t.byp.set_sr(sr);
        t.ceil.set_sr(sr);
        t.meter.set_sr(sr);
        let p = t.params.clone();
        for f in [&p.mix, &p.dry, &p.cross] {
            f.smoothed.reset(f.value());
        }
        for l in [&p.a, &p.b] {
            for f in [&l.send, &l.fb, &l.modu, &l.width, &l.pan, &l.out] {
                f.smoothed.reset(f.value());
            }
        }
        t
    }
    pub fn params_arc(&self) -> Arc<TbParams> {
        self.params.clone()
    }
}

impl Plugin for ThrowBack {
    const NAME: &'static str = "ThrowBack";
    const VENDOR: &'static str = "awk studios";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[new_nonzero_u32(2)],
            names: PortNames { layout: Some("Stereo"), main_input: Some("Input"), main_output: Some("Output"), aux_inputs: &["Duck (sidechain)"], aux_outputs: &[] },
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout { main_input_channels: NonZeroU32::new(2), main_output_channels: NonZeroU32::new(2), ..AudioIOLayout::const_default() },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, cfg: &BufferConfig, _ctx: &mut impl InitContext<Self>) -> bool {
        if (cfg.sample_rate - self.sr).abs() > 0.5 || self.lanes.is_none() {
            self.sr = cfg.sample_rate;
            // Allocate here, never on the audio thread.
            self.lanes = Some([Lane::new(self.sr, 1), Lane::new(self.sr, 2)]);
        }
        self.byp.set_sr(self.sr);
        self.ceil.set_sr(self.sr);
        self.meter.set_sr(self.sr);
        true
    }

    fn reset(&mut self) {
        if let Some(l) = self.lanes.as_mut() {
            l.iter_mut().for_each(Lane::clear);
        }
    }

    fn process(&mut self, buffer: &mut Buffer, aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        if let Some(b) = context.transport().tempo {
            if b > 1.0 {
                self.bpm = b as f32;
            }
        }
        let n = buffer.samples();
        let sl = buffer.as_slice();
        if sl.len() < 2 {
            return ProcessStatus::Normal;
        }
        let (a, b) = sl.split_at_mut(1);
        let (l, r) = (&mut *a[0], &mut *b[0]);
        let sc = aux.inputs.first_mut().and_then(|s| {
            let ch = s.as_slice_immutable();
            if ch.len() >= 2 && ch[0].len() >= n {
                Some((&*ch[0], &*ch[1]))
            } else {
                None
            }
        });
        // SAFETY of borrows: sc slices live in aux, l/r in buffer.
        let sc: Option<(&[f32], &[f32])> = sc.map(|(x, y)| (&x[..n], &y[..n]));
        self.run(l, r, sc);
        self.publish();
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for ThrowBack {
    const CLAP_ID: &'static str = "com.awknode.throwback";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Two-lane parallel delay");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Delay, ClapFeature::Stereo];
}

impl Vst3Plugin for ThrowBack {
    const VST3_CLASS_ID: [u8; 16] = *b"AwknodeThrowBk01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Delay];
}

nih_export_clap!(ThrowBack);
nih_export_vst3!(ThrowBack);
