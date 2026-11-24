//! ThrowBack parameters: global controls + two identical lanes (ids a_* / b_*).

use crate::awk_ui;
use nice_plug::prelude::*;
use nice_plug_egui::EguiEditorState;
use std::sync::Arc;

pub const NOTE_NAMES: [&str; 19] = ["1/64", "1/32T", "1/32", "1/16T", "1/32D", "1/16", "1/8T", "1/16D", "1/8", "1/4T", "1/8D", "1/4", "1/2T", "1/4D", "1/2", "1/1T", "1/2D", "1/1", "2/1"];
/// Note lengths in beats (quarter notes).
pub const NOTE_BEATS: [f32; 19] = [1.0 / 16.0, 1.0 / 12.0, 1.0 / 8.0, 1.0 / 6.0, 3.0 / 16.0, 0.25, 1.0 / 3.0, 3.0 / 8.0, 0.5, 2.0 / 3.0, 0.75, 1.0, 4.0 / 3.0, 1.5, 2.0, 8.0 / 3.0, 3.0, 4.0, 8.0];

pub const FAMS: [&str; 4] = ["DIGITAL", "TAPE", "ANALOG", "SPACE"];
/// (name, description, display loss per repeat in dB, diffusion the type adds)
pub const TYPES: [(&str, &str, f32, f32); 20] = [
    ("STUDIO CLEAN", "Exact copies of the input. Nothing added, nothing lost.", 0.0, 0.0),
    ("RACK 12-BIT", "80s rack unit: 12-bit, a little gritty, top end rolled off at 12 kHz.", -0.4, 0.0),
    ("LO-FI CRUSH", "Bit and sample-rate reduction on every repeat. Dirtier as it fades.", -0.8, 0.0),
    ("RADIO", "Band-limited to 300 Hz to 3 kHz with light distortion. Phone and radio throws.", -1.5, 0.0),
    ("GLITCH", "Repeats skip, stutter and re-trigger on the beat grid.", 0.0, 0.0),
    ("TAPE ECHO", "One tape head: warm saturation, gentle wow and flutter, darker each pass.", -0.6, 0.0),
    ("WORN TAPE", "Old stretched tape: heavier wobble, hiss and the odd dropout.", -1.0, 0.0),
    ("MULTI-HEAD", "Three playback heads at 1x, 2x and 3x the time. Space-echo patterns.", -0.6, 0.0),
    ("DRUM ECHO", "Spinning magnetic drum: dark, wobbly, smeared repeats.", -1.2, 0.2),
    ("SLAPBACK", "One or two short tape slaps. The classic vocal double.", -0.5, 0.0),
    ("BUCKET", "Analog bucket-brigade chip: dark, soft repeats with warm chorus.", -1.0, 0.0),
    ("DIRTY BUCKET", "Pushed bucket-brigade: clock noise, compander pumping, grit.", -1.3, 0.0),
    ("CHORUS ECHO", "Every repeat is chorused (3 detuned voices). Wide and lush.", -0.4, 0.05),
    ("DUB SPACE", "Resonant filter in the loop. Built for long self-oscillating throws.", -0.2, 0.0),
    ("FILTER ECHO", "A swept filter moves through the repeats (LFO or envelope).", -0.6, 0.0),
    ("DIFFUSE", "Repeats smear into each other. With feedback it turns into a reverb.", -0.3, 0.5),
    ("GHOST", "Diffused and slightly detuned. Repeats blur into a haze.", -0.5, 0.6),
    ("REVERSE", "Each repeat plays backwards and swells into the next beat.", -0.2, 0.0),
    ("SHIMMER", "Pitch shift inside the loop: every repeat climbs (or falls) again.", -0.2, 0.15),
    ("CLOUD", "Granular: repeats scatter into tiny grains across the stereo field.", -0.5, 0.4),
];
pub const T_GLITCH: i32 = 4;
pub const T_MULTI: i32 = 7;
pub const T_SLAP: i32 = 9;
pub const T_CHORUS: i32 = 12;
pub const T_REVERSE: i32 = 17;
pub const T_SHIMMER: i32 = 18;
pub const T_CLOUD: i32 = 19;
pub fn fam(t: i32) -> usize {
    (t.clamp(0, 19) / 5) as usize
}

