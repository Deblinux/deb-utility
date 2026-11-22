use crate::i18n::{t, tr};
use crate::Page;
use eframe::egui::{self, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, TextStyle};
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

pub const ACCENT: Color32 = Color32::from_rgb(0, 196, 168);
pub const TEXT: Color32 = Color32::from_rgb(238, 242, 248);
pub const TEXT_DIM: Color32 = Color32::from_rgb(172, 183, 199);
pub const BTN_BLUE: Color32 = Color32::from_rgb(37, 99, 235);
pub const BTN_BLUE_HOVER: Color32 = Color32::from_rgb(59, 130, 246);
pub const BTN_BLUE_ACTIVE: Color32 = Color32::from_rgb(29, 78, 216);
pub const BAR_BLUE: Color32 = Color32::from_rgb(37, 99, 235);
pub const GOOD: Color32 = Color32::from_rgb(70, 195, 105);
pub const WARN: Color32 = Color32::from_rgb(244, 168, 40);
pub const BAD: Color32 = Color32::from_rgb(232, 72, 72);

pub const HIST_LEN: usize = 60;

// Funzioni di sfondo dinamiche in base al tema
pub fn sidebar_bg(ctx: &egui::Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        Color32::from_rgb(16, 21, 29)
    } else {
        Color32::from_rgb(230, 234, 240)
    }
}

pub fn content_bg(ctx: &egui::Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        Color32::from_rgb(25, 31, 42)
    } else {
        Color32::from_rgb(245, 247, 250)
    }
}

pub fn card_bg(ctx: &egui::Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        Color32::from_rgb(36, 44, 58)
    } else {
        Color32::from_rgb(255, 255, 255)
    }
}

pub fn apply_theme(ctx: &egui::Context, dark_mode: bool) {
    let mut v = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = content_bg(ctx);
    v.window_fill = card_bg(ctx);
    v.extreme_bg_color = if dark_mode {
        Color32::from_rgb(14, 28, 58)
    } else {
        Color32::from_rgb(230, 235, 240)
    };
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, if dark_mode { Color32::WHITE } else { Color32::BLACK });
    v.window_rounding = egui::Rounding::same(12.0);

    let text_color = if dark_mode { TEXT } else { Color32::from_rgb(30, 40, 50) };
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text_color);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, if dark_mode { Color32::WHITE } else { Color32::BLACK });
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, if dark_mode { Color32::WHITE } else { Color32::BLACK });
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, if dark_mode { Color32::WHITE } else { Color32::BLACK });

    let r = egui::Rounding::same(8.0);
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    let normal = BTN_BLUE;
    let hover = BTN_BLUE_HOVER;
    v.widgets.inactive.bg_fill = normal;
    v.widgets.inactive.weak_bg_fill = normal;
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.active.bg_fill = BTN_BLUE_ACTIVE;
    v.widgets.active.weak_bg_fill = BTN_BLUE_ACTIVE;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(27.0)),
        (TextStyle::Body, FontId::proportional(17.0)),
        (TextStyle::Button, FontId::proportional(17.0)),
        (TextStyle::Small, FontId::proportional(14.0)),
        (TextStyle::Monospace, FontId::monospace(15.0)),
    ]
    .into();
    style.spacing.item_spacing = vec2(10.0, 9.0);
    style.spacing.button_padding = vec2(14.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    ctx.set_style(style);
}

pub fn title(ui: &mut egui::Ui, text: &'static str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(6.0, 34.0), Sense::hover());
        ui.painter().rect_filled(r, 3.0, ACCENT);
        let title_col = if ui.visuals().dark_mode { Color32::WHITE } else { Color32::from_rgb(20, 30, 40) };
        ui.label(egui::RichText::new(t(text)).size(30.0).strong().color(title_col));
    });
    ui.add_space(12.0);
}

