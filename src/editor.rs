//! Edytor egui — techniczny przekrój werbla z parametrami modyfikowanymi
//! przez przeciąganie elementów rysunku (naciąg, korpus, sprężyny, pałeczka...).

use crate::WerbelParams;
use nice_plug::{context::gui::GuiContext, prelude::*};
use nice_plug_egui::{NiceEguiApp, resizable_window::ResizableWindow, widgets};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Presety
// ---------------------------------------------------------------------------
struct Preset {
    name: &'static str,
    tone: f32,
    ring: f32,
    damping: f32,
    snare: f32,
    snap: f32,
    sweep: f32,
    glue: f32,
    verb: f32,
}

const PRESETS: &[Preset] = &[
    Preset { name: "Studio Maple (ciepły)",    tone: 200.0, ring: 0.80, damping: 3200.0, snare: 0.55, snap: 0.70, sweep: 0.25, glue: 0.40, verb: 0.25 },
    Preset { name: "Tight Piccolo Crack",      tone: 350.0, ring: 0.55, damping: 6000.0, snare: 0.70, snap: 0.90, sweep: 0.18, glue: 0.50, verb: 0.20 },
    Preset { name: "Fat & Deep",               tone: 130.0, ring: 0.90, damping: 2500.0, snare: 0.40, snap: 0.60, sweep: 0.45, glue: 0.50, verb: 0.30 },
    Preset { name: "Brass Supraphonic (ring)", tone: 220.0, ring: 0.95, damping: 8000.0, snare: 0.60, snap: 0.75, sweep: 0.20, glue: 0.35, verb: 0.28 },
    Preset { name: "Dry Vintage (muffled)",    tone: 210.0, ring: 0.60, damping: 2200.0, snare: 0.40, snap: 0.65, sweep: 0.15, glue: 0.55, verb: 0.15 },
    Preset { name: "Funk Snap",                tone: 250.0, ring: 0.70, damping: 5000.0, snare: 0.65, snap: 0.85, sweep: 0.30, glue: 0.45, verb: 0.20 },
    Preset { name: "Open Ringy (bez tłumika)", tone: 190.0, ring: 0.99, damping: 9000.0, snare: 0.60, snap: 0.70, sweep: 0.20, glue: 0.30, verb: 0.35 },
    Preset { name: "Electro Snip",             tone: 300.0, ring: 0.50, damping: 7000.0, snare: 0.80, snap: 0.95, sweep: 0.10, glue: 0.60, verb: 0.10 },
    Preset { name: "Lo-Fi Tape",               tone: 170.0, ring: 0.70, damping: 1800.0, snare: 0.45, snap: 0.60, sweep: 0.30, glue: 0.60, verb: 0.20 },
    Preset { name: "New Wave Gated",           tone: 320.0, ring: 0.45, damping: 7500.0, snare: 0.85, snap: 0.90, sweep: 0.12, glue: 0.70, verb: 0.05 },
];

// ---------------------------------------------------------------------------
// Elementy rysunku (strefy przeciągania)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    None,
    Tone,
    Ring,
    Snare,
    Snap,
    Damping,
    Sweep,
}

struct Rects {
    top_rim: egui::Rect,
    batter: egui::Rect,
    shell: egui::Rect,
    reson: egui::Rect,
    bottom_rim: egui::Rect,
    snares: egui::Rect,
    sticks: egui::Rect,
    rod: egui::Rect,
}

