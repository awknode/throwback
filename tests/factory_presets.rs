//! Factory presets. `cargo test --test factory_presets` checks every preset only
//! names real params with legal values; with PRESET_OUT=dir it also writes the
//! .awkp files there (full state JSON; anything a preset doesn't mention loads
//! at its default).
use nice_plug::prelude::*;
use serde_json::{json, Map, Value};
use throw_back::TbParams;

type P = &'static [(&'static str, f64)];

// Lane B off in single-lane presets so the picture stays clear.
const B_OFF: (&str, f64) = ("b_on", 0.0);

const PRESETS: &[(&str, P)] = &[
    ("VOCAL - Tape Throw + Space", &[]),
    // The classic vocal throw: automate A SEND (OFF -> 0 dB) on the last word of a line.
    // Dark ping-pong tape quarters that duck under the vocal, blooming into a small
    // diffuse space (A › B), so only the thrown words get space.
    ("VOCAL - The Throw (automate A SEND)", &[("route", 1.0), ("a_type", 5.0), ("a_style", 2.0), ("a_sync", 1.0), ("a_note", 11.0), ("a_fb", 0.38), ("a_drive", 3.0), ("a_lc", 350.0), ("a_hc", 4500.0), ("a_width", 1.0), ("a_damt", 12.0), ("a_dthr", -28.0), ("a_drel", 350.0), ("a_out", -6.0), ("b_type", 15.0), ("b_sync", 0.0), ("b_ms", 60.0), ("b_fb", 0.55), ("b_diff", 0.85), ("b_lc", 400.0), ("b_hc", 6000.0), ("b_damt", 10.0), ("b_out", -12.0)]),
    ("VOCAL - Slapback Double", &[("a_type", 9.0), ("a_style", 0.0), ("a_sync", 0.0), ("a_ms", 110.0), ("a_fb", 0.2), ("a_out", -8.0), ("a_drive", 3.0), B_OFF]),
    ("VOCAL - Ducked Quarter", &[("a_type", 0.0), ("a_style", 0.0), ("a_note", 11.0), ("a_fb", 0.35), ("a_damt", 18.0), ("a_dthr", -30.0), ("a_drive", 0.0), ("b_damt", 20.0)]),
    ("VOCAL - Radio Throw", &[("a_type", 3.0), ("a_note", 11.0), ("a_fb", 0.45), ("a_out", -8.0), B_OFF]),
    ("VOCAL - Shimmer Halo", &[("a_type", 0.0), ("a_style", 0.0), ("a_note", 8.0), ("a_fb", 0.25), ("a_drive", 0.0), ("b_type", 18.0), ("b_pitch", 12.0), ("b_fb", 0.6), ("b_diff", 0.5), ("b_out", -14.0), ("b_note", 14.0)]),
    ("VOCAL - Ghost Haze", &[("a_type", 16.0), ("a_style", 0.0), ("a_note", 13.0), ("a_fb", 0.5), ("a_drive", 0.0), ("a_dpos", 1.0), B_OFF]),
    ("VOCAL - Pitch Up Down Ping", &[("a_type", 0.0), ("a_style", 2.0), ("a_pitch", 7.0), ("a_pmode", 2.0), ("a_fb", 0.45), ("a_drive", 0.0), B_OFF]),
    ("DRUMS - Snare Dub Throw", &[("a_type", 13.0), ("a_style", 2.0), ("a_note", 13.0), ("a_fb", 0.75), ("a_res", 0.6), ("a_dark", 0.4), ("a_damt", 0.0), B_OFF]),
    ("DRUMS - Rhythm Bounce", &[("a_type", 0.0), ("a_style", 4.0), ("a_fb", 0.3), ("a_drive", 0.0), ("a_damt", 0.0), B_OFF]),
    ("DRUMS - Glitch Hats", &[("a_type", 4.0), ("a_style", 2.0), ("a_note", 5.0), ("a_fb", 0.4), ("a_stutter", 0.6), ("a_damt", 0.0), B_OFF]),
    ("DRUMS - Drum Room Echo", &[("a_type", 8.0), ("a_style", 0.0), ("a_note", 8.0), ("a_fb", 0.35), ("a_damt", 6.0), B_OFF]),
    ("DRUMS - Lo-Fi Crush 16ths", &[("a_type", 2.0), ("a_style", 2.0), ("a_note", 5.0), ("a_fb", 0.4), ("a_bits", 8.0), ("a_damt", 0.0), B_OFF]),
    ("MELODY - Dual Dotted", &[("a_type", 0.0), ("a_style", 1.0), ("a_ratio", 0.667), ("a_fb", 0.4), ("a_drive", 0.0)]),
    ("MELODY - Chorus Echo Wide", &[("a_type", 12.0), ("a_style", 0.0), ("a_note", 11.0), ("a_fb", 0.4), ("a_width", 1.6), ("a_drive", 0.0), B_OFF]),
    ("MELODY - BBD Warmth", &[("a_type", 10.0), ("a_fb", 0.5), ("a_drive", 3.0), B_OFF]),
    ("MELODY - Dirty Bucket", &[("a_type", 11.0), ("a_style", 0.0), ("a_note", 11.0), ("a_fb", 0.55), B_OFF]),
    ("MELODY - Filter Echo Sweep", &[("a_type", 14.0), ("a_note", 8.0), ("a_fb", 0.55), ("a_mod", 0.7), ("a_mshape", 0.0), ("a_mrate", 0.25), ("a_mtgt", 2.0), B_OFF]),
    ("MELODY - Multi-Head Space", &[("a_type", 7.0), ("a_style", 0.0), ("a_note", 8.0), ("a_fb", 0.45), B_OFF]),
    ("MELODY - Worn Tape Dream", &[("a_type", 6.0), ("a_style", 0.0), ("a_note", 11.0), ("a_fb", 0.5), ("a_wow", 0.5)]),
    ("MELODY - Rack 12-Bit Ping", &[("a_type", 1.0), ("a_note", 8.0), ("a_fb", 0.45), ("a_drive", 0.0), B_OFF]),
    ("MELODY - Reverse Swells", &[("a_type", 17.0), ("a_style", 0.0), ("a_note", 14.0), ("a_fb", 0.3), ("a_revwin", 500.0), ("a_drive", 0.0), ("b_out", -14.0)]),
    ("MELODY - Cloud Grains", &[("a_type", 19.0), ("a_style", 0.0), ("a_note", 11.0), ("a_fb", 0.5), ("a_drive", 0.0)]),
    ("SPACE - Diffuse Hall", &[("a_on", 0.0), ("b_type", 15.0), ("b_sync", 0.0), ("b_ms", 90.0), ("b_fb", 0.8), ("b_diff", 0.9), ("b_dsize", 120.0), ("b_out", -8.0), ("b_lc", 200.0), ("b_hc", 7000.0)]),
    ("SPACE - Serial Echo Into Verb", &[("route", 1.0), ("b_fb", 0.7), ("b_diff", 0.8), ("b_out", -6.0)]),
    ("SPACE - Cross-Feed Wash", &[("route", 2.0), ("cross", 0.5), ("a_note", 11.0), ("b_note", 16.0)]),
    ("FX - Send Mode 100 Wet", &[("dryon", 0.0), ("a_out", 0.0), ("b_out", -3.0)]),
    ("FX - Runaway Dub (SAFE holds it)", &[("a_type", 13.0), ("a_fb", 1.05), ("a_note", 13.0), ("a_damt", 0.0), B_OFF]),
    ("FX - Studio Clean Quarter", &[("a_type", 0.0), ("a_style", 0.0), ("a_note", 11.0), ("a_fb", 0.3), ("a_drive", 0.0), ("a_lc", 20.0), ("a_hc", 20000.0), ("a_mod", 0.0), ("a_damt", 0.0), B_OFF]),
];