pub fn sub_tabs(ui: &mut egui::Ui, sub: &mut usize, names: &[&'static str]) {
    ui.horizontal_wrapped(|ui| {
        for (i, n) in names.iter().enumerate() {
            ui.selectable_value(sub, i, egui::RichText::new(t(*n)).size(17.0));
        }
    });
    ui.separator();
}

pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let dark_mode = ui.visuals().dark_mode;
    let bg = card_bg(ui.ctx());
    let stroke_color = if dark_mode { Color32::from_white_alpha(16) } else { Color32::from_black_alpha(20) };
    egui::Frame::default()
        .fill(bg)
        .rounding(12.0)
        .inner_margin(egui::Margin::same(16.0))
        .stroke(Stroke::new(1.0_f32, stroke_color))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn hint(ui: &mut egui::Ui, text: &'static str) {
    let dark_mode = ui.visuals().dark_mode;
    let fill = if dark_mode { Color32::from_rgb(20, 52, 104) } else { Color32::from_rgb(215, 230, 250) };
    let text_col = if dark_mode { Color32::WHITE } else { Color32::from_rgb(20, 40, 70) };
    egui::Frame::default()
        .fill(fill)
        .rounding(10.0)
        .inner_margin(egui::Margin::same(12.0))
        .stroke(Stroke::new(1.0_f32, BTN_BLUE_HOVER))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let w = ui.available_width();
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 12.0, BTN_BLUE_HOVER);
                ui.painter().text(r.center(), Align2::CENTER_CENTER, "i", FontId::proportional(17.0), Color32::WHITE);
                ui.vertical(|ui| {
                    ui.set_max_width((w - 56.0).max(200.0));
                    ui.label(egui::RichText::new(t(text)).size(16.0).color(text_col));
                });
            });
        });
    ui.add_space(8.0);
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(text).strong().size(20.0).color(Color32::WHITE))
            .fill(Color32::from_rgb(0, 150, 136))
            .rounding(12.0)
            .min_size(vec2(235.0, 50.0)),
    )
}

pub fn bar(frac: f32) -> egui::ProgressBar {
    egui::ProgressBar::new(frac).fill(BAR_BLUE).desired_height(26.0)
}

pub fn heat_color(celsius: f32) -> Color32 {
    if celsius < 65.0 {
        BAR_BLUE
    } else if celsius < 80.0 {
        Color32::from_rgb(214, 120, 0)
    } else {
        Color32::from_rgb(200, 40, 40)
    }
}

pub fn level_color(f: f32) -> Color32 {
    if f < 0.6 {
        GOOD
    } else if f < 0.85 {
        WARN
    } else {
        BAD
    }
}

fn page_color(p: Page) -> Color32 {
    let (r, g, b) = match p {
        Page::Dashboard => (0, 168, 150),
        Page::Cleaner => (46, 170, 100),
        Page::SafeOptimize => (50, 180, 140),
        Page::Fixer => (232, 140, 40),
        Page::Info => (60, 130, 230),
        Page::Disk => (140, 100, 220),
        Page::Procs => (60, 160, 200),
        Page::Apps => (200, 100, 160),
        Page::Zram => (226, 178, 40),
        Page::Temps => (225, 90, 70),
        Page::Network => (50, 150, 220),
        Page::Kernel => (110, 124, 150),
        Page::Repos => (176, 120, 60),
        Page::Startup => (80, 180, 120),
        Page::Perms => (150, 100, 200),
        Page::Firewall => (205, 60, 70),
        Page::Backup => (60, 140, 200),
        Page::Shredder => (190, 70, 70),
        Page::Log => (110, 124, 150),
        Page::Settings => (120, 130, 150),
        Page::Guide => (0, 150, 200),
        Page::Updates => (30, 160, 120),
        Page::Containers => (40, 140, 200),
        Page::Timers => (140, 100, 200),
        Page::Synaptic => (100, 160, 220),
        Page::About => (100, 160, 220),
        Page::History => (80, 140, 200),
        Page::AdvancedCleaner => (100, 200, 150),
        Page::Duplicates => (100, 180, 220),
        Page::Services => (120, 180, 240),
        Page::Snaps => (120, 180, 240),
        Page::Report => (0, 168, 150),
        Page::DiskAnalyzer => (53, 162, 159),
        Page::Boot => (53, 162, 159),
        Page::AppImages => (114, 9, 183),
        Page::MenuEditor => (140, 100, 220),
        Page::SecurityHardening => (50, 150, 220),
        Page::HardwareHealth => (70, 180, 120),
        Page::AppLifecycle => (220, 80, 100),
        Page::Themes => (180, 100, 220),
        Page::SystemEditor => (53, 162, 159),
        Page::TelemetryGuard => (46, 139, 87),
        Page::Mounts => (50, 150, 220),
    };
    Color32::from_rgb(r, g, b)
}

