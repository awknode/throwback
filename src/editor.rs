//! ThrowBack editor, awk studios "Vector HUD" look. Lane A orange, lane B pink.
//! Layout (1000 x 950), same as the approved mockup:
//! - header: presets, A/B, undo/redo, power, MIX (scales the lanes only)
//! - ROUTING: PARALLEL / SERIAL A›B / CROSS-FEED, the flow drawn live, DRY knob +
//!   DRY ON/OFF, CROSS, TAP
//! - THE ECHOES: every repeat of both lanes, L up / R down, bar.beat + ms axes
//! - lane strips A and B: ON MUTE SOLO Ø, EDIT, SYNC, 11 knobs, OUTPUT, meter
//! - EDIT panel: TYPE TIME FEEDBACK MOD PITCH DIFFUSE DUCK RHYTHM for the chosen lane
//! - IN/OUT meters, footer with OVERSAMPLING (drive only) and SAFE
//! Knobs: drag (Ctrl/Shift fine), scroll, double-click resets.

use crate::awk_ui::*;
use crate::model::{self, Train};
use crate::params::*;
use crate::presets::PresetBar;
use crate::Shared;
use nice_plug::prelude::*;
use nice_plug_egui::create_egui_editor;
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Shape, Stroke, StrokeKind};
use std::sync::Arc;

pub const WIDTH: u32 = 1000;
pub const HEIGHT: u32 = 1062;
pub const LC: [Color32; 2] = [rgb(0xff, 0x8a, 0x3d), rgb(0xff, 0x5f, 0xa2)];
const TXT: Color32 = rgb(0xcd, 0xef, 0xff);
const HI: Color32 = rgb(0xf0, 0xfb, 0xff);
const DIMT: Color32 = Color32::from_rgba_premultiplied(0x6b, 0x96, 0xaa, 0xd9);
const TABS: [&str; 8] = ["TYPE", "TIME", "FEEDBACK", "MOD", "PITCH", "DIFFUSE", "DUCK", "RHYTHM"];

struct St {
    presets: PresetBar,
    skin: Skin,
    edit: usize,
    tab: usize,
    zoom: usize,
    taps: Vec<f64>,
    /// last 4 s of (input dB, duck dB) per lane, 30 per second
    duck_hist: [Vec<(f32, f32)>; 2],
    last_hist: f64,
    lane_hold: [f32; 2],
}

fn set_i(setter: &ParamSetter, p: &IntParam, v: i32) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn set_b(setter: &ParamSetter, p: &BoolParam, v: bool) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn set_f(setter: &ParamSetter, p: &FloatParam, v: f32) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn fmt_db(d: f32) -> String {
    if !d.is_finite() || d < -99.0 {
        "-∞".into()
    } else {
        format!("{d:+.1}")
    }
}
fn lab(p: &egui::Painter, pos: Pos2, s: &str) {
    txt(p, pos, Align2::LEFT_CENTER, s, disp(7.5), DIMT, 1.5);
}

/// Row of choice chips; returns the clicked index.
#[allow(clippy::too_many_arguments)]
fn chips(ui: &mut egui::Ui, skin: &Skin, at: Pos2, w: f32, h: f32, gap: f32, names: &[&str], cur: i32, id: &str) -> Option<i32> {
    let mut hit = None;
    for (i, s) in names.iter().enumerate() {
        let r = Rect::from_min_size(at + vec2(i as f32 * (w + gap), 0.0), vec2(w, h));
        if skin.chip(ui, r, s, cur == i as i32, (id, i)).clicked() {
            hit = Some(i as i32);
        }
    }
    hit
}
fn choice_row(ui: &mut egui::Ui, setter: &ParamSetter, skin: &Skin, at: Pos2, w: f32, names: &[&str], p: &IntParam, id: &str) {
    if let Some(i) = chips(ui, skin, at, w, 22.0, 6.0, names, p.value(), id) {
        set_i(setter, p, i);
    }
}

pub fn create(params: Arc<TbParams>, shared: Arc<Shared>) -> Option<crate::nice_app::AwkEditor> {
    let state = params.editor_state.clone();
    let st = new_state();
    crate::presets::apply_scale(&state, &params.ui_scale);
    create_egui_editor(state, nice_plug_egui::RepaintNotifier::new(), nice_plug_egui::EguiNiceSettings::default(), crate::nice_app::AwkApp::new(install_fonts, {
        let mut __st = st;
        Box::new(move |ctx: &mut egui::Ui, setter: &ParamSetter| {
            let st = &mut __st;
            frame(ctx, setter, st, params.as_ref(), shared.as_ref())
        })
    }))
}

fn new_state() -> St {
    St {
        presets: PresetBar::new("ThrowBack").with_dice(&["gmix", "safe", "os", "out", "oout", "dry", "dryon", "on", "mute", "solo", "freeze", "inloop"]),
        skin: Skin::new(Look::Hud, LC[0], "THROWBACK", env!("CARGO_PKG_VERSION"), "2 PARALLEL LANES // DRY NEVER PROCESSED // 20 ECHO TYPES // HOST SYNC // SAFE 0 dB CEILING // PDC 0 SMP"),
        edit: 0,
        tab: 0,
        zoom: 1,
        taps: Vec::new(),
        duck_hist: [Vec::new(), Vec::new()],
        last_hist: 0.0,
        lane_hold: [-100.0; 2],
    }
}

fn frame(root: &mut egui::Ui, setter: &ParamSetter, st: &mut St, p: &TbParams, shared: &Shared) {
    let ctx_owned = root.ctx().clone();
    let ctx = &ctx_owned;
    ctx.request_repaint_after(std::time::Duration::from_millis(33));
    st.skin.begin(ctx);
    let bpm = Shared::get(&shared.bpm).clamp(20.0, 999.0);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(rgb(0x04, 0x07, 0x0c))).show(root, |ui| {
        let whole = ui.max_rect();
        st.skin.background(ui.painter(), whole);
        let _full = st.presets.strip(ui, setter, p, whole, &st.skin, &p.mix, &p.bypass, &p.ui_scale);
        let o = whole.min.to_vec2();
        let at = |x: f32, y: f32| pos2(x, y) + o;
        let rc = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(at(x, y), vec2(w, h));
        let painter = ui.painter().clone();
        let trains = model::all(p, bpm);
        let m = &shared.meters;

        // history for the DUCK screen
        let now = st.skin.time;
        if now - st.last_hist > 1.0 / 30.0 {
            st.last_hist = now;
            for k in 0..2 {
                let h = &mut st.duck_hist[k];
                h.push((m.db(0).max(m.db(1)), Shared::get(&shared.duck_db[k])));
                if h.len() > 120 {
                    h.remove(0);
                }
            }
        }

        routing(ui, setter, &mut st.skin, p, &trains, rc(16.0, 56.0, 300.0, 296.0), bpm, &mut st.taps, st.edit);
        echoes(ui, &mut st.skin, p, &trains, rc(328.0, 56.0, 572.0, 296.0), bpm, &mut st.zoom);
        orig_strip(ui, setter, &mut st.skin, p, rc(16.0, 364.0, 884.0, 64.0));
        for li in 0..2 {
            let y = if li == 0 { 440.0 } else { 568.0 };
            let db = Shared::get(&shared.lane_db[li]);
            st.lane_hold[li] = if db > st.lane_hold[li] { db } else { st.lane_hold[li] - 0.4 };
            lane_strip(ui, setter, &mut st.skin, p, li, &trains[li], rc(16.0, y, 884.0, 120.0), db, st.lane_hold[li], &mut st.edit);
        }
        detail(ui, setter, p, &trains, rc(16.0, 700.0, 884.0, 318.0), st, bpm);

        // meters
        let skin = &mut st.skin;
        skin.panel(&painter, rc(912.0, 56.0, 72.0, 962.0), "", "");
        skin.meter(&painter, at(920.0, 0.0).x, at(0.0, 92.0).y, 882.0, "IN", m.db(0), m.db(1), Side::Right, "in");
        skin.meter(&painter, at(962.0, 0.0).x, at(0.0, 92.0).y, 882.0, "OUT", m.db(2), m.db(3), Side::None, "out");

        // footer
        let foot = Rect::from_min_max(pos2(whole.left(), whole.bottom() - 30.0), whole.right_bottom());
        skin.footer(&painter, foot, foot.left() + 470.0);
        skin.os_selector(ui, setter, pos2(foot.left() + 16.0, foot.center().y - 8.5), &p.os, 1.4);
        skin.safe_chip(ui, setter, Rect::from_center_size(pos2(foot.left() + 404.0, foot.center().y), vec2(112.0, 18.0)), &p.safe, m.db(4));
    });
}

/// Recessed well a lit key sits in (key and well share one centre).
fn well(pa: &egui::Painter, r: Rect) {
    pa.rect_filled(r, 5.0, rgb(0x02, 0x04, 0x07));
    pa.rect_stroke(r, 5.0, Stroke::new(1.0_f32, rgb(0x14, 0x24, 0x2e)), StrokeKind::Outside);
}