fn build(params: &TbParams, over: P) -> Value {
    let map = params.param_map();
    let mut out = Map::new();
    for (id, v) in over {
        let ptr = map.iter().find(|(i, _, _)| i == id).unwrap_or_else(|| panic!("no param {id}")).1;
        let val = unsafe {
            match ptr {
                ParamPtr::FloatParam(p) => {
                    let p = &*p;
                    let n = p.preview_normalized(*v as f32);
                    assert!((p.preview_plain(n) - *v as f32).abs() < 1e-3 * (1.0 + v.abs() as f32), "{id} out of range: {v}");
                    json!({ "f32": *v as f32 })
                }
                ParamPtr::IntParam(p) => {
                    let p = &*p;
                    let n = p.preview_normalized(*v as i32);
                    assert_eq!(p.preview_plain(n), *v as i32, "{id} out of range: {v}");
                    json!({ "i32": *v as i32 })
                }
                ParamPtr::BoolParam(_) => json!({ "bool": *v != 0.0 }),
                _ => panic!("{id}: unexpected param kind"),
            }
        };
        out.insert(id.to_string(), val);
    }
    json!({ "version": env!("CARGO_PKG_VERSION"), "params": out, "fields": {} })
}

#[test]
fn factory_presets_are_valid() {
    let params = TbParams::default();
    let dir = std::env::var("PRESET_OUT").ok();
    for (name, over) in PRESETS {
        let v = build(&params, over);
        let _: nice_plug::plugin::PluginState = serde_json::from_value(v.clone()).unwrap();
        if let Some(d) = &dir {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(format!("{d}/{name}.awkp"), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        }
    }
    assert!(PRESETS.len() >= 25);
}

#[test]
fn print_ids() {
    let p = TbParams::default();
    let ids: Vec<String> = p.param_map().into_iter().map(|x| x.0).collect();
    println!("IDS {}", ids.join(" "));
}
