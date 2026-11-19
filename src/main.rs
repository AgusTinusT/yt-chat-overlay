#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod youtube;

use config::Config;
use eframe::egui::{
    self, text::LayoutJob, vec2, Align, Align2, Color32, FontData, FontDefinitions, FontFamily,
    FontId, Frame, Layout, Margin, Rect, Sense, Stroke, TextFormat, ViewportCommand,
};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use youtube::{Event, Kind, Role};

fn main() -> eframe::Result<()> {
    let cfg = Config::load();
    let mut vp = egui::ViewportBuilder::default()
        .with_title("YT Chat Overlay")
        .with_inner_size(cfg.size)
        .with_min_inner_size([240.0, 160.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(true);
    if let Some(p) = cfg.pos {
        vp = vp.with_position(p);
    }
    let options = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native(
        "YT Chat Overlay",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, cfg)))),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum MsgKind {
    Chat(Kind, Role),
    System,
}

struct Msg {
    author: String,
    text: String,
    amount: Option<String>,
    kind: MsgKind,
    t: Instant,
}

struct App {
    cfg: Config,
    input_buf: String,
    status: String,
    msgs: VecDeque<Msg>,
    seen: HashSet<String>,
    seen_order: VecDeque<String>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    gen: Arc<AtomicU64>,
    edit: bool,
    hidden: bool,
    _hotkeys: Option<GlobalHotKeyManager>,
    hk_toggle: u32,
    hk_edit: u32,
    hwnd: Option<isize>,
    affinity_applied: Option<bool>,
    passthrough_applied: Option<bool>,
}

impl App {
    fn new(cc: &eframe::CreationContext, cfg: Config) -> Self {
        setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let (tx, rx) = channel();
        let mut app = App {
            input_buf: cfg.input.clone(),
            cfg,
            status: "Belum terhubung".into(),
            msgs: VecDeque::new(),
            seen: HashSet::new(),
            seen_order: VecDeque::new(),
            tx,
            rx,
            gen: Arc::new(AtomicU64::new(0)),
            edit: false,
            hidden: false,
            _hotkeys: None,
            hk_toggle: 0,
            hk_edit: 0,
            hwnd: None,
            affinity_applied: None,
            passthrough_applied: None,
        };

        match GlobalHotKeyManager::new() {
            Ok(m) => {
                let mods = Some(Modifiers::CONTROL | Modifiers::SHIFT);
                let toggle = HotKey::new(mods, Code::F9);
                let edit = HotKey::new(mods, Code::F10);
                let ok = m.register(toggle).is_ok() & m.register(edit).is_ok();
                app.hk_toggle = toggle.id();
                app.hk_edit = edit.id();
                app._hotkeys = Some(m);
                if !ok {
                    app.system("Hotkey gagal didaftarkan (mungkin dipakai aplikasi lain).");
                }
            }
            Err(e) => app.system(format!("Hotkey tidak aktif: {e}")),
        }

        if app.cfg.input.trim().is_empty() {
            app.edit = true;
        } else {
            app.connect(&cc.egui_ctx);
        }
        app
    }

    fn system(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.msgs.push_back(Msg {
            author: "●".into(),
            text: self.status.clone(),
            amount: None,
            kind: MsgKind::System,
            t: Instant::now(),
        });
    }

    fn connect(&mut self, ctx: &egui::Context) {
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.cfg.input = self.input_buf.trim().to_string();
        self.cfg.save();
        self.seen.clear();
        self.seen_order.clear();
        self.system("Menghubungkan...");
        youtube::spawn(self.cfg.input.clone(), gen, self.gen.clone(), self.tx.clone(), ctx.clone());
    }