fn layout(r: egui::Rect) -> Rects {
    let cx = r.center().x;
    let drum_w = r.width() * 0.44;
    let half = drum_w / 2.0;
    let rim_h = 14.0;
    let y0 = r.top() + 40.0;

    let top_rim = egui::Rect::from_min_size(egui::pos2(cx - half - 8.0, y0), egui::vec2(drum_w + 16.0, rim_h));
    let batter = egui::Rect::from_min_size(egui::pos2(cx - half, top_rim.bottom() + 3.0), egui::vec2(drum_w, 10.0));
    let shell = egui::Rect::from_min_size(egui::pos2(cx - half, batter.bottom() + 2.0), egui::vec2(drum_w, 240.0));
    let reson = egui::Rect::from_min_size(egui::pos2(cx - half, shell.bottom() + 2.0), egui::vec2(drum_w, 10.0));
    let bottom_rim = egui::Rect::from_min_size(egui::pos2(cx - half - 8.0, reson.bottom() + 3.0), egui::vec2(drum_w + 16.0, rim_h));
    let snares = egui::Rect::from_min_size(egui::pos2(cx - half - 12.0, bottom_rim.bottom() + 6.0), egui::vec2(drum_w + 24.0, 60.0));
    let sticks = egui::Rect::from_min_size(egui::pos2(cx + 10.0, r.top() + 4.0), egui::vec2(r.width() * 0.42, 80.0));
    let rod = egui::Rect::from_min_size(egui::pos2(cx + half - 18.0, top_rim.top() - 14.0), egui::vec2(30.0, bottom_rim.bottom() + 28.0 - top_rim.top()));

    Rects { top_rim, batter, shell, reson, bottom_rim, snares, sticks, rod }
}

fn hit_test(d: &Rects, p: egui::Pos2) -> Drag {
    if d.batter.contains(p) { Drag::Tone }
    else if d.snares.contains(p) { Drag::Snare }
    else if d.sticks.contains(p) { Drag::Snap }
    else if d.rod.contains(p) { Drag::Sweep }
    else if d.shell.contains(p) { Drag::Ring }
    else if d.top_rim.contains(p) { Drag::Damping }
    else { Drag::None }
}

/// Blueprint werbla jako surowe RGBA 940x940 (generowane gen_blueprint.py).
const BLUEPRINT: &[u8] = include_bytes!("../assets/werbel_blueprint.rgba");
const BP_W: usize = 940;
const BP_H: usize = 940;

// Tło panelu — zbiegające z blueprintem (ciemnoniebieskie).
const BG: egui::Color32 = egui::Color32::from_rgb(0x16, 0x3a, 0x6e);
fn hl_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(110, 180, 255, 46)
}
const HL_LINE: egui::Color32 = egui::Color32::from_rgb(140, 210, 255);

/// Prostokąt strefy danego elementu (do podświetlania).
fn zone_rect(d: &Rects, drag: Drag) -> egui::Rect {
    match drag {
        Drag::Tone => d.batter,
        Drag::Ring => d.shell,
        Drag::Snare => d.snares,
        Drag::Snap => d.sticks,
        Drag::Sweep => d.rod,
        Drag::Damping => d.top_rim,
        Drag::None => egui::Rect::NOTHING,
    }
}

struct OpenState {
    nice_gui_ctx: GuiContext,
    tex: egui::TextureHandle,
}

pub struct WerbelEditor {
    open_state: Option<OpenState>,
    params: Arc<WerbelParams>,
    preset_idx: usize,
    active_drag: Drag,
    about_open: bool,
}

impl WerbelEditor {
    pub fn new(params: Arc<WerbelParams>) -> Self {
        Self { open_state: None, params, preset_idx: 0, active_drag: Drag::None, about_open: false }
    }
}