/// Backlit key: smoked glass when off, glowing `col` when lit.
fn lit_key(ui: &mut egui::Ui, r: Rect, s: &str, on: bool, col: Color32, id: impl std::hash::Hash + std::fmt::Debug) -> egui::Response {
    let resp = ui.interact(r, ui.scope_id().with(("tb-litkey", id)), Sense::click());
    let pa = ui.painter();
    let k = r.translate(vec2(0.0, if on { 1.0 } else if resp.is_pointer_button_down_on() { 0.5 } else { -0.5 }));
    if on {
        for i in 0..5 {
            pa.rect_filled(k.expand(1.5 + i as f32 * 2.0), 4.0 + i as f32 * 2.0, al(col, 0.05));
        }
        pa.rect_filled(k, 3.0, mix_col(col, Color32::BLACK, 0.3));
        for i in 0..8 {
            let t = i as f32 / 8.0;
            pa.rect_filled(k.shrink2(vec2(k.width() * 0.44 * t, k.height() * 0.38 * t)), 3.0 + 6.0 * t, al(mix_col(col, Color32::WHITE, 0.15 + 0.6 * t), 0.4));
        }
        vgrad(pa, Rect::from_min_size(k.left_top() + vec2(3.0, 2.0), vec2(k.width() - 6.0, k.height() * 0.4)), Color32::from_white_alpha(60), Color32::TRANSPARENT);
    } else {
        vgrad(pa, k, rgb(0x16, 0x22, 0x2b), rgb(0x08, 0x0e, 0x13));
        vgrad(pa, Rect::from_min_size(k.left_top() + vec2(3.0, 2.0), vec2(k.width() - 6.0, k.height() * 0.4)), Color32::from_white_alpha(if resp.hovered() { 22 } else { 10 }), Color32::TRANSPARENT);
    }
    pa.rect_stroke(k, 3.0, Stroke::new(1.0_f32, rgb(0x01, 0x03, 0x05)), StrokeKind::Outside);
    let tc = if on { mix_col(col, Color32::BLACK, 0.85) } else if resp.hovered() { al(TXT, 0.7) } else { al(TXT, 0.4) };
    txt(pa, k.center(), Align2::CENTER_CENTER, s, mono(10.5), tc, 1.2);
    resp
}

// ---------------------------------------------------------------- ORIGINAL

/// Your own sound's strip. Live only when PARALLEL is off (INSERT).
fn orig_strip(ui: &mut egui::Ui, setter: &ParamSetter, skin: &mut Skin, p: &TbParams, r: Rect) {
    let painter = ui.painter().clone();
    skin.panel(&painter, r, "", "");
    let par = p.parallel.value();
    let o = r.min.to_vec2() - vec2(16.0, 0.0);
    let at = |x: f32, y: f32| pos2(x, y) + o;
    if !par {
        cham_fill(&painter, r, 10.0, 10.0, Color32::TRANSPARENT, Stroke::new(1.2_f32, al(HI, 0.8)));
    }
    painter.rect_filled(Rect::from_min_size(at(17.0, 14.0), vec2(3.0, 36.0)), 0.0, if par { al(HI, 0.3) } else { HI });
    txt(&painter, at(32.0, 24.0), Align2::LEFT_CENTER, "ORIGINAL · YOUR SOUND", mono(12.0), if par { al(HI, 0.5) } else { HI }, 1.0);
    let sub = if par { "PARALLEL ON · YOUR SOUND IS UNTOUCHED" } else { "INSERT · THESE SHAPE YOUR SOUND (NO REPEATS)" };
    txt(&painter, at(32.0, 42.0), Align2::LEFT_CENTER, sub, mono(9.0), DIMT, 1.0);
    let saved = skin.acc;
    skin.acc = HI;
    let ky = 30.0;
    let kx = |i: usize| 446.0 + i as f32 * 66.0;
    skin.knob(ui, setter, at(kx(0), ky), 14.0, "DRIVE", &p.o_drive, false, false, par);
    skin.knob(ui, setter, at(kx(1), ky), 14.0, "LOW CUT", &p.o_lc, false, false, par);
    skin.knob(ui, setter, at(kx(2), ky), 14.0, "HIGH CUT", &p.o_hc, false, false, par);
    skin.knob(ui, setter, at(kx(3), ky), 14.0, "WIDTH", &p.o_width, false, false, par);
    skin.knob(ui, setter, at(kx(4), ky), 14.0, "PAN", &p.o_pan, false, true, par);
    skin.knob(ui, setter, at(kx(5), ky), 14.0, "OUTPUT", &p.o_out, false, false, par);
    skin.acc = saved;
}

// ------------------------------------------------------------------ ROUTING

#[allow(clippy::too_many_arguments)]
fn routing(ui: &mut egui::Ui, setter: &ParamSetter, skin: &mut Skin, p: &TbParams, tr: &[Train; 2], r: Rect, bpm: f32, taps: &mut Vec<f64>, edit: usize) {
    let painter = ui.painter().clone();
    skin.panel(&painter, r, "ROUTING", "SEC.01");
    let o = r.min.to_vec2() - vec2(16.0, 56.0);
    let at = |x: f32, y: f32| pos2(x, y) + o;
    let route = p.route.value();
    // PARALLEL master key (white = your sound untouched)
    let par = p.parallel.value();
    let pk = Rect::from_min_size(at(28.0, 83.0), vec2(150.0, 26.0));
    well(&painter, pk.expand(5.0));
    if lit_key(ui, pk, if par { "PARALLEL · ON" } else { "PARALLEL · OFF" }, par, HI, "par")
        .on_hover_text("ON: your sound passes untouched, echoes add on top.\nOFF (INSERT): your sound runs through the ORIGINAL strip.")
        .clicked()
    {
        set_b(setter, &p.parallel, !par);
    }
    let (h1, h2) = if par { ("YOUR SOUND UNTOUCHED", "ECHOES ADD ON TOP") } else { ("INSERT: YOUR SOUND", "RUNS THROUGH ORIGINAL") };
    txt(&painter, at(194.0, 90.0), Align2::LEFT_CENTER, h1, mono(8.5), TXT, 0.3);
    txt(&painter, at(194.0, 102.0), Align2::LEFT_CENTER, h2, mono(8.5), DIMT, 0.3);
    lab(&painter, at(28.0, 122.0), "LANES A + B");
    if let Some(i) = chips(ui, skin, at(28.0, 129.0), 88.0, 20.0, 6.0, &ROUTES, route, "route") {
        set_i(setter, &p.route, i);
    }
    let t = skin.time as f32;
    let (inp, outp) = (at(44.0, 194.0), at(288.0, 194.0));
    let boxes = if route == 1 { [at(120.0, 194.0), at(200.0, 194.0)] } else { [at(166.0, 176.0), at(166.0, 214.0)] };
    let dry_on = p.dry_on.value();
    let path = |pts: &[Pos2], col: Color32, dash: bool| {
        for w in pts.windows(2) {
            if dash {
                dashed(&painter, w[0], w[1], col, 1.2, 3.0, 3.0);
            } else {
                line(&painter, w[0], w[1], col, 1.2);
            }
        }
    };
    let dot = |pts: &[Pos2], ph: f32, col: Color32| {
        let lens: Vec<f32> = pts.windows(2).map(|w| (w[1] - w[0]).length()).collect();
        let total: f32 = lens.iter().sum();
        let mut s = ((t * 0.35 + ph) % 1.0) * total;
        for (i, l) in lens.iter().enumerate() {
            if s <= *l {
                let q = pts[i] + (pts[i + 1] - pts[i]) * (s / l.max(1e-3));
                painter.circle_filled(q, 4.5, al(col, 0.25));
                painter.circle_filled(q, 2.5, col);
                return;
            }
            s -= l;
        }
    };
    let dp = [inp, pos2(inp.x, at(0.0, 157.0).y), pos2(outp.x, at(0.0, 157.0).y), outp];
    let bypassed = p.bypass.value();
    path(&dp, if dry_on { rgb(0xe6, 0xf0, 0xf5) } else { rgb(0xff, 0x4a, 0x3d) }, !dry_on);
    if dry_on {
        dot(&dp, 0.0, rgb(0xe6, 0xf8, 0xff));
    }
    let dry_db = p.dry.value();
    let dry_tag = if !dry_on { "DRY OFF" } else if par { "ORIGINAL · CLEAN" } else { "ORIGINAL · INSERT" };
    let dm = Rect::from_center_size(at(166.0, 157.0), vec2(txt_w(&painter, dry_tag, mono(8.5), 0.5) + 12.0, 13.0));
    painter.rect_filled(dm, 2.0, Color32::from_rgba_unmultiplied(3, 10, 18, 250));
    txt(&painter, dm.center(), Align2::CENTER_CENTER, dry_tag, mono(8.5), if dry_on { rgb(0xe6, 0xf8, 0xff) } else { rgb(0xff, 0x6a, 0x5a) }, 0.5);
    let lanes: [Vec<Vec<Pos2>>; 2] = if route == 1 {
        [
            vec![vec![inp, boxes[0]], vec![boxes[0], pos2(boxes[0].x, at(0.0, 226.0).y), pos2(outp.x, at(0.0, 226.0).y), outp]],
            vec![vec![boxes[0], boxes[1]], vec![boxes[1], outp]],
        ]
    } else {
        let lane = |b: Pos2| vec![vec![inp, at(80.0, 194.0), pos2(at(80.0, 0.0).x, b.y), b], vec![b, pos2(at(252.0, 0.0).x, b.y), at(252.0, 194.0), outp]];
        [lane(boxes[0]), lane(boxes[1])]
    };
    for li in 0..2 {
        let heard = crate::audible(p, li).1 && !bypassed;
        let col = if heard { LC[li] } else { al(LC[li], 0.3) };
        for (k, pp) in lanes[li].iter().enumerate() {
            path(pp, col, false);
            if heard {
                dot(pp, li as f32 * 0.37 + k as f32 * 0.5, LC[li]);
            }
        }
    }
    if route == 2 {
        let (a, b) = (boxes[0], boxes[1]);
        dashed(&painter, a + vec2(10.0, 13.0), b + vec2(10.0, -13.0), rgb(0xe6, 0xf8, 0xff), 1.0, 2.0, 3.0);
        dashed(&painter, b + vec2(-10.0, -13.0), a + vec2(-10.0, 13.0), rgb(0xe6, 0xf8, 0xff), 1.0, 2.0, 3.0);
        txt(&painter, a + vec2(38.0, 33.0), Align2::LEFT_CENTER, format!("×{:.0}%", p.cross.value() * 100.0), mono(9.0), rgb(0xe6, 0xf8, 0xff), 0.0);
    }
    for li in 0..2 {
        let b = boxes[li];
        let br = Rect::from_center_size(b, vec2(68.0, 26.0));
        cham_fill(&painter, br, 6.0, 6.0, Color32::from_rgba_unmultiplied(3, 10, 18, 242), Stroke::new(1.2_f32, if edit == li { HI } else { LC[li] }));
        let name = TYPES[tr[li].b.kind as usize].0.split(' ').next().unwrap_or("");
        txt(&painter, b, Align2::CENTER_CENTER, format!("{} · {name}", if li == 0 { "A" } else { "B" }), mono(9.0), Color32::WHITE, 0.5);
    }
    for (q, s) in [(inp, "IN"), (outp, "OUT")] {
        painter.circle_filled(q, 11.0, Color32::from_rgba_unmultiplied(3, 10, 18, 242));
        painter.circle_stroke(q, 11.0, Stroke::new(1.2_f32, CY));
        txt(&painter, q, Align2::CENTER_CENTER, s, mono(8.0), rgb(0xe6, 0xf8, 0xff), 0.0);
    }
    // SIGNAL NOW: one row per path, lamp lit when it is running
    lab(&painter, at(28.0, 238.0), "SIGNAL NOW");
    let dry_state = if !dry_on { "OFF · SEND MODE".to_string() } else if dry_db <= -59.9 { "OFF".to_string() } else if par { format!("CLEAN · {dry_db:+.1} dB") } else { format!("INSERT · {dry_db:+.1} dB") };
    let mut rows: Vec<(String, String, Color32, bool)> = vec![("ORIGINAL".into(), dry_state, HI, dry_on && dry_db > -59.9)];
    for li in 0..2 {
        let l = p.lane(li);
        let (on, heard) = crate::audible(p, li);
        let t = &tr[li];
        let cnt = t.echoes.iter().filter(|e| e.ch == 0 || t.b.style == S_PING).count();
        let time = if l.sync.value() { NOTE_NAMES[l.note.value() as usize].to_string() } else { format!("{:.0} ms", t.t * 1000.0) };
        let st_ = if !on { "OFF".to_string() } else if !heard { "MUTED".to_string() } else { format!("ON · {time} · {cnt} REPEATS") };
        let name = format!("{} · {}", if li == 0 { "A" } else { "B" }, TYPES[t.b.kind as usize].0.split(' ').next().unwrap_or(""));
        rows.push((name, st_, LC[li], heard && !bypassed));
    }
    for (k, (name, st_, col, lit)) in rows.iter().enumerate() {
        let y = at(0.0, 251.0 + k as f32 * 13.0).y;
        let c = pos2(at(32.0, 0.0).x, y);
        if *lit {
            painter.circle_filled(c, 6.0, al(*col, 0.18));
            painter.circle_filled(c, 3.6, *col);
        } else {
            painter.circle_filled(c, 3.6, Color32::from_rgba_unmultiplied(40, 60, 72, 255));
        }
        let tc = if *lit { *col } else { DIMT };
        txt(&painter, pos2(c.x + 10.0, y), Align2::LEFT_CENTER, name, mono(9.0), tc, 0.5);
        txt(&painter, pos2(at(304.0, 0.0).x, y), Align2::RIGHT_CENTER, st_, mono(8.5), tc, 0.3);
    }
    skin.knob(ui, setter, at(46.0, 300.0), 12.0, "DRY", &p.dry, false, false, !dry_on);
    skin.knob(ui, setter, at(100.0, 300.0), 12.0, "CROSS", &p.cross, false, false, route != 2);
    if skin.chip(ui, Rect::from_min_size(at(146.0, 292.0), vec2(72.0, 20.0)), if dry_on { "DRY: ON" } else { "DRY: OFF" }, dry_on, "dryon").on_hover_text("Off = 100% wet, for a send/return channel").clicked() {
        set_b(setter, &p.dry_on, !dry_on);
    }
    if skin.chip(ui, Rect::from_min_size(at(226.0, 292.0), vec2(78.0, 20.0)), "TAP", false, "tap").on_hover_text("Tap the time for the lane you're editing (sets it to FREE ms)").clicked() {
        let now = skin.time;
        taps.retain(|&x| now - x < 2.5);
        taps.push(now);
        if taps.len() >= 2 {
            let n = taps.len() - 1;
            let avg = (taps[n] - taps[0]) / n as f64;
            let l = p.lane(edit);
            set_b(setter, &l.sync, false);
            set_f(setter, &l.ms, (avg * 1000.0).clamp(1.0, 4000.0) as f32);
        }
    }
    txt(&painter, at(265.0, 326.0), Align2::CENTER_CENTER, format!("HOST · {bpm:.0} BPM"), mono(9.0), DIMT, 0.5);
}

