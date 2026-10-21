//! Edytor egui — graficzny werbel z parametrami przypisanymi elementom konstrukcyjnym.

use crate::WerbelParams;
use nice_plug::{context::gui::GuiContext, prelude::*};
use nice_plug_egui::{NiceEguiApp, resizable_window::ResizableWindow, widgets};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Presety (wzorowane na charakterach werbli z dema: Drum Center of Portsmouth)
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
    Preset { name: "Studio Maple (ciepły)",   tone: 200.0, ring: 0.80,  damping: 2800.0, snare: 0.5,  snap: 0.65, sweep: 0.25, glue: 0.40, verb: 0.25 },
    Preset { name: "Tight Piccolo Crack",     tone: 360.0, ring: 0.55,  damping: 6000.0, snare: 0.70, snap: 0.90, sweep: 0.20, glue: 0.50, verb: 0.20 },
    Preset { name: "Fat & Deep",              tone: 130.0, ring: 0.92,  damping: 1500.0, snare: 0.40, snap: 0.60, sweep: 0.40, glue: 0.50, verb: 0.30 },
    Preset { name: "Brass Supraphonic (ring)",tone: 220.0, ring: 0.95,  damping: 8000.0, snare: 0.55, snap: 0.75, sweep: 0.20, glue: 0.30, verb: 0.25 },
    Preset { name: "Dry Vintage (muffled)",   tone: 210.0, ring: 0.60,  damping: 1200.0, snare: 0.35, snap: 0.70, sweep: 0.15, glue: 0.55, verb: 0.15 },
    Preset { name: "Funk Snap",               tone: 250.0, ring: 0.70,  damping: 4500.0, snare: 0.65, snap: 0.85, sweep: 0.30, glue: 0.45, verb: 0.20 },
    Preset { name: "Open Ringy (bez tłumika)",tone: 190.0, ring: 0.985, damping: 9000.0, snare: 0.60, snap: 0.70, sweep: 0.20, glue: 0.30, verb: 0.35 },
    Preset { name: "Electro Snip",            tone: 300.0, ring: 0.50,  damping: 7000.0, snare: 0.80, snap: 0.95, sweep: 0.10, glue: 0.60, verb: 0.10 },
];

/// Stan istniejący tylko, gdy okno edytora jest otwarte.
struct OpenState {
    nice_gui_ctx: GuiContext,
}

pub struct WerbelEditor {
    open_state: Option<OpenState>,
    params: Arc<WerbelParams>,
    preset_idx: usize,
}

impl WerbelEditor {
    pub fn new(params: Arc<WerbelParams>) -> Self {
        Self {
            open_state: None,
            params,
            preset_idx: 0,
        }
    }
}

// Kolory lustrzane do referencyjnej ilustracji: kremowe tło + ciemny rysunek.
const BG: egui::Color32 = egui::Color32::from_rgb(0xf4, 0xef, 0xe4);
const DARK: egui::Color32 = egui::Color32::from_rgb(0x3a, 0x36, 0x30);
const SILVER: egui::Color32 = egui::Color32::from_rgb(0xc2, 0xc6, 0xcb);
const HEAD: egui::Color32 = egui::Color32::from_rgb(0xfc, 0xfb, 0xf6);
const WARM: egui::Color32 = egui::Color32::from_rgb(0xb8, 0x8a, 0x5c);