pub fn page_icon(p: &egui::Painter, c: Pos2, size: f32, page: Page) {
    let tile = Rect::from_center_size(c, vec2(size, size));
    p.rect_filled(tile, size * 0.28, page_color(page));
    let s = size * 0.28;
    let w = Color32::WHITE;
    let st = Stroke::new((size * 0.075).max(1.5), w);
    let th = Stroke::new((size * 0.055).max(1.2), w);
    let pt = |x: f32, y: f32| Pos2::new(c.x + x * s, c.y + y * s);
    let line = |a: Pos2, b: Pos2, stroke: Stroke| {
        p.line_segment([a, b], stroke);
    };
    let poly = |pts: Vec<Pos2>, stroke: Stroke| {
        p.add(Shape::line(pts, stroke));
    };
    let rr = |a: Pos2, b: Pos2| {
        p.rect_stroke(Rect::from_min_max(a, b), 2.0, st);
    };
    match page {
        Page::Dashboard => {
            for (i, h) in [0.5f32, 1.0, 0.75].iter().enumerate() {
                let x = -0.7 + i as f32 * 0.7;
                p.rect_filled(Rect::from_min_max(pt(x - 0.22, 1.0 - 2.0 * h), pt(x + 0.22, 1.0)), 1.5, w);
            }
        }
        Page::Cleaner => {
            line(pt(0.0, -1.0), pt(0.0, 1.0), st);
            line(pt(-1.0, 0.0), pt(1.0, 0.0), st);
            line(pt(-0.55, -0.55), pt(0.55, 0.55), th);
            line(pt(-0.55, 0.55), pt(0.55, -0.55), th);
        }
        Page::SafeOptimize => {
            p.circle_stroke(c, s, st);
            line(pt(0.0, -0.55), pt(0.0, 0.55), st);
            line(pt(-0.55, 0.0), pt(0.55, 0.0), st);
        }
        Page::Fixer => {
            line(pt(-0.8, 0.8), pt(0.2, -0.2), Stroke::new((size * 0.12).max(2.0), w));
            p.circle_stroke(pt(0.45, -0.45), s * 0.5, st);
        }
        Page::Info => {
            rr(pt(-1.0, -0.8), pt(1.0, 0.4));
            line(pt(0.0, 0.4), pt(0.0, 0.85), st);
            line(pt(-0.5, 0.85), pt(0.5, 0.85), st);
        }
        Page::Disk => {
            p.circle_stroke(c, s, st);
            p.circle_filled(c, s * 0.25, w);
        }
        Page::DiskAnalyzer => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "📊", FontId::proportional(size * 0.55), w);
        }
        Page::SecurityHardening => {
            // Disegna qui l'icona personalizzata per il modulo di sicurezza (es. uno scudo o linee guida)
            // Oppure lascia temporaneamente todo!() se vuoi implementare il disegno grafico in un secondo momento
            line(pt(0.0, -0.6), pt(0.6, 0.6), st);
            line(pt(0.0, -0.6), pt(-0.6, 0.6), st);
        }
        Page::Boot => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "⏱", FontId::proportional(size * 0.55), w);
        }
        Page::Procs => {
            p.circle_stroke(c, s * 0.55, st);
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0;
                line(pt(0.75 * a.cos(), 0.75 * a.sin()), pt(a.cos(), a.sin()), st);
            }
        }
        Page::Apps => {
            rr(pt(-0.9, -0.4), pt(0.9, 0.95));
            rr(pt(-1.0, -0.95), pt(1.0, -0.4));
        }
        Page::Zram => {
            rr(pt(-0.8, -0.5), pt(0.8, 0.5));
            for i in 0..4 {
                let x = -0.55 + i as f32 * 0.37;
                line(pt(x, -0.5), pt(x, -0.95), th);
                line(pt(x, 0.5), pt(x, 0.95), th);
            }
        }
        Page::Temps => {
            line(pt(0.0, -0.95), pt(0.0, 0.3), Stroke::new((size * 0.11).max(2.0), w));
            p.circle_filled(pt(0.0, 0.6), s * 0.5, w);
        }
        Page::Network => {
            p.circle_stroke(c, s, st);
            line(pt(-1.0, 0.0), pt(1.0, 0.0), th);
            line(pt(0.0, -1.0), pt(0.0, 1.0), th);
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = i as f32 / 24.0 * TAU;
                    pt(0.5 * a.cos(), a.sin())
                })
                .collect();
            poly(pts, th);
        }
        Page::Snaps => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "📦", FontId::proportional(size * 0.55), w);
        }
        Page::Report => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "📋", FontId::proportional(size * 0.55), w);
        }
        Page::Kernel => {
            poly(vec![pt(-0.9, -0.5), pt(-0.2, 0.0), pt(-0.9, 0.5)], st);
            line(pt(0.1, 0.6), pt(0.9, 0.6), st);
        }
        Page::Containers | Page::Timers | Page::Synaptic | Page::History | Page::AdvancedCleaner => {
            p.circle_stroke(c, s, st);
        }
        Page::Repos => {
            for y in [-0.85f32, -0.2, 0.45] {
                rr(pt(-0.9, y), pt(0.9, y + 0.45));
            }
        }
        Page::Startup => {
            p.add(Shape::convex_polygon(
                vec![pt(-0.6, -0.9), pt(0.9, 0.0), pt(-0.6, 0.9)],
                w,
                Stroke::NONE,
            ));
        }
        Page::Perms => {
            p.circle_stroke(pt(0.0, -0.45), s * 0.45, st);
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = PI + i as f32 / 16.0 * PI;
                    pt(0.9 * a.cos(), 0.95 + 0.9 * a.sin())
                })
                .collect();
            poly(pts, st);
        }
        Page::Firewall => {
            p.add(Shape::closed_line(
                vec![pt(-0.8, -0.8), pt(0.8, -0.8), pt(0.8, 0.1), pt(0.0, 0.95), pt(-0.8, 0.1)],
                st,
            ));
        }
        Page::Backup => {
            line(pt(0.0, -0.9), pt(0.0, 0.3), st);
            poly(vec![pt(-0.5, -0.2), pt(0.0, 0.35), pt(0.5, -0.2)], st);
            poly(vec![pt(-0.9, 0.4), pt(-0.9, 0.9), pt(0.9, 0.9), pt(0.9, 0.4)], st);
        }
        Page::Shredder => {
            line(pt(-0.9, -0.6), pt(0.9, -0.6), st);
            poly(vec![pt(-0.3, -0.6), pt(-0.3, -0.9), pt(0.3, -0.9), pt(0.3, -0.6)], th);
            rr(pt(-0.7, -0.6), pt(0.7, 0.95));
            line(pt(-0.25, -0.2), pt(-0.25, 0.6), th);
            line(pt(0.25, -0.2), pt(0.25, 0.6), th);
        }
        Page::Log => {
            rr(pt(-0.7, -0.95), pt(0.7, 0.95));
            for y in [-0.4f32, 0.0, 0.4] {
                line(pt(-0.4, y), pt(0.4, y), th);
            }
        }
        Page::Settings => {
            for (y, k) in [(-0.6f32, -0.3f32), (0.0, 0.4), (0.6, -0.1)] {
                line(pt(-0.9, y), pt(0.9, y), th);
                p.circle_filled(pt(k, y), s * 0.22, w);
            }
        }
        Page::Updates => {
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = 0.3 * PI + i as f32 / 20.0 * 1.4 * PI;
                    pt(0.9 * a.cos(), 0.9 * a.sin())
                })
                .collect();
            poly(pts, st);
            p.add(Shape::convex_polygon(vec![pt(0.894, -0.465), pt(0.736, -1.013), pt(0.324, -0.447)], w, Stroke::NONE));
        }
        Page::Guide => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "?", FontId::proportional(size * 0.55), w);
        }
        Page::About => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "i", FontId::proportional(size * 0.55), w);
        }
        Page::Duplicates => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "📁", FontId::proportional(size * 0.55), w);
        }
        Page::Services => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "⚙", FontId::proportional(size * 0.55), w);
        }
        Page::AppImages => {
          p.circle_stroke(c, s, st);
          p.text(c, Align2::CENTER_CENTER, "📦", FontId::proportional(size * 0.55), w);
        }
        Page::MenuEditor => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "📋", FontId::proportional(size * 0.55), w);
        }
        Page::HardwareHealth => { 
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "🔋", FontId::proportional(size * 0.55), w);
        }
        Page::AppLifecycle => { 
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "✂", FontId::proportional(size * 0.55), w);
        }
        Page::Themes => {
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "🎨", FontId::proportional(size * 0.55), w);
        }
        Page::SystemEditor => { 
            p.circle_stroke(c, s, st);
            p.text(c, Align2::CENTER_CENTER, "🛠", FontId::proportional(size * 0.55), w);
        }
        Page::TelemetryGuard => {
         p.circle_stroke(c, s, st);
         p.text(c, Align2::CENTER_CENTER, "🛡", FontId::proportional(size * 0.55), w);
        }
        Page::Mounts => {
            rr(pt(-0.9, -0.55), pt(0.9, 0.55));
            line(pt(-0.55, 0.2), pt(0.55, 0.2), th);
        }
    }
}