// --------------------------------------------------------------- THE ECHOES

fn echoes(ui: &mut egui::Ui, skin: &mut Skin, p: &TbParams, tr: &[Train; 2], r: Rect, bpm: f32, zoom: &mut usize) {
    let outer = ui.painter().clone();
    let (c, pal) = skin.screen(&outer, r);
    let (x, y, w, h) = (r.left(), r.top(), r.width(), r.height());
    let beat = 60.0 / bpm;
    let bars = [1.0f32, 2.0, 4.0][*zoom];
    let span = bars * 4.0 * beat;
    let (x0, x1) = (x + 50.0, x + w - 14.0);
    let mid = y + h / 2.0 + 12.0;
    let hh = (h - 84.0) / 2.0;
    let xm = |t: f32| x0 + t / span * (x1 - x0);
    let ym = |d: f32, ch: usize| {
        let rr = (d.clamp(-60.0, 0.0) + 60.0) / 60.0 * hh;
        if ch == 1 {
            mid + rr
        } else {
            mid - rr
        }
    };
    txt(&c, pos2(x + 22.0, y + 17.0), Align2::LEFT_CENTER, "THE ECHOES", disp(8.0), TXT, 2.0);
    txt(&c, pos2(x + 128.0, y + 17.0), Align2::LEFT_CENTER, "L ▲  R ▼", mono(9.0), pal.axis, 1.0);
    for (i, s) in ["1 BAR", "2 BARS", "4 BARS"].iter().enumerate() {
        let br = Rect::from_min_size(pos2(x1 - 180.0 + i as f32 * 60.0, y + 8.0), vec2(56.0, 17.0));
        if skin.chip(ui, br, s, *zoom == i, ("zoom", i)).clicked() {
            *zoom = i;
        }
    }
    for d in [0.0f32, -12.0, -24.0, -36.0, -48.0] {
        for ch in 0..2 {
            let yy = ym(d, ch).round() + 0.5;
            line(&c, pos2(x0, yy), pos2(x1, yy), pal.grid, 1.0);
            if (d as i32) % 24 == 0 {
                txt(&c, pos2(x0 - 6.0, yy), Align2::RIGHT_CENTER, format!("{d:.0}"), mono(8.0), pal.axis, 0.0);
            }
        }
    }
    line(&c, pos2(x0, mid + 0.5), pos2(x1, mid + 0.5), al(CY, 0.3), 1.0);
    txt(&c, pos2(x0 - 6.0, mid), Align2::RIGHT_CENTER, "-∞", mono(8.0), pal.axis, 0.0);
    txt(&c, pos2(x + 14.0, mid - hh / 2.0), Align2::CENTER_CENTER, "L", disp(9.0), TXT, 0.0);
    txt(&c, pos2(x + 14.0, mid + hh / 2.0), Align2::CENTER_CENTER, "R", disp(9.0), TXT, 0.0);
    let nb = (bars * 4.0) as i32;
    for bt in 0..=nb {
        let xx = xm(bt as f32 * beat).round() + 0.5;
        line(&c, pos2(xx, mid - hh), pos2(xx, mid + hh), if bt % 4 == 0 { al(CY, 0.22) } else { pal.grid }, 1.0);
        if nb <= 8 || bt % 2 == 0 {
            txt(&c, pos2(xx, mid - hh - 6.0), Align2::CENTER_BOTTOM, format!("{}.{}", bt / 4 + 1, bt % 4 + 1), mono(8.0), if bt % 4 == 0 { TXT } else { pal.axis }, 0.0);
            txt(&c, pos2(xx, mid + hh + 4.0), Align2::CENTER_TOP, format!("{:.0}", bt as f32 * beat * 1000.0), mono(8.0), pal.axis, 0.0);
        }
        if bars == 1.0 && bt < nb {
            for s in 1..4 {
                let sx = xm((bt as f32 + s as f32 / 4.0) * beat);
                line(&c, pos2(sx, mid - 3.0), pos2(sx, mid + 3.0), pal.axis, 1.0);
            }
        }
    }
    txt(&c, pos2(x0 - 6.0, mid - hh - 6.0), Align2::RIGHT_BOTTOM, "BAR.BEAT", mono(7.0), pal.axis, 0.0);
    txt(&c, pos2(x0 - 6.0, mid + hh + 4.0), Align2::RIGHT_TOP, "ms", mono(8.0), pal.axis, 0.0);
    let dry = p.dry.value();
    if p.dry_on.value() && dry > -59.9 {
        for ch in 0..2 {
            line(&c, pos2(xm(0.0) + 1.0, mid), pos2(xm(0.0) + 1.0, ym(dry, ch)), rgb(0xe6, 0xf0, 0xf5), 2.0);
        }
        txt(&c, pos2(xm(0.0) + 5.0, ym(dry, 0) + 8.0), Align2::LEFT_CENTER, "DRY", mono(8.0), rgb(0xe6, 0xf0, 0xf5), 0.0);
    }
    let mix_db = 20.0 * p.mix.value().max(1e-4).log10();
    let n = (x1 - x0).ceil() as usize;
    for li in [1usize, 0] {
        let heard = crate::audible(p, li).1;
        let a = if heard { 1.0 } else { 0.25 };
        let col = al(LC[li], a);
        let mut env = [vec![0.0f32; n], vec![0.0f32; n]];
        for e in tr[li].echoes.iter() {
            if e.t > span {
                continue;
            }
            let db = e.db + mix_db;
            if db < -60.0 {
                continue;
            }
            let px = xm(e.t);
            let yy = ym(db, e.ch);
            if e.w > 0.002 {
                let amp = (0.006 / e.w).min(1.0);
                let p0 = ((xm(e.t - 3.0 * e.w) - x0).floor().max(0.0)) as usize;
                let p1 = ((xm(e.t + 3.0 * e.w) - x0).ceil() as usize).min(n - 1);
                for q in p0..=p1 {
                    let tt = q as f32 / (x1 - x0) * span;
                    let g = (-((tt - e.t) / e.w).powi(2) / 2.0).exp();
                    env[e.ch][q] += 10f32.powf(db / 10.0) * amp * g;
                }
            }
            let fa = 1.0 - (e.w / 0.03).min(1.0);
            if fa <= 0.02 {
                continue;
            }
            let ec = al(LC[li], a * fa);
            if e.rev {
                let x2 = xm((e.t - tr[li].b.revwin / 48000.0).max(0.0));
                c.add(Shape::convex_polygon(vec![pos2(x2, mid), pos2(px, yy), pos2(px, mid)], al(LC[li], 0.3 * a * fa), Stroke::NONE));
                line(&c, pos2(x2, mid), pos2(px, yy), ec, 1.2);
            } else {
                line(&c, pos2(px, mid), pos2(px, yy), ec, 2.0);
                c.circle_filled(pos2(px, yy), 2.5, ec);
            }
            if e.st.abs() > 0.05 && e.ch == 0 && e.n <= 6 {
                txt(&c, pos2(px, yy - 7.0), Align2::CENTER_BOTTOM, format!("{:+.0}", e.st), mono(8.0), ec, 0.0);
            }
        }
        for ch in 0..2 {
            if !env[ch].iter().any(|v| *v > 1e-6) {
                continue;
            }
            let pts: Vec<Pos2> = (0..n).map(|q| pos2(x0 + q as f32, ym(if env[ch][q] > 1e-7 { 10.0 * env[ch][q].log10() } else { -90.0 }, ch))).collect();
            fill_under(&c, &pts, mid, al(LC[li], 0.2 * a));
            polyline(&c, pts, col, 1.2, false);
        }
    }
    for li in 0..2 {
        let t = &tr[li];
        let l = p.lane(li);
        let cnt = t.echoes.iter().filter(|e| e.ch == 0 || t.b.style == S_PING).count();
        let rt = t.rt60();
        let tail = if t.b.freeze {
            "FROZEN".to_string()
        } else if rt.is_finite() {
            format!("RT60 {rt:.1} s")
        } else {
            "RUNAWAY (LIMITED)".to_string()
        };
        let time = if l.sync.value() { format!("{} = {:.0} ms", NOTE_NAMES[l.note.value() as usize], t.t * 1000.0) } else { format!("{:.0} ms", t.t * 1000.0) };
        let s = format!("{} {} · {time} · {cnt} repeats · {tail}", if li == 0 { "A" } else { "B" }, TYPES[t.b.kind as usize].0);
        let heard = crate::audible(p, li).1;
        txt(&c, pos2(x + 20.0 + li as f32 * (w / 2.0), y + h - 8.0), Align2::LEFT_CENTER, s, mono(8.5), if heard { LC[li] } else { al(LC[li], 0.4) }, 0.3);
    }
    if p.bypass.value() {
        c.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(2, 6, 10, 153));
        txt(&c, pos2(x + w / 2.0, mid), Align2::CENTER_CENTER, "BYPASSED · DRY ONLY", disp(13.0), Color32::WHITE, 3.0);
    }
    skin.screen_end(&outer, r);
}