pub const STYLES: [&str; 5] = ["SINGLE", "DUAL", "PING-PONG", "RATIO", "RHYTHM"];
pub const S_DUAL: i32 = 1;
pub const S_PING: i32 = 2;
pub const S_RATIO: i32 = 3;
pub const S_RHYTHM: i32 = 4;
pub const ROUTES: [&str; 3] = ["SIDE BY SIDE", "A › B", "CROSS-FEED"];
pub const SLOPES: [&str; 3] = ["6", "12", "24 dB"];
pub const TCHANGE: [&str; 3] = ["TAPE SLIDE", "CROSSFADE", "JUMP"];
pub const MOD_SHAPES: [&str; 5] = ["SINE", "TRIANGLE", "RANDOM", "TAPE WOW", "ENVELOPE"];
pub const MOD_TARGETS: [&str; 4] = ["TIME", "FILTER", "PAN", "PITCH"];
pub const PMODES: [&str; 3] = ["ONCE", "EVERY REPEAT", "UP / DOWN"];
pub const DPOS: [&str; 2] = ["BEFORE THE DELAY", "IN THE LOOP"];
pub const HEARS: [&str; 3] = ["DRY INPUT", "SIDECHAIN", "OTHER LANE"];
pub const STEP_NAMES: [&str; 4] = ["OFF", "-6 dB", "-2 dB", "0 dB"];
pub const STEP_GAIN: [f32; 4] = [0.0, 0.5, 0.8, 1.0];

/// level_gain with a one-value cache (the knob is usually not moving).
#[derive(Clone, Copy)]
pub struct GainCache {
    db: f32,
    g: f32,
}
impl Default for GainCache {
    fn default() -> Self {
        Self { db: f32::NAN, g: 0.0 }
    }
}
impl GainCache {
    #[inline]
    pub fn get(&mut self, db: f32) -> f32 {
        if db != self.db {
            self.db = db;
            self.g = level_gain(db);
        }
        self.g
    }
}

/// Level knobs: -60 dB = OFF.
pub fn level_gain(db: f32) -> f32 {
    if db <= -59.9 {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

fn level(name: &str, def: f32) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Linear { min: -60.0, max: 6.0 })
        .with_smoother(SmoothingStyle::Linear(30.0))
        .with_value_to_string(Arc::new(|v| if v <= -59.9 { "OFF".into() } else { format!("{v:+.1} dB") }))
        .with_string_to_value(Arc::new(|s| {
            let t = s.trim().trim_end_matches("dB").trim();
            if t.eq_ignore_ascii_case("off") {
                Some(-60.0)
            } else {
                t.parse().ok()
            }
        }))
}
fn pct(name: &str, def: f32, max: f32) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Linear { min: 0.0, max })
        .with_smoother(SmoothingStyle::Linear(30.0))
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
fn lin(name: &str, def: f32, min: f32, max: f32, unit: &'static str, dec: usize) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Linear { min, max })
        .with_smoother(SmoothingStyle::Linear(30.0))
        .with_unit(unit)
        .with_value_to_string(formatters::v2s_f32_rounded(dec))
}
fn skew(name: &str, def: f32, min: f32, max: f32, unit: &'static str, dec: usize) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-1.5) })
        .with_smoother(SmoothingStyle::Linear(30.0))
        .with_unit(unit)
        .with_value_to_string(formatters::v2s_f32_rounded(dec))
}
fn hz(name: &str, def: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, def, FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-2.0) })
        .with_smoother(SmoothingStyle::Logarithmic(30.0))
        .with_value_to_string(formatters::v2s_f32_hz_then_khz(1))
        .with_string_to_value(formatters::s2v_f32_hz_then_khz())
}
fn pan_param(name: &str) -> FloatParam {
    FloatParam::new(name, 0.0, FloatRange::Linear { min: -1.0, max: 1.0 })
        .with_smoother(SmoothingStyle::Linear(30.0))
        .with_value_to_string(Arc::new(|v| {
            let p = (v * 100.0).round() as i32;
            if p == 0 {
                "C".into()
            } else if p < 0 {
                format!("L{}", -p)
            } else {
                format!("R{p}")
            }
        }))
}