pub fn sidebar_item(ui: &mut egui::Ui, page: Page, name: &str, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    let dark_mode = ui.visuals().dark_mode;
    if selected {
        let sel_bg = if dark_mode { Color32::from_rgb(36, 52, 66) } else { Color32::from_rgb(210, 220, 235) };
        p.rect_filled(rect, 10.0, sel_bg);
        p.rect_filled(Rect::from_min_size(rect.left_top() + vec2(0.0, 8.0), vec2(4.0, 26.0)), 2.0, ACCENT);
    } else if resp.hovered() {
        let hover_bg = if dark_mode { Color32::from_white_alpha(12) } else { Color32::from_black_alpha(15) };
        p.rect_filled(rect, 10.0, hover_bg);
    }
    page_icon(p, Pos2::new(rect.left() + 28.0, rect.center().y), 30.0, page);
    let text_color = if dark_mode {
        if selected { Color32::WHITE } else { TEXT }
    } else {
        if selected { Color32::BLACK } else { Color32::from_rgb(40, 50, 60) }
    };
    p.text(
        Pos2::new(rect.left() + 54.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(17.0),
        text_color,
    );
    resp.clicked()
}

pub fn tile(ui: &mut egui::Ui, page: Page, title: &str, sub: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(290.0, 98.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    let dark_mode = ui.visuals().dark_mode;
    let fill = if resp.hovered() {
        if dark_mode { Color32::from_rgb(47, 58, 77) } else { Color32::from_rgb(230, 235, 242) }
    } else {
        card_bg(ui.ctx())
    };
    p.rect_filled(rect, 12.0, fill);
    let stroke_color = if dark_mode { Color32::from_white_alpha(16) } else { Color32::from_black_alpha(20) };
    p.rect_stroke(rect, 12.0, Stroke::new(1.0_f32, stroke_color));
    page_icon(p, Pos2::new(rect.left() + 50.0, rect.center().y), 60.0, page);
    let title_color = if dark_mode { Color32::WHITE } else { Color32::from_rgb(30, 40, 50) };
    p.text(
        Pos2::new(rect.left() + 96.0, rect.center().y - 13.0),
        Align2::LEFT_CENTER,
        tr(title),
        FontId::proportional(20.0),
        title_color,
    );
    p.text(
        Pos2::new(rect.left() + 96.0, rect.center().y + 14.0),
        Align2::LEFT_CENTER,
        tr(sub),
        FontId::proportional(15.0),
        TEXT_DIM,
    );
    resp.clicked()
}

pub fn dash_tile(ui: &mut egui::Ui, page: Page, title: &str, width: f32, height: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(width.max(150.0), height.max(120.0)), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    let dark_mode = ui.visuals().dark_mode;

    let fill = if resp.hovered() {
        if dark_mode { Color32::from_rgb(48, 60, 80) } else { Color32::from_rgb(228, 234, 242) }
    } else {
        card_bg(ui.ctx())
    };
    p.rect_filled(rect, 14.0, fill);

    let stroke_color = if dark_mode { Color32::from_white_alpha(20) } else { Color32::from_black_alpha(24) };
    p.rect_stroke(rect, 14.0, Stroke::new(1.0_f32, stroke_color));

    let title_color = if dark_mode { Color32::WHITE } else { Color32::from_rgb(25, 35, 45) };
    p.text(
        Pos2::new(rect.center().x, rect.top() + 26.0),
        Align2::CENTER_CENTER,
        tr(title),
        FontId::proportional(17.0),
        title_color,
    );

    let icon_center = Pos2::new(rect.center().x, rect.top() + height * 0.58);
    let ring_r = (height * 0.25).clamp(26.0, 40.0);
    let ring_bg = if dark_mode { Color32::from_rgb(26, 33, 45) } else { Color32::from_rgb(235, 240, 246) };
    p.circle_filled(icon_center, ring_r, ring_bg);
    p.circle_stroke(icon_center, ring_r, Stroke::new(1.5_f32, stroke_color));

    page_icon(p, icon_center, ring_r * 1.15, page);
    resp.clicked()
}

pub fn cpu_dial(ui: &mut egui::Ui, frac: f32, label: &str) {
    let frac = frac.clamp(0.0, 1.0);
    let size = 132.0;
    let (rect, _) = ui.allocate_exact_size(vec2(size + 16.0, size + 12.0), Sense::hover());
    let p = ui.painter_at(rect);
    let c = rect.center();
    let r = size * 0.42;
    let start = 0.75 * PI;
    let sweep = 1.5 * PI;

    let bg_ring = if ui.visuals().dark_mode { Color32::from_white_alpha(26) } else { Color32::from_black_alpha(22) };
    p.add(Shape::line(arc(c, r, start, start + sweep), Stroke::new(11.0_f32, bg_ring)));
    if frac > 0.005 {
        p.add(Shape::line(
            arc(c, r, start, start + sweep * frac),
            Stroke::new(11.0_f32, level_color(frac)),
        ));
    }

    let text_col = if ui.visuals().dark_mode { Color32::WHITE } else { Color32::from_rgb(25, 35, 45) };
    p.text(
        Pos2::new(c.x, c.y - 6.0),
        Align2::CENTER_CENTER,
        format!("{:.0}%", frac * 100.0),
        FontId::proportional(24.0),
        text_col,
    );
    p.text(
        Pos2::new(c.x, c.y + 18.0),
        Align2::CENTER_CENTER,
        tr(label),
        FontId::proportional(13.0),
        TEXT_DIM,
    );
}

fn arc(c: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let n = 48;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

pub fn gauge(ui: &mut egui::Ui, frac: f32, title: &str, sub: &str) {
    let frac = frac.clamp(0.0, 1.0);
    let size = 160.0;
    let (rect, _) = ui.allocate_exact_size(vec2(size + 24.0, size + 52.0), Sense::hover());
    let p = ui.painter_at(rect);
    let c = Pos2::new(rect.center().x, rect.top() + size / 2.0 + 6.0);
    let r = size * 0.40;
    let start = 0.75 * PI;
    let sweep = 1.5 * PI;
    p.add(Shape::line(arc(c, r, start, start + sweep), Stroke::new(13.0_f32, Color32::from_white_alpha(28))));
    if frac > 0.005 {
        p.add(Shape::line(
            arc(c, r, start, start + sweep * frac),
            Stroke::new(13.0_f32, level_color(frac)),
        ));
    }
    let text_col = if ui.visuals().dark_mode { Color32::WHITE } else { Color32::from_rgb(30, 40, 50) };
    p.text(c, Align2::CENTER_CENTER, format!("{:.0}%", frac * 100.0), FontId::proportional(30.0), text_col);
    p.text(Pos2::new(c.x, c.y + r + 26.0), Align2::CENTER_CENTER, tr(title), FontId::proportional(18.0), text_col);
    p.text(Pos2::new(c.x, c.y + r + 46.0), Align2::CENTER_CENTER, sub, FontId::proportional(14.0), TEXT_DIM);
}

pub fn sparkline(ui: &mut egui::Ui, data: &VecDeque<f32>, color: Color32, height: f32) {
    let w = ui.available_width().max(100.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter_at(rect);
    let dark_mode = ui.visuals().dark_mode;
    let bg = if dark_mode { Color32::from_black_alpha(70) } else { Color32::from_black_alpha(20) };
    p.rect_filled(rect, 6.0, bg);
    let n = data.len();
    if n < 2 {
        return;
    }
    let pts: Vec<Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = rect.left() + rect.width() * (i + HIST_LEN - n) as f32 / (HIST_LEN - 1) as f32;
            Pos2::new(x, rect.bottom() - 4.0 - (rect.height() - 8.0) * v.clamp(0.0, 1.0))
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(2.5_f32, color)));
}

pub fn led(ui: &mut egui::Ui, state: Option<bool>, text: &str) {
    let col = match state {
        Some(true) => GOOD,
        Some(false) => BAD,
        None => Color32::GRAY,
    };
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 9.0, col);
        ui.label(text);
    });
}