impl NiceEguiApp for WerbelEditor {
    fn build(
        &mut self,
        egui_ctx: egui::Context,
        nice_gui_ctx: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        let image = egui::ColorImage::from_rgba_unmultiplied([BP_W, BP_H], BLUEPRINT);
        let tex = egui_ctx.load_texture(
            "werbel-blueprint",
            image,
            egui::TextureOptions::LINEAR,
        );
        self.open_state = Some(OpenState { nice_gui_ctx, tex });
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.open_state = None;
        self.active_drag = Drag::None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(state) = self.open_state.as_mut() else { return };
        let setter = state.nice_gui_ctx.param_setter();

        ResizableWindow::new("werbel-window")
            .min_size(egui::Vec2::new(660.0, 380.0))
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            // ===== LEWA: interaktywny przekrój =====
                            let (resp, painter) = ui.allocate_painter(
                                egui::vec2(470.0, 470.0),
                                egui::Sense::click_and_drag(),
                            );
                            let d = layout(resp.rect);

                            if resp.drag_started() {
                                if let Some(p0) = resp.interact_pointer_pos() {
                                    self.active_drag = hit_test(&d, p0);
                                }
                            }
                            if resp.dragged() && self.active_drag != Drag::None {
                                let dd = resp.drag_delta();
                                match self.active_drag {
                                    Drag::Tone => {
                                        let p = &self.params.tone;
                                        let v = (p.value() - dd.y * 1.4).clamp(80.0, 600.0);
                                        setter.begin_set_parameter(p);
                                        setter.set_parameter(p, v);
                                        setter.end_set_parameter(p);
                                    }
                                    Drag::Ring => {
                                        let p = &self.params.ring;
                                        let v = (p.value() - dd.y * 0.002).clamp(0.5, 0.998);
                                        setter.begin_set_parameter(p);
                                        setter.set_parameter(p, v);
                                        setter.end_set_parameter(p);
                                    }
                                    Drag::Snare => {
                                        let p = &self.params.snare;
                                        let v = (p.value() - dd.y * 0.0026).clamp(0.0, 1.0);
                                        setter.begin_set_parameter(p);
                                        setter.set_parameter(p, v);
                                        setter.end_set_parameter(p);
                                    }
                                    Drag::Snap => {
                                        let p = &self.params.snap;
                                        let v = (p.value() - dd.y * 0.0026).clamp(0.0, 1.0);
                                        setter.begin_set_parameter(p);
                                        setter.set_parameter(p, v);
                                        setter.end_set_parameter(p);
                                    }
                                    Drag::Sweep => {
                                        let p = &self.params.sweep;
                                        let v = (p.value() - dd.y * 0.002).clamp(0.0, 1.0);
                                        setter.begin_set_parameter(p);
                                        setter.set_parameter(p, v);
                                        setter.end_set_parameter(p);
                                    }
                                    Drag::Damping => {
                                        let p = &self.params.damping;
                                        let v = (p.value() + dd.x * 10.0).clamp(300.0, 14_000.0);
                                        setter.begin_set_parameter(p);
                                        setter.set_parameter(p, v);
                                        setter.end_set_parameter(p);
                                    }
                                    Drag::None => {}
                                }
                            }
                            if resp.drag_stopped() {
                                self.active_drag = Drag::None;
                            }

                            // blueprint jako tło + podświetlenie aktywnej strefy
                            let tex_id = state.tex.id();
                            painter.image(
                                tex_id,
                                resp.rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                            if self.active_drag != Drag::None {
                                let r = zone_rect(&d, self.active_drag);
                                painter.rect_filled(r, 2.0, hl_fill());
                                painter.rect_stroke(
                                    r,
                                    2.0,
                                    egui::Stroke::new(1.5, HL_LINE),
                                    egui::StrokeKind::Inside,
                                );
                            }

                            ui.separator();

                            // ===== PRAWA: presety + suwaki =====
                            let w = ui.available_width() - 6.0;
                            ui.vertical(|ui| {
                                ui.set_width(w);
                                ui.horizontal(|ui| {
                                    ui.heading("Werbel");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("?").clicked() {
                                            self.about_open = true;
                                        }
                                    });
                                });
                                ui.small("feedback synthesis · gwolaniuk");
                                ui.small("przeciągaj elementy rysunku (naciąg, korpus, sprężyny…)");
                                ui.add_space(4.0);

                                let sp = |param: &FloatParam, v: f32| {
                                    setter.begin_set_parameter(param);
                                    setter.set_parameter(param, v);
                                    setter.end_set_parameter(param);
                                };

                                let cur_name = if self.preset_idx == 0 {
                                    "— Custom —"
                                } else {
                                    PRESETS[self.preset_idx - 1].name
                                };
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_label("Preset")
                                        .selected_text(cur_name)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(self.preset_idx == 0, "— Custom —").clicked() {
                                                self.preset_idx = 0;
                                            }
                                            for (i, pres) in PRESETS.iter().enumerate() {
                                                if ui.selectable_label(self.preset_idx == i + 1, pres.name).clicked() {
                                                    self.preset_idx = i + 1;
                                                    sp(&self.params.tone, pres.tone);
                                                    sp(&self.params.ring, pres.ring);
                                                    sp(&self.params.damping, pres.damping);
                                                    sp(&self.params.snare, pres.snare);
                                                    sp(&self.params.snap, pres.snap);
                                                    sp(&self.params.sweep, pres.sweep);
                                                    sp(&self.params.glue, pres.glue);
                                                    sp(&self.params.verb, pres.verb);
                                                }
                                            }
                                        });
                                    if ui.button("🎲 Randomize").clicked() {
                                        self.preset_idx = 0;
                                        let seed = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.subsec_nanos())
                                            .unwrap_or(1);
                                        let mut r = seed;
                                        let mut rr = |min: f32, max: f32| {
                                            r ^= r << 13; r ^= r >> 17; r ^= r << 5;
                                            let u = (r >> 8) as f32 * (1.0 / 16_777_216.0);
                                            min + u * (max - min)
                                        };
                                        sp(&self.params.tone, rr(100.0, 420.0));
                                        sp(&self.params.ring, rr(0.5, 0.995));
                                        sp(&self.params.damping, rr(1500.0, 8000.0));
                                        sp(&self.params.snare, rr(0.2, 0.9));
                                        sp(&self.params.snap, rr(0.4, 1.0));
                                        sp(&self.params.sweep, rr(0.05, 0.6));
                                        sp(&self.params.glue, rr(0.15, 0.7));
                                        sp(&self.params.verb, rr(0.05, 0.5));
                                    }
                                });
                                ui.add_space(6.0);

                                let p = |ui: &mut egui::Ui, name: &str, param: &FloatParam| {
                                    ui.label(name);
                                    ui.add(widgets::ParamSlider::for_param(param, &setter));
                                };
                                p(ui, "Snap — pałeczka (atak)", &self.params.snap);
                                p(ui, "Tone — naciąg (wysokość)", &self.params.tone);
                                p(ui, "Sweep — ugięcie naciągu", &self.params.sweep);
                                p(ui, "Damping — tłumik (jasność)", &self.params.damping);
                                p(ui, "Ring — korpus (wybrzmienie)", &self.params.ring);
                                p(ui, "Snare — sprężyny", &self.params.snare);
                                p(ui, "Glue — kompresja", &self.params.glue);
                                p(ui, "Verb — pokój (reverb)", &self.params.verb);
                            });
                        });
                    });
            });

        // modal „O pluginie"
        if self.about_open {
            egui::Window::new("O pluginie")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.label(egui::RichText::new("Werbel").strong().size(18.0));
                    ui.label("VST3/CLAP — syntezator werbla (feedback synthesis)");
                    ui.add_space(6.0);
                    ui.label(format!("Wersja: {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Build: release · Rust / nice-plug 0.4");
                    ui.label("Autor: gwolaniuk");
                    ui.label("E-mail: gwolaniuk@users.noreply.github.com");
                    ui.label("Rok produkcji: 2026");
                    ui.add_space(6.0);
                    ui.hyperlink_to("github.com/gwolaniuk/werbel", "https://github.com/gwolaniuk/werbel");
                    ui.add_space(6.0);
                    if ui.button("Zamknij").clicked() {
                        self.about_open = false;
                    }
                });
        }
    }
}