fn choice(name: &str, def: i32, names: &'static [&'static str]) -> IntParam {
    IntParam::new(name, def, IntRange::Linear { min: 0, max: names.len() as i32 - 1 })
        .with_value_to_string(Arc::new(move |v| names[v.clamp(0, names.len() as i32 - 1) as usize].to_string()))
        .with_string_to_value(Arc::new(move |s| names.iter().position(|n| n.eq_ignore_ascii_case(s.trim())).map(|i| i as i32)))
}

#[derive(Params)]
pub struct Step {
    #[id = "st"]
    pub v: IntParam,
}

#[derive(Params)]
pub struct LaneParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "mute"]
    pub mute: BoolParam,
    #[id = "solo"]
    pub solo: BoolParam,
    #[id = "inv"]
    pub inv: BoolParam,
    #[id = "type"]
    pub kind: IntParam,
    #[id = "style"]
    pub style: IntParam,
    #[id = "sync"]
    pub sync: BoolParam,
    #[id = "note"]
    pub note: IntParam,
    #[id = "ms"]
    pub ms: FloatParam,

    #[id = "send"]
    pub send: FloatParam,
    #[id = "fb"]
    pub fb: FloatParam,
    #[id = "pitch"]
    pub pitch: IntParam,
    #[id = "mod"]
    pub modu: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "diff"]
    pub diff: FloatParam,
    #[id = "lc"]
    pub lc: FloatParam,
    #[id = "hc"]
    pub hc: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "pan"]
    pub pan: FloatParam,
    #[id = "out"]
    pub out: FloatParam,

    // FEEDBACK tab
    #[id = "slope"]
    pub slope: IntParam,
    #[id = "inloop"]
    pub inloop: BoolParam,
    #[id = "freeze"]
    pub freeze: BoolParam,
    #[id = "res"]
    pub res: FloatParam,
    #[id = "tilt"]
    pub tilt: FloatParam,
    #[id = "ldrive"]
    pub ldrive: FloatParam,
    #[id = "xfb"]
    pub xfb: FloatParam,

    // TIME tab
    #[id = "tchange"]
    pub tchange: IntParam,
    #[id = "ratio"]
    pub ratio: FloatParam,
    #[id = "offset"]
    pub offset: FloatParam,
    #[id = "groove"]
    pub groove: FloatParam,
    #[id = "feel"]
    pub feel: FloatParam,
    #[id = "glide"]
    pub glide: FloatParam,

    // MOD tab
    #[id = "mshape"]
    pub mshape: IntParam,
    #[id = "mtgt"]
    pub mtgt: IntParam,
    #[id = "mrate"]
    pub mrate: FloatParam,
    #[id = "mphase"]
    pub mphase: FloatParam,
    #[id = "menv"]
    pub menv: FloatParam,
    #[id = "mdrift"]
    pub mdrift: FloatParam,

    // PITCH tab
    #[id = "pmode"]
    pub pmode: IntParam,
    #[id = "fine"]
    pub fine: FloatParam,
    #[id = "pmix"]
    pub pmix: FloatParam,
    #[id = "pgrain"]
    pub pgrain: FloatParam,
    #[id = "pspread"]
    pub pspread: FloatParam,

    // DIFFUSE tab
    #[id = "dpos"]
    pub dpos: IntParam,
    #[id = "dsize"]
    pub dsize: FloatParam,
    #[id = "dstages"]
    pub dstages: IntParam,
    #[id = "dmotion"]
    pub dmotion: FloatParam,
    #[id = "dspread"]
    pub dspread: FloatParam,

    // DUCK tab
    #[id = "hears"]
    pub hears: IntParam,
    #[id = "damt"]
    pub damt: FloatParam,
    #[id = "dthr"]
    pub dthr: FloatParam,
    #[id = "datt"]
    pub datt: FloatParam,
    #[id = "drel"]
    pub drel: FloatParam,

    // CHARACTER (per family)
    #[id = "bits"]
    pub bits: FloatParam,
    #[id = "srate"]
    pub srate: FloatParam,
    #[id = "bw"]
    pub bw: FloatParam,
    #[id = "dnoise"]
    pub dnoise: FloatParam,
    #[id = "stutter"]
    pub stutter: FloatParam,
    #[id = "wow"]
    pub wow: FloatParam,
    #[id = "flutter"]
    pub flutter: FloatParam,
    #[id = "tsat"]
    pub tsat: FloatParam,
    #[id = "bump"]
    pub bump: FloatParam,
    #[id = "hiss"]
    pub hiss: FloatParam,
    #[id = "clock"]
    pub clock: FloatParam,
    #[id = "comp"]
    pub comp: FloatParam,
    #[id = "dark"]
    pub dark: FloatParam,
    #[id = "age"]
    pub age: FloatParam,
    #[id = "density"]
    pub density: FloatParam,
    #[id = "detune"]
    pub detune: FloatParam,
    #[id = "revwin"]
    pub revwin: FloatParam,
    #[id = "cgrain"]
    pub cgrain: FloatParam,
    #[id = "cspread"]
    pub cspread: FloatParam,

    // RHYTHM steps
    #[nested(array, group = "Rhythm Steps")]
    pub steps: [Step; 16],
}