/// Rysuje werbel (widok z boku) na zadanym obszarze.
fn paint_snare(painter: &egui::Painter, rect: egui::Rect, font: egui::FontId) {
    let w = rect.width();
    let cx = rect.center().x;
    let drum_w = w * 0.40;

    // --- obręcze / naciągi / korpus ---
    let rim_h = 13.0;
    let top_rim = egui::Rect::from_min_size(
        egui::pos2(cx - drum_w / 2.0 - 8.0, rect.top() + 34.0),
        egui::vec2(drum_w + 16.0, rim_h),
    );
    let batter = egui::Rect::from_min_size(
        egui::pos2(cx - drum_w / 2.0, top_rim.bottom() + 3.0),
        egui::vec2(drum_w, 9.0),
    );
    let shell = egui::Rect::from_min_size(
        egui::pos2(cx - drum_w / 2.0, batter.bottom() + 2.0),
        egui::vec2(drum_w, 250.0),
    );
    let reson = egui::Rect::from_min_size(
        egui::pos2(cx - drum_w / 2.0, shell.bottom() + 2.0),
        egui::vec2(drum_w, 9.0),
    );
    let bottom_rim = egui::Rect::from_min_size(
        egui::pos2(cx - drum_w / 2.0 - 8.0, reson.bottom() + 3.0),
        egui::vec2(drum_w + 16.0, rim_h),
    );

    painter.rect_filled(top_rim, 3.0, DARK);
    painter.rect_filled(bottom_rim, 3.0, DARK);

    painter.rect_filled(batter, 0.0, HEAD);
    painter.rect_stroke(batter, 0.0, egui::Stroke::new(1.5, DARK), egui::StrokeKind::Inside);
    painter.rect_filled(shell, 8.0, SILVER);
    painter.rect_stroke(shell, 8.0, egui::Stroke::new(1.5, DARK), egui::StrokeKind::Inside);
    painter.rect_filled(reson, 0.0, HEAD);
    painter.rect_stroke(reson, 0.0, egui::Stroke::new(1.5, DARK), egui::StrokeKind::Inside);

    // otwór wentylacyjny
    painter.circle_filled(egui::pos2(cx + drum_w * 0.28, shell.top() + 22.0), 5.0, DARK);

    // --- śruby naciągowe + uszy (lugs) ---
    for i in 0..6 {
        let x = cx - drum_w / 2.0 + 12.0 + i as f32 * (drum_w - 24.0) / 5.0;
        painter.line_segment(
            [egui::pos2(x, top_rim.center().y - 9.0), egui::pos2(x, bottom_rim.center().y + 9.0)],
            egui::Stroke::new(2.0, DARK),
        );
        painter.rect_filled(
            egui::Rect::from_center_size(egui::pos2(x, shell.center().y), egui::vec2(8.0, 26.0)),
            2.0,
            WARM,
        );
    }

    // --- sprężyny (snare wires) ---
    let sw = drum_w + 26.0;
    for k in 0..4 {
        let sy = bottom_rim.bottom() + 8.0 + k as f32 * 9.0;
        let sxx = cx - sw / 2.0;
        let exx = cx + sw / 2.0;
        painter.line_segment([egui::pos2(sxx, sy), egui::pos2(exx, sy)], egui::Stroke::new(1.2, DARK));
        painter.line_segment([egui::pos2(sxx, sy), egui::pos2(exx, sy + 6.0)], egui::Stroke::new(1.0, DARK));
    }

    // --- throw-off ---
    let lx = cx - drum_w / 2.0 - 22.0;
    painter.line_segment(
        [egui::pos2(lx, shell.bottom() + 4.0), egui::pos2(lx, shell.bottom() + 40.0)],
        egui::Stroke::new(3.0, DARK),
    );
    painter.circle_filled(egui::pos2(lx, shell.bottom() + 40.0), 4.0, WARM);

    // --- pałeczka ---
    let tip = egui::pos2(cx + drum_w * 0.18, top_rim.top() - 6.0);
    let tail = egui::pos2(cx + drum_w * 0.62, top_rim.top() - 110.0);
    painter.line_segment([tail, tip], egui::Stroke::new(4.0, WARM));
    painter.circle_filled(tip, 6.0, WARM);

    // --- etykiety ---
    let rx = rect.right() - 150.0;
    let label = |p: egui::Pos2, t: &str| {
        painter.text(p, egui::Align2::LEFT_CENTER, t, font.clone(), DARK);
    };
    label(egui::pos2(rx, batter.center().y), "naciąg · TONE");
    label(egui::pos2(rx, shell.top() + 40.0), "korpus · RING");
    label(egui::pos2(rx, shell.center().y), "otwór went.");
    label(egui::pos2(rx, shell.bottom() - 40.0), "tłumik · DAMPING");
    label(egui::pos2(rx, bottom_rim.bottom() + 20.0), "sprężyny · SNARE");
    label(egui::pos2(rx, bottom_rim.bottom() + 46.0), "throw-off");
    label(egui::pos2(rx - 20.0, top_rim.top() - 95.0), "pałeczka · SNAP");
}

impl NiceEguiApp for WerbelEditor {
    fn build(
        &mut self,
        _egui_ctx: egui::Context,
        nice_gui_ctx: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        self.open_state = Some(OpenState { nice_gui_ctx });
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.open_state = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(state) = self.open_state.as_mut() else {
            return;
        };
        let setter = state.nice_gui_ctx.param_setter();

        ResizableWindow::new("werbel-window")
            .min_size(egui::Vec2::new(620.0, 360.0))
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // ===== LEWA: rysunek werbla =====
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(400.0, 460.0),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            let font = egui::FontId::proportional(11.0);
                            paint_snare(&painter, rect, font);

                            ui.separator();

                            // ===== PRAWA: presety + suwaki =====
                            let w = ui.available_width() - 4.0;
                            ui.vertical(|ui| {
                                ui.set_width(w);
                                ui.heading("Werbel");
                                ui.small("feedback synthesis · gwolaniuk");
                                ui.add_space(4.0);

                                // --- lista presetów ---
                                let cur_name = if self.preset_idx == 0 {
                                    "— Custom —"
                                } else {
                                    PRESETS[self.preset_idx - 1].name
                                };
                                egui::ComboBox::from_label("Preset")
                                    .selected_text(cur_name)
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(self.preset_idx == 0, "— Custom —").clicked() {
                                            self.preset_idx = 0;
                                        }
                                        for (i, p) in PRESETS.iter().enumerate() {
                                            if ui.selectable_label(self.preset_idx == i + 1, p.name).clicked() {
                                                self.preset_idx = i + 1;
                                                let sp = |param: &FloatParam, v: f32| {
                                                    setter.begin_set_parameter(param);
                                                    setter.set_parameter(param, v);
                                                    setter.end_set_parameter(param);
                                                };
                                                sp(&self.params.tone, p.tone);
                                                sp(&self.params.ring, p.ring);
                                                sp(&self.params.damping, p.damping);
                                                sp(&self.params.snare, p.snare);
                                                sp(&self.params.snap, p.snap);
                                                sp(&self.params.sweep, p.sweep);
                                                sp(&self.params.glue, p.glue);
                                                sp(&self.params.verb, p.verb);
                                            }
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
                                p(ui, "Damping — tłumik", &self.params.damping);
                                p(ui, "Ring — korpus (wybrzmienie)", &self.params.ring);
                                p(ui, "Snare — sprężyny", &self.params.snare);
                                p(ui, "Glue — kompresja", &self.params.glue);
                                p(ui, "Verb — pokój (reverb)", &self.params.verb);
                            });
                        });
                    });
            });
    }
}