pub fn output_box(ui: &mut egui::Ui, text: &str, height: f32) {
    let dark_mode = ui.visuals().dark_mode;
    let bg = if dark_mode { Color32::from_rgb(17, 22, 30) } else { Color32::from_rgb(235, 240, 245) };
    egui::Frame::default()
        .fill(bg)
        .inner_margin(egui::Margin::same(10.0))
        .rounding(8.0)
        .show(ui, |ui| {
            egui::ScrollArea::both().max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                let shown = if text.trim().is_empty() { t("(nessun output)") } else { text };
                ui.label(egui::RichText::new(shown).monospace());
            });
        });
}

pub fn output_box_follow(ui: &mut egui::Ui, text: &str, height: f32) {
    let dark_mode = ui.visuals().dark_mode;
    let bg = if dark_mode { Color32::from_rgb(17, 22, 30) } else { Color32::from_rgb(235, 240, 245) };
    egui::Frame::default()
        .fill(bg)
        .inner_margin(egui::Margin::same(10.0))
        .rounding(8.0)
        .show(ui, |ui| {
            egui::ScrollArea::both().max_height(height).auto_shrink([false, true]).stick_to_bottom(true).show(ui, |ui| {
                let t = if text.trim().is_empty() { "(nessun output)" } else { text };
                ui.label(egui::RichText::new(t).monospace());
            });
        });
}

pub fn kv(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.label(egui::RichText::new(tr(k)).strong());
    ui.label(v);
    ui.end_row();
}