pub const DEFAULT_STEPS: [i32; 16] = [0, 0, 2, 0, 0, 0, 1, 0, 0, 2, 0, 0, 1, 0, 0, 3];

impl LaneParams {
    pub fn new(b: bool) -> Self {
        let pick = |a: f32, bb: f32| if b { bb } else { a };
        let pi = |a: i32, bb: i32| if b { bb } else { a };
        Self {
            on: BoolParam::new("On", true),
            mute: BoolParam::new("Mute", false),
            solo: BoolParam::new("Solo", false),
            inv: BoolParam::new("Phase Invert", false),
            kind: IntParam::new("Echo Type", pi(5, 15), IntRange::Linear { min: 0, max: 19 })
                .with_value_to_string(Arc::new(|v| TYPES[v.clamp(0, 19) as usize].0.to_string()))
                .with_string_to_value(Arc::new(|s| TYPES.iter().position(|t| t.0.eq_ignore_ascii_case(s.trim())).map(|i| i as i32))),
            style: choice("Style", pi(2, 0), &STYLES),
            sync: BoolParam::new("Sync", true),
            note: choice("Time (Sync)", pi(10, 14), &NOTE_NAMES),
            ms: skew("Time (ms)", pick(375.0, 600.0), 1.0, 4000.0, " ms", 0),
            send: level("Send", 0.0),
            fb: pct("Feedback", pick(0.38, 0.55), 1.1),
            pitch: IntParam::new("Pitch", 0, IntRange::Linear { min: -24, max: 24 }).with_unit(" st"),
            modu: pct("Mod", pick(0.15, 0.3), 1.0),
            drive: lin("Drive", pick(6.0, 0.0), 0.0, 24.0, " dB", 1),
            diff: pct("Diffuse", pick(0.0, 0.6), 1.0),
            lc: hz("Low Cut", pick(150.0, 250.0), 20.0, 2000.0),
            hc: hz("High Cut", pick(6000.0, 9000.0), 1000.0, 20000.0),
            width: pct("Width", pick(1.0, 1.2), 2.0),
            pan: pan_param("Pan"),
            out: level("Output", pick(-6.0, -9.0)),
            slope: choice("Filter Slope", 1, &SLOPES),
            inloop: BoolParam::new("Filter In Loop", true),
            freeze: BoolParam::new("Freeze", false),
            res: pct("Resonance", 0.15, 1.0),
            tilt: lin("Tilt", 0.0, -6.0, 6.0, " dB", 1),
            ldrive: lin("Loop Drive", 3.6, 0.0, 18.0, " dB", 1),
            xfb: pct("L/R Cross", 0.0, 1.0),
            tchange: choice("When Time Changes", pi(0, 1), &TCHANGE),
            ratio: lin("L/R Ratio", 0.75, 0.25, 2.0, "x", 2),
            offset: lin("R Offset", 0.0, -50.0, 50.0, " ms", 1),
            groove: pct("Groove", 0.0, 1.0).with_unit("").with_value_to_string(Arc::new(|v| format!("{:.0}% swing", 50.0 + v * 25.0))),
            feel: lin("Feel", 0.0, -20.0, 20.0, " ms", 1),
            glide: skew("Glide", 600.0, 0.0, 2000.0, " ms", 0),
            mshape: choice("Mod Shape", pi(3, 0), &MOD_SHAPES),
            mtgt: choice("Mod Target", 0, &MOD_TARGETS),
            mrate: hz("Mod Rate", 0.6, 0.05, 10.0),
            mphase: lin("Mod Stereo Phase", 90.0, 0.0, 180.0, "°", 0),
            menv: pct("Mod Env Sens", 0.5, 1.0),
            mdrift: pct("Mod Drift", 0.2, 1.0),
            pmode: choice("Pitch Mode", 0, &PMODES),
            fine: lin("Pitch Fine", 0.0, -100.0, 100.0, " ct", 0),
            pmix: pct("Pitch Mix", 1.0, 1.0),
            pgrain: skew("Pitch Grain", 60.0, 10.0, 200.0, " ms", 0),
            pspread: lin("Pitch L/R Detune", 0.0, 0.0, 30.0, " ct", 0),
            dpos: choice("Diffuse Position", pi(0, 1), &DPOS),
            dsize: skew("Diffuse Size", 90.0, 5.0, 200.0, " ms", 0),
            dstages: IntParam::new("Diffuse Density", 8, IntRange::Linear { min: 2, max: 12 }).with_unit(" stages"),
            dmotion: pct("Diffuse Motion", 0.3, 1.0),
            dspread: pct("Diffuse Spread", 0.7, 1.0),
            hears: choice("Duck Listens To", 0, &HEARS),
            damt: lin("Duck Amount", pick(14.0, 20.0), 0.0, 40.0, " dB", 1),
            dthr: lin("Duck Threshold", -24.0, -60.0, 0.0, " dB", 1),
            datt: lin("Duck Attack", 20.0, 0.0, 100.0, " ms", 0),
            drel: skew("Duck Release", 600.0, 10.0, 2000.0, " ms", 0),
            bits: lin("Bits", 24.0, 4.0, 24.0, "-bit", 0),
            srate: skew("Sample Rate", 48.0, 2.0, 48.0, " kHz", 1),
            bw: hz("Bandwidth", 20000.0, 2000.0, 20000.0),
            dnoise: pct("Digital Noise", 0.0, 1.0),
            stutter: pct("Stutter", 0.5, 1.0),
            wow: pct("Wow", 0.25, 1.0),
            flutter: pct("Flutter", 0.2, 1.0),
            tsat: pct("Tape Saturation", 0.35, 1.0),
            bump: lin("Head Bump", 2.4, 0.0, 6.0, " dB", 1),
            hiss: pct("Hiss", 0.1, 1.0),
            clock: pct("Clock Noise", 0.15, 1.0),
            comp: pct("Compander", 0.4, 1.0),
            dark: pct("Darkness", 0.5, 1.0),
            age: pct("Age", 0.2, 1.0),
            density: pct("Cloud Density", 0.6, 1.0),
            detune: lin("Detune", 10.0, 0.0, 50.0, " ct", 0),
            revwin: skew("Reverse Window", 510.0, 20.0, 1000.0, " ms", 0),
            cgrain: skew("Grain", 86.0, 10.0, 200.0, " ms", 0),
            cspread: pct("Grain Spread", 0.6, 1.0),
            steps: std::array::from_fn(|k| Step { v: choice("Step", DEFAULT_STEPS[k], &STEP_NAMES) }),
        }
    }
}