// --------------------------------------------------------------- lane strip

#[allow(clippy::too_many_arguments)]
fn lane_strip(ui: &mut egui::Ui, setter: &ParamSetter, skin: &mut Skin, p: &TbParams, li: usize, t: &Train, r: Rect, db: f32, hold: f32, edit: &mut usize) {
    let painter = ui.painter().clone();
    let l = p.lane(li);
    let col = LC[li];
    skin.panel(&painter, r, "", "");
    let o = r.min.to_vec2() - vec2(16.0, 0.0);
    let at = |x: f32, y: f32| pos2(x, y) + o;
    if *edit == li {
        cham_fill(&painter, r, 10.0, 10.0, Color32::TRANSPARENT, Stroke::new(1.2_f32, al(col, 0.85)));
    }
    painter.rect_filled(Rect::from_min_size(at(17.0, 30.0), vec2(3.0, 64.0)), 0.0, col);
    txt(&painter, at(42.0, 36.0), Align2::CENTER_CENTER, if li == 0 { "A" } else { "B" }, disp(26.0), col, 0.0);
    txt(&painter, at(66.0, 26.0), Align2::LEFT_CENTER, TYPES[t.b.kind as usize].0, mono(12.0), HI, 1.0);
    txt(&painter, at(66.0, 42.0), Align2::LEFT_CENTER, format!("LANE {} · {}", if li == 0 { "A" } else { "B" }, STYLES[t.b.style.clamp(0, 4) as usize]), mono(9.0), DIMT, 1.0);
    let saved = skin.acc;
    skin.acc = col;
    let lon = l.on.value();
    let kr = Rect::from_min_size(at(28.0, 55.0), vec2(40.0, 22.0));
    well(&painter, kr.expand(3.0));
    if lit_key(ui, kr, if lon { "ON" } else { "OFF" }, lon, col, (li, "lane-on")).on_hover_text("Lane on / off").clicked() {
        set_b(setter, &l.on, !lon);
    }
    for (s, x, w, prm) in [("MUTE", 76.0, 40.0, &l.mute), ("SOLO", 120.0, 40.0, &l.solo), ("Ø", 164.0, 24.0, &l.inv)] {
        if skin.chip(ui, Rect::from_min_size(at(x, 56.0), vec2(w, 20.0)), s, prm.value(), (li, s)).clicked() {
            set_b(setter, prm, !prm.value());
        }
    }
    if skin.chip(ui, Rect::from_min_size(at(28.0, 84.0), vec2(54.0, 20.0)), if *edit == li { "EDITING" } else { "EDIT ›" }, *edit == li, (li, "edit")).clicked() {
        *edit = li;
    }
    let sync = l.sync.value();
    if skin.chip(ui, Rect::from_min_size(at(86.0, 84.0), vec2(50.0, 20.0)), if sync { "SYNC" } else { "FREE ms" }, sync, (li, "sync")).clicked() {
        set_b(setter, &l.sync, !sync);
    }
    // FREEZE: holds what's in the lane forever, nothing new gets in
    let fr = l.freeze.value();
    if skin.chip(ui, Rect::from_min_size(at(140.0, 84.0), vec2(52.0, 20.0)), if fr { "FROZEN" } else { "FREEZE" }, fr, (li, "freeze")).on_hover_text("Hold the repeats forever (nothing new gets in). Click again to let go.").clicked() {
        set_b(setter, &l.freeze, !fr);
    }
    let off = !l.on.value();
    let ky = 50.0;
    let kx = |i: usize| 222.0 + i as f32 * 56.0;
    skin.knob(ui, setter, at(kx(0), ky), 16.0, "SEND", &l.send, false, false, off);
    if sync {
        skin.knob(ui, setter, at(kx(1), ky), 16.0, "TIME", &l.note, false, false, off);
    } else {
        skin.knob(ui, setter, at(kx(1), ky), 16.0, "TIME", &l.ms, false, false, off);
    }
    skin.knob(ui, setter, at(kx(2), ky), 16.0, "FEEDBACK", &l.fb, false, false, off);
    skin.knob(ui, setter, at(kx(3), ky), 16.0, "PITCH", &l.pitch, false, true, off);
    skin.knob(ui, setter, at(kx(4), ky), 16.0, "MOD", &l.modu, false, false, off);
    skin.knob(ui, setter, at(kx(5), ky), 16.0, "DRIVE", &l.drive, false, false, off);
    skin.knob(ui, setter, at(kx(6), ky), 16.0, "DIFFUSE", &l.diff, false, false, off);
    skin.knob(ui, setter, at(kx(7), ky), 16.0, "LOW CUT", &l.lc, false, false, off);
    skin.knob(ui, setter, at(kx(8), ky), 16.0, "HIGH CUT", &l.hc, false, false, off);
    skin.knob(ui, setter, at(kx(9), ky), 16.0, "WIDTH", &l.width, false, false, off);
    skin.knob(ui, setter, at(kx(10), ky), 16.0, "PAN", &l.pan, false, true, off);
    skin.knob(ui, setter, at(838.0, 52.0), 22.0, "OUTPUT", &l.out, true, false, off);
    skin.acc = saved;
    // lane meter
    let (mx, my, mh) = (at(876.0, 0.0).x, at(0.0, 26.0).y, 82.0);
    let map = |d: f32| my + mh - (d.clamp(-60.0, 6.0) + 60.0) / 66.0 * mh;
    painter.rect_filled(Rect::from_min_max(pos2(mx, my), pos2(mx + 7.0, my + mh)), 1.0, Color32::from_rgba_unmultiplied(3, 10, 18, 230));
    if db > -60.0 {
        painter.rect_filled(Rect::from_min_max(pos2(mx, map(db)), pos2(mx + 7.0, my + mh)), 1.0, if db > -0.1 { rgb(0xff, 0x4a, 0x3d) } else { col });
    }
    for d in [0.0f32, -24.0, -48.0] {
        let yy = map(d).round() + 0.5;
        line(&painter, pos2(mx + 9.0, yy), pos2(mx + 12.0, yy), DIMT, 1.0);
    }
    txt(&painter, pos2(mx + 8.0, my - 9.0), Align2::CENTER_CENTER, fmt_db(hold.max(-100.0)), mono(9.0), if hold > -0.05 { rgb(0xff, 0x6a, 0x5a) } else { rgb(0xdf, 0xf6, 0xff) }, 0.0);
}