    fn pump_events(&mut self) {
        let cur = self.gen.load(Ordering::SeqCst);
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Status { gen, text } if gen == cur => self.system(text),
                Event::Items { gen, items } if gen == cur => {
                    for it in items {
                        if !it.id.is_empty() {
                            if !self.seen.insert(it.id.clone()) {
                                continue;
                            }
                            self.seen_order.push_back(it.id);
                            if self.seen_order.len() > 3000 {
                                if let Some(old) = self.seen_order.pop_front() {
                                    self.seen.remove(&old);
                                }
                            }
                        }
                        self.msgs.push_back(Msg {
                            author: it.author,
                            text: it.text,
                            amount: it.amount,
                            kind: MsgKind::Chat(it.kind, it.role),
                            t: Instant::now(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn prune(&mut self) {
        let fade = self.cfg.fade_secs;
        if fade > 0.0 {
            let now = Instant::now();
            self.msgs.retain(|m| now.duration_since(m.t).as_secs_f32() < fade);
        }
        while self.msgs.len() > self.cfg.max_msgs.max(1) {
            self.msgs.pop_front();
        }
    }

    fn set_edit(&mut self, ctx: &egui::Context, on: bool) {
        self.edit = on;
        if on {
            self.hidden = false;
        } else {
            self.save_window(ctx);
        }
    }

    fn save_window(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if let Some(r) = i.viewport().outer_rect {
                self.cfg.pos = Some([r.min.x, r.min.y]);
            }
            if let Some(r) = i.viewport().inner_rect {
                self.cfg.size = [r.width(), r.height()];
            }
        });
        self.cfg.save();
    }

    fn ensure_hwnd(&mut self, frame: &eframe::Frame) {
        if self.hwnd.is_some() {
            return;
        }
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(h) = frame.window_handle() {
            if let RawWindowHandle::Win32(w) = h.as_raw() {
                self.hwnd = Some(w.hwnd.get());
            }
        }
    }

    /// WDA_EXCLUDEFROMCAPTURE: overlay terlihat di layar Anda, tapi tidak ikut
    /// terekam Display Capture OBS.
    fn apply_capture_affinity(&mut self) {
        let Some(h) = self.hwnd else { return };
        let want = self.cfg.hide_from_capture;
        if self.affinity_applied == Some(want) {
            return;
        }
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity(
                h as _,
                if want { 0x11 } else { 0 },
            );
        }
        self.affinity_applied = Some(want);
    }

    fn add_samples(&mut self) {
        let now = Instant::now();
        let samples = [
            ("Budi", "Halo bang, mainnya keren!", None, MsgKind::Chat(Kind::Text, Role::Normal)),
            ("Sari", "Ini game apa ya namanya?", None, MsgKind::Chat(Kind::Text, Role::Member)),
            ("ModRian", "Jangan spam ya teman-teman", None, MsgKind::Chat(Kind::Text, Role::Moderator)),
            ("Andi", "Semangat streamnya!", Some("Rp20.000"), MsgKind::Chat(Kind::Paid, Role::Normal)),
        ];
        for (a, t, amt, k) in samples {
            self.msgs.push_back(Msg {
                author: a.into(),
                text: t.into(),
                amount: amt.map(String::from),
                kind: k,
                t: now,
            });
        }
    }

    fn draw_msgs(&self, ui: &mut egui::Ui) {
        let now = Instant::now();
        let size = self.cfg.font_size;
        let fade = self.cfg.fade_secs;
        let bga = (self.cfg.bg_opacity.clamp(0.0, 1.0) * 255.0) as u8;
        let strong = (self.cfg.bg_opacity.max(0.6).min(1.0) * 255.0) as u8;
        let bold = FontId::new(size, FontFamily::Name("bold".into()));
        let regular = FontId::proportional(size);
        let white = Color32::from_rgb(242, 242, 242);
        let gold = Color32::from_rgb(255, 214, 0);

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for m in self.msgs.iter().rev() {
                let age = now.duration_since(m.t).as_secs_f32();
                let a = if fade > 0.0 { ((fade - age) / 1.5).clamp(0.0, 1.0) } else { 1.0 };
                if a <= 0.0 {
                    continue;
                }
                let question = matches!(m.kind, MsgKind::Chat(Kind::Text, _)) && m.text.contains('?');
                let normal_bg = if question {
                    Color32::from_rgba_unmultiplied(20, 45, 110, bga.max(150))
                } else {
                    Color32::from_rgba_unmultiplied(0, 0, 0, bga)
                };
                let (name_col, bg) = match m.kind {
                    MsgKind::System => (Color32::from_rgb(255, 170, 80), normal_bg),
                    MsgKind::Chat(Kind::Paid, _) => {
                        (gold, Color32::from_rgba_unmultiplied(130, 75, 0, strong))
                    }
                    MsgKind::Chat(Kind::Membership, _) => (
                        Color32::from_rgb(110, 230, 130),
                        Color32::from_rgba_unmultiplied(10, 85, 35, strong),
                    ),
                    MsgKind::Chat(_, Role::Owner) => (gold, normal_bg),
                    MsgKind::Chat(_, Role::Moderator) => (Color32::from_rgb(120, 160, 255), normal_bg),
                    MsgKind::Chat(_, Role::Member) => (Color32::from_rgb(95, 215, 115), normal_bg),
                    MsgKind::Chat(_, Role::Normal) => (Color32::from_rgb(150, 210, 255), normal_bg),
                };

                let fmt = |font: &FontId, color: Color32| TextFormat {
                    font_id: font.clone(),
                    color: color.gamma_multiply(a),
                    ..Default::default()
                };
                let mut job = LayoutJob::default();
                job.append(&m.author, 0.0, fmt(&bold, name_col));
                if let Some(amt) = &m.amount {
                    job.append(&format!(" [{amt}]"), 0.0, fmt(&bold, gold));
                }
                if !m.text.is_empty() {
                    job.append(": ", 0.0, fmt(&regular, white));
                    job.append(&m.text, 0.0, fmt(&regular, white));
                }

                Frame::none()
                    .fill(bg.gamma_multiply(a))
                    .rounding(6.0)
                    .inner_margin(Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(job).wrap().selectable(false));
                    });
            }
        });
    }

    fn draw_edit(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let full = ui.max_rect();
        let accent = Color32::from_rgb(255, 90, 90);
        ui.painter()
            .rect(full, 8.0, Color32::from_black_alpha(205), Stroke::new(2.0_f32, accent));

        let (bar_rect, bar) =
            ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click_and_drag());
        ui.painter().rect_filled(bar_rect, 8.0, Color32::from_rgba_unmultiplied(255, 90, 90, 60));
        ui.painter().text(
            bar_rect.left_center() + vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            "MODE EDIT  -  seret bagian ini untuk memindah",
            FontId::proportional(14.0),
            Color32::WHITE,
        );
        if bar.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }

        let (mut connect, mut sample, mut done, mut quit) = (false, false, false, false);
        Frame::none().inner_margin(Margin::symmetric(10.0, 6.0)).show(ui, |ui| {
            ui.label("Channel YouTube (@nama) atau link live:");
            ui.add(
                egui::TextEdit::singleline(&mut self.input_buf)
                    .hint_text("@namachannel")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                connect = ui.button("Hubungkan").clicked();
                sample = ui.button("Contoh pesan").clicked();
                done = ui.button("Selesai").clicked();
                quit = ui.button("Keluar").clicked();
            });
            ui.add(egui::Slider::new(&mut self.cfg.font_size, 12.0..=36.0).text("Ukuran huruf"));
            ui.add(egui::Slider::new(&mut self.cfg.fade_secs, 0.0..=120.0).text("Hilang (detik, 0=tidak)"));
            ui.add(egui::Slider::new(&mut self.cfg.max_msgs, 1..=30).text("Jumlah pesan"));
            ui.add(egui::Slider::new(&mut self.cfg.bg_opacity, 0.0..=1.0).text("Gelap latar"));
            ui.checkbox(&mut self.cfg.hide_from_capture, "Sembunyikan dari OBS (tidak ikut terekam)");
            ui.small(format!("Status: {}", self.status));
            ui.small("Ctrl+Shift+F9 = tampil/sembunyi   |   Ctrl+Shift+F10 = mode edit");
        });
        ui.separator();
        self.draw_msgs(ui);

        // Pegangan resize di pojok kanan bawah
        let grip = Rect::from_min_max(full.max - vec2(22.0, 22.0), full.max);
        let resp = ui.interact(grip, ui.id().with("resize_grip"), Sense::drag());
        for k in 0..3 {
            let o = 6.0 + k as f32 * 5.0;
            ui.painter().line_segment(
                [grip.max - vec2(o, 3.0), grip.max - vec2(3.0, o)],
                Stroke::new(2.0_f32, accent),
            );
        }
        if resp.drag_started() {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(
                egui::viewport::ResizeDirection::SouthEast,
            ));
        }

        if connect {
            self.connect(ctx);
        }
        if sample {
            self.add_samples();
        }
        if done {
            self.set_edit(ctx, false);
        }
        if quit {
            self.save_window(ctx);
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ensure_hwnd(frame);
        self.apply_capture_affinity();

        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if ev.state != HotKeyState::Pressed {
                continue;
            }
            if ev.id == self.hk_toggle {
                self.hidden = !self.hidden;
            } else if ev.id == self.hk_edit {
                self.set_edit(ctx, !self.edit);
            }
        }

        self.pump_events();
        self.prune();

        // Mode main: klik mouse tembus ke game. Mode edit: jendela bisa diklik.
        let passthrough = !self.edit;
        if self.passthrough_applied != Some(passthrough) {
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(passthrough));
            if self.edit {
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            self.passthrough_applied = Some(passthrough);
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_window(ctx);
        }

        egui::CentralPanel::default().frame(Frame::none()).show(ctx, |ui| {
            if self.edit {
                self.draw_edit(ui, ctx);
            } else if !self.hidden {
                self.draw_msgs(ui);
            }
        });

        let ms = if self.msgs.is_empty() || self.hidden { 250 } else { 66 };
        ctx.request_repaint_after(Duration::from_millis(ms));
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::path::Path::new(r"C:\Windows\Fonts");
    let mut prop = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();

    if let Ok(b) = std::fs::read(dir.join("segoeui.ttf")) {
        fonts.font_data.insert("segoe".into(), FontData::from_owned(b));
        prop.insert(0, "segoe".into());
    }
    if let Ok(b) = std::fs::read(dir.join("seguisym.ttf")) {
        fonts.font_data.insert("segoe_sym".into(), FontData::from_owned(b));
        prop.push("segoe_sym".into());
    }
    let mut bold = prop.clone();
    if let Ok(b) = std::fs::read(dir.join("segoeuib.ttf")) {
        fonts.font_data.insert("segoe_bold".into(), FontData::from_owned(b));
        bold.insert(0, "segoe_bold".into());
    }
    fonts.families.insert(FontFamily::Proportional, prop);
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
}