#[derive(Params)]
pub struct TbParams {
    /// window size (hosts keep track of it; not persisted)
    pub editor_state: Arc<EguiEditorState>,
    #[id = "gmix"]
    pub mix: FloatParam,
    #[id = "bypass"]
    pub bypass: BoolParam,
    /// Window size (1.0 = 100%), saved with the project. 0 = not set: open at the default.
    #[persist = "ui-scale"]
    pub ui_scale: Arc<std::sync::RwLock<f32>>,
    #[id = "safe"]
    pub safe: BoolParam,
    #[id = "os"]
    pub os: IntParam,
    #[id = "route"]
    pub route: IntParam,
    #[id = "dry"]
    pub dry: FloatParam,
    #[id = "dryon"]
    pub dry_on: BoolParam,
    #[id = "cross"]
    pub cross: FloatParam,
    /// ON (default): your sound passes untouched, lanes add on top.
    /// OFF (INSERT): your sound runs through the ORIGINAL strip first.
    #[id = "par"]
    pub parallel: BoolParam,
    #[id = "odrv"]
    pub o_drive: FloatParam,
    #[id = "olc"]
    pub o_lc: FloatParam,
    #[id = "ohc"]
    pub o_hc: FloatParam,
    #[id = "owid"]
    pub o_width: FloatParam,
    #[id = "opan"]
    pub o_pan: FloatParam,
    #[id = "oout"]
    pub o_out: FloatParam,
    #[nested(id_prefix = "a", group = "Lane A")]
    pub a: LaneParams,
    #[nested(id_prefix = "b", group = "Lane B")]
    pub b: LaneParams,
}