// ------------------------------------------------------------------- EDIT

#[allow(clippy::too_many_arguments)]
fn detail(ui: &mut egui::Ui, setter: &ParamSetter, p: &TbParams, tr: &[Train; 2], r: Rect, st: &mut St, bpm: f32) {
    let St { skin, edit, tab, duck_hist, .. } = st;
    let painter = ui.painter().clone();
    skin.panel(&painter, r, "", "");
    let o = r.min.to_vec2() - vec2(16.0, 588.0);
    let at = |x: f32, y: f32| pos2(x, y) + o;
    let rc = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(at(x, y), vec2(w, h));
    txt(&painter, at(30.0, 607.0), Align2::LEFT_CENTER, "EDIT", disp(8.0), TXT, 2.0);
    let saved = skin.acc;
    for i in 0..2 {
        skin.acc = LC[i];
        if skin.chip(ui, rc(74.0 + i as f32 * 30.0, 597.0, 26.0, 20.0), if i == 0 { "A" } else { "B" }, *edit == i, ("ed", i)).clicked() {
            *edit = i;
        }
    }
    let li = *edit;
    let col = LC[li];
    skin.acc = col;
    for (i, s) in TABS.iter().enumerate() {
        if skin.chip(ui, rc(146.0 + i as f32 * 92.0, 597.0, 88.0, 20.0), s, *tab == i, ("tab", i)).clicked() {
            *tab = i;
        }
    }
    line(&painter, at(28.0, 626.5), at(888.0, 626.5), al(CY, 0.2), 1.0);
    let l = p.lane(li);
    let t = &tr[li];
    let b = &t.b;
    let note = |s: &str, x: f32, y: f32, c: Color32| {
        txt(&painter, at(x, y), Align2::LEFT_CENTER, s, mono(10.0), c, 0.0);
    };
    let soft = rgb(0xcd, 0xef, 0xff);
    match *tab {
        0 => {
            for (row, f) in FAMS.iter().enumerate() {
                let yy = 638.0 + row as f32 * 34.0;
                lab(&painter, at(30.0, yy + 14.0), f);
                for k in 0..5 {
                    let ti = (row * 5 + k) as i32;
                    if skin.chip(ui, rc(104.0 + k as f32 * 124.0, yy, 118.0, 28.0), TYPES[ti as usize].0, b.kind == ti, ("type", ti)).clicked() {
                        set_i(setter, &l.kind, ti);
                    }
                }
            }
            let (name, desc, loss, _) = TYPES[b.kind as usize];
            txt(&painter, at(732.0, 648.0), Align2::LEFT_CENTER, name, disp(10.0), HI, 1.5);
            let mut y = 668.0;
            let mut ln = String::new();
            for wd in desc.split(' ') {
                let cand = if ln.is_empty() { wd.to_string() } else { format!("{ln} {wd}") };
                if txt_w(&painter, &cand, mono(10.0), 0.0) > 150.0 && !ln.is_empty() {
                    note(&ln, 732.0, y, soft);
                    y += 15.0;
                    ln = wd.to_string();
                } else {
                    ln = cand;
                }
            }
            note(&ln, 732.0, y, soft);
            txt(&painter, at(732.0, y + 20.0), Align2::LEFT_CENTER, format!("LOSS PER REPEAT {loss:+.1} dB"), mono(9.0), DIMT, 0.0);
            lab(&painter, at(30.0, 796.0), &format!("CHARACTER · {}", FAMS[b.fam]));
            let set: Vec<(&str, &FloatParam)> = match b.fam {
                0 => vec![("BITS", &l.bits), ("SAMPLE RATE", &l.srate), ("BANDWIDTH", &l.bw), ("NOISE", &l.dnoise), ("STUTTER", &l.stutter)],
                1 => vec![("WOW", &l.wow), ("FLUTTER", &l.flutter), ("SATURATION", &l.tsat), ("HEAD BUMP", &l.bump), ("HISS", &l.hiss)],
                2 => vec![("CLOCK NOISE", &l.clock), ("COMPANDER", &l.comp), ("DARKNESS", &l.dark), ("AGE", &l.age)],
                _ => vec![("DENSITY", &l.density), ("DETUNE", &l.detune), ("REV WINDOW", &l.revwin), ("GRAIN", &l.cgrain), ("SPREAD", &l.cspread)],
            };
            for (i, (s, prm)) in set.iter().enumerate() {
                skin.knob(ui, setter, at(150.0 + i as f32 * 112.0, 840.0), 16.0, s, *prm, false, false, false);
            }
        }
        1 => {
            lab(&painter, at(30.0, 651.0), "STYLE");
            choice_row(ui, setter, skin, at(96.0, 640.0), 96.0, &STYLES, &l.style, "style");
            lab(&painter, at(30.0, 689.0), "TIME");
            if let Some(i) = chips(ui, skin, at(96.0, 678.0), 110.0, 22.0, 6.0, &["SYNC TO HOST", "FREE (ms)"], if l.sync.value() { 0 } else { 1 }, "syncsel") {
                set_b(setter, &l.sync, i == 0);
            }
            let ratio_on = b.style == S_DUAL || b.style == S_RATIO;
            skin.knob(ui, setter, at(70.0, 750.0), 18.0, "L/R RATIO", &l.ratio, false, false, !ratio_on);
            skin.knob(ui, setter, at(174.0, 750.0), 18.0, "R OFFSET", &l.offset, false, true, false);
            skin.knob(ui, setter, at(278.0, 750.0), 18.0, "GROOVE", &l.groove, false, false, !(b.style == S_PING || b.style == S_RATIO || b.style == S_RHYTHM));
            skin.knob(ui, setter, at(382.0, 750.0), 18.0, "FEEL", &l.feel, false, true, false);
            skin.knob(ui, setter, at(486.0, 750.0), 18.0, "GLIDE", &l.glide, false, false, b.tchange == 2);
            lab(&painter, at(30.0, 839.0), "WHEN TIME CHANGES");
            choice_row(ui, setter, skin, at(176.0, 828.0), 100.0, &TCHANGE, &l.tchange, "tch");
            // first bar screen
            let sr_ = rc(600.0, 638.0, 288.0, 256.0);
            let (c, pal) = skin.screen(&painter, sr_);
            txt(&c, sr_.min + vec2(18.0, 14.0), Align2::LEFT_CENTER, "FIRST BAR · WHERE REPEATS LAND", disp(7.0), TXT, 1.5);
            let bar = 4.0 * 60.0 / bpm;
            let (rx0, rx1) = (sr_.left() + 16.0, sr_.right() - 12.0);
            let rx = |tt: f32| rx0 + tt / bar * (rx1 - rx0);
            for s in 0..=16 {
                let xx = rx(s as f32 * bar / 16.0).round() + 0.5;
                line(&c, pos2(xx, sr_.top() + 30.0), pos2(xx, sr_.top() + 222.0), if s % 4 == 0 { al(CY, 0.25) } else { pal.grid }, 1.0);
                if s % 4 == 0 {
                    txt(&c, pos2(xx, sr_.top() + 236.0), Align2::CENTER_CENTER, format!("{}", s / 4 + 1), mono(8.0), pal.axis, 0.0);
                }
            }
            txt(&c, pos2(sr_.left() + 6.0, sr_.top() + 84.0), Align2::LEFT_CENTER, "L", disp(8.0), TXT, 0.0);
            txt(&c, pos2(sr_.left() + 6.0, sr_.top() + 176.0), Align2::LEFT_CENTER, "R", disp(8.0), TXT, 0.0);
            let mut k = 0;
            let mut evs: Vec<_> = t.echoes.iter().filter(|e| e.t <= bar).collect();
            evs.sort_by(|a, b| a.t.total_cmp(&b.t));
            for e in evs {
                let xx = rx(e.t);
                let by = sr_.top() + if e.ch == 1 { 212.0 } else { 122.0 };
                let hgt = (e.db.clamp(-60.0, 0.0) + 60.0) / 60.0 * 80.0;
                line(&c, pos2(xx, by), pos2(xx, by - hgt), col, 2.0);
                if k < 6 {
                    txt(&c, pos2(xx, by - hgt - 4.0), Align2::CENTER_BOTTOM, format!("{:.0}", e.t * 1000.0), mono(8.0), col, 0.0);
                }
                k += 1;
            }
            skin.screen_end(&painter, sr_);
        }
        2 => {
            let sr_ = rc(28.0, 638.0, 520.0, 256.0);
            let (c, pal) = skin.screen(&painter, sr_);
            txt(&c, sr_.min + vec2(18.0, 14.0), Align2::LEFT_CENTER, "EACH REPEAT · LEVEL BY FREQUENCY", disp(7.0), TXT, 1.5);
            let (gx0, gx1, gy0, gy1) = (sr_.left() + 44.0, sr_.right() - 12.0, sr_.top() + 26.0, sr_.top() + 230.0);
            let fy = |d: f32| gy0 + (6.0 - d.clamp(-48.0, 6.0)) / 54.0 * (gy1 - gy0);
            skin.db_axis(&c, &pal, gx0, gx1, fy, &[6.0, 0.0, -12.0, -24.0, -36.0, -48.0], gx0 - 6.0);
            skin.freq_axis(&c, &pal, |u| gx0 + u * (gx1 - gx0), gy0, gy1, gy1 + 4.0);
            let send = 20.0 * level_gain(l.send.value()).max(1e-6).log10();
            let outg = 20.0 * level_gain(l.out.value()).max(1e-6).log10();
            let fbdb = 20.0 * (if b.kind == T_SLAP { l.fb.value() * 0.35 } else { l.fb.value() }).max(1e-6).log10();
            let tloss = TYPES[b.kind as usize].2;
            for n in 1..=4 {
                let pts: Vec<Pos2> = (0..=160)
                    .map(|i| {
                        let u = i as f32 / 160.0;
                        let f = 20.0 * 1000f32.powf(u);
                        let h = crate::lane::filt_db(f, b);
                        let passes = if b.inloop { n as f32 } else { 1.0 };
                        let d = send + outg + passes * h + (n - 1) as f32 * (if b.freeze { 0.0 } else { fbdb + tloss }) + tloss;
                        pos2(gx0 + u * (gx1 - gx0), fy(d))
                    })
                    .collect();
                let cc = al(col, 1.0 - (n - 1) as f32 * 0.22);
                let lastp = pts[155];
                polyline(&c, pts, cc, if n == 1 { 1.8 } else { 1.2 }, n == 1);
                txt(&c, lastp + vec2(0.0, -6.0), Align2::CENTER_BOTTOM, format!("{n}"), mono(9.0), cc, 0.0);
            }
            skin.screen_end(&painter, sr_);
            skin.knob(ui, setter, at(600.0, 690.0), 17.0, "RESONANCE", &l.res, false, false, false);
            skin.knob(ui, setter, at(682.0, 690.0), 17.0, "TILT", &l.tilt, false, true, false);
            skin.knob(ui, setter, at(764.0, 690.0), 17.0, "LOOP DRIVE", &l.ldrive, false, false, false);
            skin.knob(ui, setter, at(846.0, 690.0), 17.0, "L↔R CROSS", &l.xfb, false, false, false);
            lab(&painter, at(566.0, 785.0), "SLOPE");
            if let Some(i) = chips(ui, skin, at(620.0, 774.0), 62.0, 22.0, 6.0, &SLOPES, l.slope.value(), "slope") {
                set_i(setter, &l.slope, i);
            }
            lab(&painter, at(566.0, 817.0), "FILTER");
            if let Some(i) = chips(ui, skin, at(620.0, 806.0), 124.0, 22.0, 6.0, &["IN THE LOOP", "ON OUTPUT"], if l.inloop.value() { 0 } else { 1 }, "inloop") {
                set_b(setter, &l.inloop, i == 0);
            }
            let fr = l.freeze.value();
            if skin.chip(ui, rc(620.0, 838.0, 254.0, 24.0), if fr { "FROZEN · INFINITE HOLD" } else { "FREEZE (INFINITE HOLD)" }, fr, "freeze").clicked() {
                set_b(setter, &l.freeze, !fr);
            }
            let rt = t.rt60();
            let s = format!(
                "EACH REPEAT {} dB  ·  {}",
                fmt_db(t.per),
                if fr {
                    "HOLDING".to_string()
                } else if rt.is_finite() {
                    format!("GONE (-60 dB) IN {rt:.1} s")
                } else {
                    "OVER 100%: SAFE LIMITER HOLDS IT".into()
                }
            );
            txt(&painter, at(566.0, 884.0), Align2::LEFT_CENTER, s, mono(9.0), soft, 0.0);
        }
        3 => {
            lab(&painter, at(30.0, 651.0), "SHAPE");
            choice_row(ui, setter, skin, at(96.0, 640.0), 88.0, &MOD_SHAPES, &l.mshape, "msh");
            lab(&painter, at(30.0, 689.0), "MODULATES");
            choice_row(ui, setter, skin, at(96.0, 678.0), 88.0, &MOD_TARGETS, &l.mtgt, "mtg");
            skin.knob(ui, setter, at(70.0, 760.0), 18.0, "RATE", &l.mrate, false, false, false);
            skin.knob(ui, setter, at(190.0, 760.0), 18.0, "STEREO PHASE", &l.mphase, false, false, false);
            skin.knob(ui, setter, at(310.0, 760.0), 18.0, "ENV SENS", &l.menv, false, false, l.mshape.value() != 4);
            skin.knob(ui, setter, at(430.0, 760.0), 18.0, "DRIFT", &l.mdrift, false, false, false);
            note(&format!("DEPTH = the MOD knob on lane {} ({:.0}%).", if li == 0 { "A" } else { "B" }, l.modu.value() * 100.0), 30.0, 842.0, soft);
            let sr_ = rc(560.0, 638.0, 328.0, 256.0);
            let (c, pal) = skin.screen(&painter, sr_);
            let tgt = l.mtgt.value();
            let (unit, full) = match tgt {
                0 => ("±ms", 8.0),
                1 => ("± octaves", 2.0),
                2 => ("PAN ±", 100.0),
                _ => ("± st", 1.0),
            };
            txt(&c, sr_.min + vec2(18.0, 14.0), Align2::LEFT_CENTER, format!("{} MOVEMENT · {unit}", MOD_TARGETS[tgt as usize]), disp(7.0), TXT, 1.5);
            let depth = l.modu.value() * full;
            let (mx0, mx1, mc) = (sr_.left() + 40.0, sr_.right() - 12.0, sr_.top() + 128.0);
            let sc = depth.max(full * 0.06);
            let my = |d: f32| mc - d / sc * 80.0;
            for k in [1.0f32, 0.0, -1.0] {
                let yy = my(k * sc);
                line(&c, pos2(mx0, yy), pos2(mx1, yy), pal.grid, 1.0);
                txt(&c, pos2(mx0 - 6.0, yy), Align2::RIGHT_CENTER, if k == 0.0 { "0".into() } else { format!("{}{:.1}", if k > 0.0 { "+" } else { "-" }, sc) }, mono(8.0), pal.axis, 0.0);
            }
            let rate = l.mrate.value();
            let ph = l.mphase.value() / 360.0;
            let shape = l.mshape.value();
            let lfo = |pp: f32| -> f32 {
                let pp = pp.rem_euclid(1.0);
                match shape {
                    0 => (std::f32::consts::TAU * pp).sin(),
                    1 => 1.0 - 4.0 * (pp - 0.5).abs(),
                    2 => ((pp * 8.0).floor() * 12.9898).sin() * 0.9,
                    3 => 0.7 * (std::f32::consts::TAU * pp).sin() + 0.3 * (std::f32::consts::TAU * pp * 7.3).sin(),
                    _ => 0.0,
                }
            };
            let tt = skin.time as f32;
            for ch in 0..2 {
                let pts: Vec<Pos2> = (0..=200).map(|i| {
                    let u = i as f32 / 200.0;
                    pos2(mx0 + u * (mx1 - mx0), my(depth * lfo(u * 2.0 * rate + tt * rate + if ch == 1 { ph } else { 0.0 })))
                }).collect();
                if ch == 1 {
                    for w in pts.windows(2).step_by(2) {
                        line(&c, w[0], w[1], col, 1.5);
                    }
                } else {
                    polyline(&c, pts, col, 1.5, true);
                }
            }
            if shape == 4 {
                txt(&c, pos2(sr_.center().x, mc + 40.0), Align2::CENTER_CENTER, "FOLLOWS THE INPUT LEVEL", mono(9.0), pal.axis, 1.0);
            }
            txt(&c, pos2(mx0, sr_.bottom() - 12.0), Align2::CENTER_CENTER, "0 s", mono(8.0), pal.axis, 0.0);
            txt(&c, pos2(mx1, sr_.bottom() - 12.0), Align2::CENTER_CENTER, "2 s", mono(8.0), pal.axis, 0.0);
            txt(&c, pos2(mx1, sr_.top() + 14.0), Align2::RIGHT_CENTER, "— L   - - R", mono(8.0), pal.axis, 0.0);
            skin.screen_end(&painter, sr_);
        }
        4 => {
            lab(&painter, at(30.0, 651.0), "MODE");
            if b.kind == T_SHIMMER {
                note("SHIMMER type always shifts every repeat (+12 st when PITCH is 0).", 96.0, 651.0, soft);
            } else {
                choice_row(ui, setter, skin, at(96.0, 640.0), 120.0, &PMODES, &l.pmode, "pm");
            }
            note("UP / DOWN: left repeats go up, right ones go down (try it with PING-PONG).", 30.0, 700.0, soft);
            skin.knob(ui, setter, at(70.0, 760.0), 18.0, "FINE", &l.fine, false, true, false);
            skin.knob(ui, setter, at(190.0, 760.0), 18.0, "PITCH MIX", &l.pmix, false, false, false);
            skin.knob(ui, setter, at(310.0, 760.0), 18.0, "GRAIN", &l.pgrain, false, false, false);
            skin.knob(ui, setter, at(430.0, 760.0), 18.0, "L/R DETUNE", &l.pspread, false, false, false);
            note("PITCH (the lane knob) sets semitones. GRAIN: short = tight, long = smooth.", 30.0, 842.0, soft);
            let sr_ = rc(560.0, 638.0, 328.0, 256.0);
            let (c, pal) = skin.screen(&painter, sr_);
            txt(&c, sr_.min + vec2(18.0, 14.0), Align2::LEFT_CENTER, "PITCH OF EACH REPEAT", disp(7.0), TXT, 1.5);
            let (px0, px1, pc) = (sr_.left() + 52.0, sr_.right() - 18.0, sr_.top() + 128.0);
            let py = |s: f32| pc - s / 24.0 * 96.0;
            for s in [24.0f32, 12.0, 0.0, -12.0, -24.0] {
                let yy = py(s).round() + 0.5;
                line(&c, pos2(px0, yy), pos2(px1, yy), pal.grid, 1.0);
                txt(&c, pos2(px0 - 6.0, yy), Align2::RIGHT_CENTER, format!("{s:+.0}"), mono(8.0), pal.axis, 0.0);
            }
            for n in 1..=8u32 {
                let s = match b.pmode {
                    0 => b.pitch_st,
                    1 => b.pitch_st * n as f32,
                    _ => {
                        if n % 2 == 1 {
                            b.pitch_st
                        } else {
                            -b.pitch_st
                        }
                    }
                };
                let xx = px0 + (n as f32 - 0.5) / 8.0 * (px1 - px0);
                let yy = py(s.clamp(-24.0, 24.0));
                c.rect_filled(Rect::from_min_max(pos2(xx - 9.0, yy.min(py(0.0))), pos2(xx + 9.0, yy.max(py(0.0)) + 1.0)), 0.0, al(col, 0.35));
                line(&c, pos2(xx - 9.0, yy), pos2(xx + 9.0, yy), col, 2.0);
                txt(&c, pos2(xx, if s < 0.0 { yy + 10.0 } else { yy - 7.0 }), Align2::CENTER_CENTER, format!("{s:+.0}"), mono(8.0), col, 0.0);
                txt(&c, pos2(xx, sr_.bottom() - 12.0), Align2::CENTER_CENTER, format!("{n}"), mono(8.0), pal.axis, 0.0);
            }
            txt(&c, pos2(px1, sr_.top() + 14.0), Align2::RIGHT_CENTER, "REPEAT", mono(8.0), pal.axis, 0.0);
            skin.screen_end(&painter, sr_);
        }
        5 => {
            lab(&painter, at(30.0, 651.0), "WHERE");
            choice_row(ui, setter, skin, at(96.0, 640.0), 150.0, &DPOS, &l.dpos, "dp");
            skin.knob(ui, setter, at(70.0, 746.0), 18.0, "SIZE", &l.dsize, false, false, false);
            skin.knob(ui, setter, at(190.0, 746.0), 18.0, "DENSITY", &l.dstages, false, false, false);
            skin.knob(ui, setter, at(310.0, 746.0), 18.0, "MOTION", &l.dmotion, false, false, false);
            skin.knob(ui, setter, at(430.0, 746.0), 18.0, "SPREAD", &l.dspread, false, false, false);
            note("Use a lane as a reverb: DIFFUSE up, IN THE LOOP, FEEDBACK 50-80%.", 30.0, 836.0, soft);
            let rt = t.rt60();
            note(&format!("Lane {} now: {} · diffusion {:.0}%", if li == 0 { "A" } else { "B" }, if rt.is_finite() { format!("RT60 {rt:.1} s") } else { "holding".into() }, b.diff * 100.0), 30.0, 856.0, col);
            let sr_ = rc(560.0, 638.0, 328.0, 256.0);
            let (c, pal) = skin.screen(&painter, sr_);
            txt(&c, sr_.min + vec2(18.0, 14.0), Align2::LEFT_CENTER, "ONE REPEAT, SMEARED", disp(7.0), TXT, 1.5);
            let (dx0, dx1) = (sr_.left() + 40.0, sr_.right() - 12.0);
            let dx = |ms: f32| dx0 + ms / 300.0 * (dx1 - dx0);
            let dy = |d: f32| sr_.top() + 22.0 + (-d.clamp(-48.0, 0.0)) / 48.0 * 200.0;
            for d in [0.0f32, -24.0, -48.0] {
                line(&c, pos2(dx0, dy(d)), pos2(dx1, dy(d)), pal.grid, 1.0);
                txt(&c, pos2(dx0 - 6.0, dy(d)), Align2::RIGHT_CENTER, format!("{d:.0}"), mono(8.0), pal.axis, 0.0);
            }
            for ms in [0.0f32, 100.0, 200.0, 300.0] {
                line(&c, pos2(dx(ms), sr_.top() + 22.0), pos2(dx(ms), sr_.top() + 222.0), pal.grid, 1.0);
                txt(&c, pos2(dx(ms), sr_.top() + 236.0), Align2::CENTER_CENTER, format!("{ms:.0} ms"), mono(8.0), pal.axis, 0.0);
            }
            let sg = (b.diff * b.dsize * 1.5).max(0.002) * 1000.0;
            let spread_db = if b.diff > 0.01 { (10.0 * (sg / 6.0 + 1.0).log10()).min(18.0) } else { 0.0 };
            let mut seed = 7u32;
            let mut rn = || {
                seed = seed.wrapping_mul(16807) % 2147483647;
                seed as f32 / 2147483647.0
            };
            let dens = 8 + (b.dstages as f32 * 5.0) as usize;
            for _ in 0..dens {
                let ms = 40.0 + (rn() - 0.3) * sg * 3.0;
                if !(0.0..=300.0).contains(&ms) {
                    continue;
                }
                let d = -((ms - 40.0) / sg).powi(2) * 4.3 - rn() * 10.0 - spread_db;
                line(&c, pos2(dx(ms), sr_.top() + 222.0), pos2(dx(ms), dy(d)), al(col, 0.8), 1.4);
            }
            let pts: Vec<Pos2> = (0..=150).map(|i| {
                let ms = i as f32 / 150.0 * 300.0;
                pos2(dx(ms), dy(-((ms - 40.0) / sg).powi(2) * 4.3 - spread_db))
            }).collect();
            polyline(&c, pts, col, 1.4, true);
            skin.screen_end(&painter, sr_);
        }
        6 => {
            lab(&painter, at(30.0, 651.0), "LISTENS TO");
            choice_row(ui, setter, skin, at(96.0, 640.0), 110.0, &HEARS, &l.hears, "hr");
            skin.knob(ui, setter, at(70.0, 746.0), 18.0, "AMOUNT", &l.damt, false, false, false);
            skin.knob(ui, setter, at(190.0, 746.0), 18.0, "THRESHOLD", &l.dthr, false, false, false);
            skin.knob(ui, setter, at(310.0, 746.0), 18.0, "ATTACK", &l.datt, false, false, false);
            skin.knob(ui, setter, at(430.0, 746.0), 18.0, "RELEASE", &l.drel, false, false, false);
            note("The repeats duck while the vocal sings and come back in the gaps.", 30.0, 842.0, soft);
            if l.hears.value() == 1 {
                note("SIDECHAIN: route a track into ThrowBack's sidechain input in FL.", 30.0, 862.0, DIMT);
            }
            let sr_ = rc(560.0, 638.0, 328.0, 256.0);
            let (c, pal) = skin.screen(&painter, sr_);
            txt(&c, sr_.min + vec2(18.0, 14.0), Align2::LEFT_CENTER, "DUCKING · LAST 4 s (LIVE)", disp(7.0), TXT, 1.5);
            let (ux0, ux1) = (sr_.left() + 40.0, sr_.right() - 12.0);
            let uy = |d: f32| sr_.top() + 26.0 + (-d.clamp(-60.0, 0.0)) / 60.0 * 196.0;
            for d in [0.0f32, -20.0, -40.0, -60.0] {
                line(&c, pos2(ux0, uy(d)), pos2(ux1, uy(d)), pal.grid, 1.0);
                txt(&c, pos2(ux0 - 6.0, uy(d)), Align2::RIGHT_CENTER, format!("{d:.0}"), mono(8.0), pal.axis, 0.0);
            }
            let thr = l.dthr.value();
            dashed(&c, pos2(ux0, uy(thr)), pos2(ux1, uy(thr)), Color32::from_white_alpha(100), 1.0, 3.0, 3.0);
            txt(&c, pos2(ux1 - 2.0, uy(thr) - 6.0), Align2::RIGHT_CENTER, "THRESHOLD", mono(8.0), Color32::from_white_alpha(150), 0.0);
            let h = &duck_hist[li];
            if h.len() > 1 {
                let xs = |i: usize| ux1 - (h.len() - 1 - i) as f32 / 119.0 * (ux1 - ux0);
                let a: Vec<Pos2> = h.iter().enumerate().map(|(i, v)| pos2(xs(i), uy(v.0))).collect();
                polyline(&c, a, Color32::from_rgba_unmultiplied(220, 230, 240, 140), 1.2, false);
                let g: Vec<Pos2> = h.iter().enumerate().map(|(i, v)| pos2(xs(i), uy(v.1))).collect();
                polyline(&c, g, col, 1.8, true);
                txt(&c, pos2(ux1, sr_.top() + 14.0), Align2::RIGHT_CENTER, format!("GAIN {:.1} dB", h.last().map(|v| v.1).unwrap_or(0.0)), mono(9.0), col, 0.0);
            }
            txt(&c, pos2(ux0, sr_.bottom() - 12.0), Align2::LEFT_CENTER, "— INPUT  — LANE GAIN", mono(8.0), pal.axis, 0.0);
            skin.screen_end(&painter, sr_);
        }
        _ => {
            if b.style != S_RHYTHM {
                if skin.chip(ui, rc(30.0, 640.0, 220.0, 22.0), "SWITCH LANE TO RHYTHM STYLE", false, "torh").clicked() {
                    set_i(setter, &l.style, S_RHYTHM);
                }
            } else {
                note("RHYTHM STYLE IS ON", 30.0, 651.0, col);
            }
            let (cw, gx, gy, gh) = (52.0, 36.0, 680.0, 170.0);
            for k in 0..16 {
                let cr = rc(gx + k as f32 * cw, gy, cw - 6.0, gh);
                let v = l.steps[k].v.value();
                let resp = ui.interact(cr, ui.scope_id().with(("step", li, k)), egui::Sense::click());
                cham_fill(&painter, cr, 4.0, 4.0, if k % 4 == 0 { al(CY, 0.08) } else { al(CY, 0.03) }, Stroke::new(1.0_f32, if resp.hovered() { al(col, 0.8) } else { al(CY, 0.3) }));
                if v > 0 {
                    let g = STEP_GAIN[v as usize];
                    let hh = g * (gh - 30.0);
                    painter.rect_filled(Rect::from_min_max(pos2(cr.left() + 6.0, cr.bottom() - 6.0 - hh), pos2(cr.right() - 6.0, cr.bottom() - 6.0)), 0.0, al(col, 0.75));
                    txt(&painter, pos2(cr.center().x, cr.bottom() - 12.0 - hh), Align2::CENTER_BOTTOM, format!("{:.0}", 20.0 * g.log10()), mono(8.0), Color32::WHITE, 0.0);
                }
                let beat_lab = ["1", "e", "&", "a"][k % 4];
                let s = if k % 4 == 0 { format!("{}", k / 4 + 1) } else { beat_lab.to_string() };
                txt(&painter, pos2(cr.center().x, cr.bottom() + 12.0), Align2::CENTER_CENTER, s, mono(9.0), if k % 4 == 0 { TXT } else { DIMT }, 0.0);
                txt(&painter, pos2(cr.center().x, cr.top() + 14.0), Align2::CENTER_CENTER, if k % 2 == 1 { "R" } else { "L" }, mono(8.0), DIMT, 0.0);
                if resp.clicked() {
                    set_i(setter, &l.steps[k].v, (v + 1) % 4);
                }
                if resp.secondary_clicked() {
                    set_i(setter, &l.steps[k].v, 0);
                }
            }
            note("One bar, 16 steps. FEEDBACK repeats the whole bar. Click a step: OFF · -6 · -2 · 0 dB (right-click = off).", 30.0, 890.0, soft);
        }
    }
    skin.acc = saved;
}