impl Default for TbParams {
    fn default() -> Self {
        Self {
            editor_state: EguiEditorState::from_size(nice_plug::editor::dpi::LogicalSize::new((crate::editor::WIDTH) as f64, (crate::editor::HEIGHT) as f64), 1.0),
            mix: awk_ui::mix_param(),
            bypass: awk_ui::bypass_param(),
            ui_scale: Arc::new(std::sync::RwLock::new(0.0)),
            safe: awk_ui::safe_param(),
            os: awk_ui::os_param(),
            route: choice("Routing", 0, &ROUTES),
            dry: level("Dry", 0.0),
            dry_on: BoolParam::new("Dry On", true),
            cross: pct("Cross-Feed", 0.3, 1.0),
            parallel: BoolParam::new("Parallel", true),
            o_drive: lin("Original Drive", 0.0, 0.0, 24.0, " dB", 1),
            o_lc: hz("Original Low Cut", 20.0, 20.0, 2000.0),
            o_hc: hz("Original High Cut", 20000.0, 1000.0, 20000.0),
            o_width: pct("Original Width", 1.0, 2.0),
            o_pan: pan_param("Original Pan"),
            o_out: level("Original Output", 0.0),
            a: LaneParams::new(false),
            b: LaneParams::new(true),
        }
    }
}

impl TbParams {
    pub fn lane(&self, i: usize) -> &LaneParams {
        if i == 0 {
            &self.a
        } else {
            &self.b
        }
    }
}