#[cfg(test)]
mod tests {
    use super::*;
    use nice_plug::context::gui::GuiContextInner;
    use nice_plug::plugin::PluginState;
    use egui::{epaint, Event, ImageData, Modifiers, PointerButton, RawInput, TextureId, TextureWrapMode, Vec2};
    use std::collections::HashMap;

    struct Mock;
    impl GuiContextInner for Mock {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }
        fn request_restart(&self) {}
        unsafe fn raw_begin_set_parameter(&self, _p: ParamPtr) {}
        unsafe fn raw_set_parameter_normalized(&self, _p: ParamPtr, _n: f32) {}
        unsafe fn raw_end_set_parameter(&self, _p: ParamPtr) {}
        fn get_state(&self) -> PluginState {
            PluginState { version: String::new(), params: Default::default(), fields: Default::default() }
        }
        fn set_state(&self, _s: PluginState) {}
    }

    /// Draws every tab for both lanes, every routing and zoom, and clicks around:
    /// nothing may panic, and the whole UI must fit the window.
    #[test]
    fn every_tab_draws() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let params = TbParams::default();
        let shared = Shared::default();
        let mock = Mock;
        let setter = ParamSetter::new(&mock);
        let mut st = new_state();
        let mut i = 0;
        let mut run = |st: &mut St, ev: Vec<Event>| {
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(WIDTH as f32, HEIGHT as f32))), time: Some(i as f64 * 0.04), events: ev, ..Default::default() };
            i += 1;
            let _ = ctx.run_ui(raw, |ctx| frame(ctx, &setter, st, &params, &shared));
        };
        for edit in 0..2 {
            for tab in 0..8 {
                for zoom in 0..3 {
                    st.edit = edit;
                    st.tab = tab;
                    st.zoom = zoom;
                    run(&mut st, vec![]);
                }
            }
        }
        // click a grid of points (chips, steps, type buttons); the mock ignores the changes
        for y in (60..940).step_by(37) {
            for x in (20..990).step_by(53) {
                let q = Pos2::new(x as f32, y as f32);
                run(&mut st, vec![Event::PointerMoved(q), Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
                run(&mut st, vec![Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
            }
        }
    }
    /// Tiny software rasteriser for egui meshes, so the real window can be checked as a PNG.
    struct Tex {
        w: usize,
        h: usize,
        px: Vec<[f32; 4]>,
        repeat: bool,
    }
    fn apply_delta(texs: &mut HashMap<TextureId, Tex>, out: &egui::FullOutput) {
        for (id, d) in out.textures_delta.set.iter().flat_map(|(id, ds)| ds.iter().map(move |d| (id, d))) {
            let (w, h, px): (usize, usize, Vec<[f32; 4]>) = match &d.image {
                ImageData::Color(c) => (c.size[0], c.size[1], c.pixels.iter().map(|p| [p.r() as f32 / 255.0, p.g() as f32 / 255.0, p.b() as f32 / 255.0, p.a() as f32 / 255.0]).collect()),
            };
            let repeat = d.options.wrap_mode == TextureWrapMode::Repeat;
            if let Some(pos) = d.pos {
                let t = texs.get_mut(id).unwrap();
                for y in 0..h {
                    for x in 0..w {
                        t.px[(pos[1] + y) * t.w + pos[0] + x] = px[y * w + x];
                    }
                }
            } else {
                texs.insert(*id, Tex { w, h, px, repeat });
            }
        }
    }
    fn render(ctx: &egui::Context, out: egui::FullOutput, texs: &mut HashMap<TextureId, Tex>, path: &str) {
        apply_delta(texs, &out);
        let (w, h) = (WIDTH as usize, HEIGHT as usize);
        let mut img = vec![[0.0f32; 3]; w * h];
        let prims = ctx.tessellate(out.shapes, 1.0);
        for cp in prims {
            let epaint::Primitive::Mesh(mesh) = cp.primitive else { continue };
            let clip = cp.clip_rect;
            let tex = texs.get(&mesh.texture_id);
            for tri in mesh.indices.chunks(3) {
                let v = [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
                let (x0, x1) = (v.iter().map(|q| q.pos.x).fold(f32::MAX, f32::min).max(clip.left()).max(0.0), v.iter().map(|q| q.pos.x).fold(f32::MIN, f32::max).min(clip.right()).min(w as f32));
                let (y0, y1) = (v.iter().map(|q| q.pos.y).fold(f32::MAX, f32::min).max(clip.top()).max(0.0), v.iter().map(|q| q.pos.y).fold(f32::MIN, f32::max).min(clip.bottom()).min(h as f32));
                let area = (v[1].pos.x - v[0].pos.x) * (v[2].pos.y - v[0].pos.y) - (v[2].pos.x - v[0].pos.x) * (v[1].pos.y - v[0].pos.y);
                if area.abs() < 1e-6 {
                    continue;
                }
                for py in (y0.floor() as usize)..(y1.ceil() as usize).min(h) {
                    for px in (x0.floor() as usize)..(x1.ceil() as usize).min(w) {
                        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                        let e = |a: &epaint::Vertex, b: &epaint::Vertex| (b.pos.x - a.pos.x) * (fy - a.pos.y) - (fx - a.pos.x) * (b.pos.y - a.pos.y);
                        let (w0, w1, w2) = (e(&v[1], &v[2]) / area, e(&v[2], &v[0]) / area, e(&v[0], &v[1]) / area);
                        if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                            continue;
                        }
                        let mut c = [0.0f32; 4];
                        for i in 0..4 {
                            c[i] = (w0 * v[0].color.to_array()[i] as f32 + w1 * v[1].color.to_array()[i] as f32 + w2 * v[2].color.to_array()[i] as f32) / 255.0;
                        }
                        if let Some(t) = tex {
                            let mut u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                            let mut vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                            if t.repeat {
                                u = u.rem_euclid(1.0);
                                vv = vv.rem_euclid(1.0);
                            }
                            let tx = ((u * t.w as f32) as usize).min(t.w - 1);
                            let ty = ((vv * t.h as f32) as usize).min(t.h - 1);
                            let s = t.px[ty * t.w + tx];
                            for i in 0..4 {
                                c[i] *= s[i];
                            }
                        }
                        // premultiplied (gamma-space) blend
                        let d = &mut img[py * w + px];
                        for i in 0..3 {
                            d[i] = c[i] + d[i] * (1.0 - c[3]);
                        }
                    }
                }
            }
        }
        let bytes: Vec<u8> = img.iter().flat_map(|p| p.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect::<Vec<_>>()).collect();
        std::fs::write(path, bytes).unwrap();
    }

    fn raw(i: usize, ev: Vec<Event>) -> RawInput {
        RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(WIDTH as f32, HEIGHT as f32))), time: Some(i as f64 * 0.04), events: ev, ..Default::default() }
    }


    /// Renders the window to a raw RGB file when SHOT_DIR is set.
    #[test]
    fn screenshots() {
        let Ok(dir) = std::env::var("SHOT_DIR") else { return };
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let p = TbParams::default();
        let sh = Shared::default();
        let mock = Mock;
        let setter = ParamSetter::new(&mock);
        let mut st = new_state();
        let mut texs = HashMap::new();
        let mut last = None;
        for f in 0..4 {
            last = Some(ctx.run_ui(raw(f, vec![]), |c| frame(c, &setter, &mut st, &p, &sh)));
            if f < 3 {
                apply_delta(&mut texs, last.as_ref().unwrap());
            }
        }
        render(&ctx, last.unwrap(), &mut texs, &format!("{}/tb0.rgb", dir));
    }
}
