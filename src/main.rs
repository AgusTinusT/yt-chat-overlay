#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod i18n;
mod packaged;
mod update;
mod youtube;

use config::Config;
use i18n::tr;
use eframe::egui::{
    self, text::LayoutJob, vec2, Align, Align2, Color32, FontData, FontDefinitions, FontFamily,
    FontId, Frame, Layout, Margin, Rect, Sense, Stroke, TextFormat, ViewportCommand,
};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use youtube::{Event, Kind, Role};

const ICON_PNG: &[u8] = include_bytes!("../assets/icon-256.png");

/// Mencegah aplikasi berjalan dua kali.
mod single_instance {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// `Some(event)` jika ini instance pertama. Jika aplikasi sudah berjalan,
    /// instance lama diberi sinyal untuk membuka mode edit, lalu `None`.
    pub fn acquire() -> Option<isize> {
        unsafe {
            let mutex = CreateMutexW(std::ptr::null(), 0, wide("Local\\YTChatOverlay.Instance").as_ptr());
            let exists = !mutex.is_null() && GetLastError() == ERROR_ALREADY_EXISTS;
            // Mutex sengaja tidak ditutup: dipegang sampai proses selesai.
            let event = CreateEventW(std::ptr::null(), 0, 0, wide("Local\\YTChatOverlay.Show").as_ptr());
            if exists {
                if !event.is_null() {
                    SetEvent(event);
                }
                None
            } else {
                Some(event as isize)
            }
        }
    }

    /// True jika instance lain meminta jendela ini ditampilkan.
    pub fn show_requested(event: isize) -> bool {
        event != 0 && unsafe { WaitForSingleObject(event as _, 0) == WAIT_OBJECT_0 }
    }
}

fn main() -> eframe::Result<()> {
    let Some(show_event) = single_instance::acquire() else {
        return Ok(());
    };
    let cfg = Config::load();
    i18n::set(&cfg.language);
    let mut vp = egui::ViewportBuilder::default()
        .with_title("YT Chat Overlay")
        .with_inner_size(cfg.size)
        .with_min_inner_size([240.0, 160.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(true);
    if let Some(p) = cfg.pos {
        vp = vp.with_position(p);
    }
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        vp = vp.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native(
        "YT Chat Overlay",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, cfg, show_event)))),
    )
}

struct Tray {
    icon: TrayIcon,
    toggle: MenuItem,
    edit: MenuItem,
    clear: MenuItem,
    quit: MenuItem,
}

impl Tray {
    /// Teks menu sesuai bahasa aktif (dipanggil lagi saat bahasa diganti).
    fn set_texts(&self) {
        self.toggle.set_text(tr("Show / hide chat", "Tampilkan / sembunyikan chat"));
        self.edit.set_text(tr("Edit mode (position && settings)", "Mode edit (posisi && pengaturan)"));
        self.clear.set_text(tr("Clear chat", "Bersihkan chat"));
        self.quit.set_text(tr("Quit", "Keluar"));
        let _ = self.icon.set_tooltip(Some(tr(
            "YT Chat Overlay\nClick: edit mode  |  Right-click: menu",
            "YT Chat Overlay\nKlik: mode edit  |  Klik kanan: menu",
        )));
    }
}

fn build_tray() -> Option<Tray> {
    let toggle = MenuItem::new("", true, None);
    let edit = MenuItem::new("", true, None);
    let clear = MenuItem::new("", true, None);
    let quit = MenuItem::new("", true, None);
    let menu = Menu::new();
    menu.append_items(&[&edit, &toggle, &clear, &PredefinedMenuItem::separator(), &quit]).ok()?;
    let data = eframe::icon_data::from_png_bytes(ICON_PNG).ok()?;
    let icon = tray_icon::Icon::from_rgba(data.rgba, data.width, data.height).ok()?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_icon(icon)
        .build()
        .ok()?;
    let t = Tray { icon: tray, toggle, edit, clear, quit };
    t.set_texts();
    Some(t)
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
    hotkeys: Option<GlobalHotKeyManager>,
    reg_toggle: Option<HotKey>,
    reg_edit: Option<HotKey>,
    tray: Option<Tray>,
    show_event: isize,
    hwnd: Option<isize>,
    affinity_applied: Option<bool>,
    passthrough_applied: Option<bool>,
    /// Hasil cek versi baru (hanya versi portable)
    latest: Option<update::Latest>,
    update_notified: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext, cfg: Config, show_event: isize) -> Self {
        setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let (tx, rx) = channel();
        let mut app = App {
            input_buf: cfg.input.clone(),
            cfg,
            status: tr("Not connected", "Belum terhubung").into(),
            msgs: VecDeque::new(),
            seen: HashSet::new(),
            seen_order: VecDeque::new(),
            tx,
            rx,
            gen: Arc::new(AtomicU64::new(0)),
            edit: false,
            hidden: false,
            hotkeys: None,
            reg_toggle: None,
            reg_edit: None,
            tray: build_tray(),
            show_event,
            hwnd: None,
            affinity_applied: None,
            passthrough_applied: None,
            latest: None,
            update_notified: false,
        };
        // Versi Store diperbarui oleh Microsoft Store, jadi hanya versi portable yang cek GitHub.
        if !packaged::is_packaged() && app.cfg.check_updates {
            app.latest = Some(update::spawn_check(cc.egui_ctx.clone()));
        }
        if app.tray.is_none() {
            app.system(tr("Could not create the system tray icon.", "Ikon system tray gagal dibuat."));
        }

        match GlobalHotKeyManager::new() {
            Ok(m) => {
                app.hotkeys = Some(m);
                app.register_hotkeys();
            }
            Err(e) => app.system(format!("{}: {e}", tr("Hotkeys unavailable", "Hotkey tidak aktif"))),
        }

        if app.cfg.input.trim().is_empty() {
            app.edit = true;
        } else {
            app.connect(&cc.egui_ctx);
        }
        app
    }

    /// (Ulang) daftarkan hotkey dari config. Dipanggil saat start dan saat tombol "Terapkan" diklik.
    fn register_hotkeys(&mut self) {
        let Some(m) = &self.hotkeys else { return };
        for hk in [self.reg_toggle.take(), self.reg_edit.take()].into_iter().flatten() {
            let _ = m.unregister(hk);
        }
        let mut errors = Vec::new();
        let mut register = |text: &str| -> Option<HotKey> {
            match text.trim().parse::<HotKey>() {
                Ok(hk) if m.register(hk).is_ok() => Some(hk),
                Ok(_) => {
                    errors.push(format!("{text} {}", tr("is already used by another app", "sudah dipakai aplikasi lain")));
                    None
                }
                Err(_) => {
                    errors.push(format!("{}: \"{text}\"", tr("Invalid hotkey format", "Format hotkey salah")));
                    None
                }
            }
        };
        self.reg_toggle = register(&self.cfg.hotkey_toggle);
        self.reg_edit = register(&self.cfg.hotkey_edit);
        if errors.is_empty() {
            self.cfg.save();
        } else {
            self.system(format!(
                "{}. {}",
                errors.join(". "),
                tr("The system tray menu still works.", "Menu di system tray tetap bisa dipakai.")
            ));
        }
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
        self.system(tr("Connecting...", "Menghubungkan..."));
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

    fn new_version(&self) -> Option<String> {
        self.latest.as_ref().and_then(|l| l.lock().ok().and_then(|v| v.clone()))
    }

    fn check_update_notice(&mut self) {
        if self.update_notified {
            return;
        }
        if let Some(v) = self.new_version() {
            self.update_notified = true;
            self.system(format!(
                "{} {v} {}",
                tr("New version", "Versi baru"),
                tr("is available. Open edit mode to download it.", "tersedia. Buka mode edit untuk mengunduh.")
            ));
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
            ("Budi", tr("Hi! Great gameplay!", "Halo bang, mainnya keren!"), None, MsgKind::Chat(Kind::Text, Role::Normal)),
            ("Sari", tr("What game is this?", "Ini game apa ya namanya?"), None, MsgKind::Chat(Kind::Text, Role::Member)),
            ("ModRian", tr("No spamming, please", "Jangan spam ya teman-teman"), None, MsgKind::Chat(Kind::Text, Role::Moderator)),
            ("Andi", tr("Keep it up!", "Semangat streamnya!"), Some(tr("$5.00", "Rp20.000")), MsgKind::Chat(Kind::Paid, Role::Normal)),
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
        // Contoh pesan yang disorot (kalau ada kata sorot)
        if let Some(w) = self.cfg.highlight_words().first() {
            self.msgs.push_back(Msg {
                author: "Dewi".into(),
                text: if i18n::is_id() { format!("Halo {w}, salam dari Bandung!") } else { format!("Hello {w}, greetings from Bandung!") },
                amount: None,
                kind: MsgKind::Chat(Kind::Text, Role::Normal),
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
        let words = self.cfg.highlight_words();

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for m in self.msgs.iter().rev() {
                let age = now.duration_since(m.t).as_secs_f32();
                let a = if fade > 0.0 { ((fade - age) / 1.5).clamp(0.0, 1.0) } else { 1.0 };
                if a <= 0.0 {
                    continue;
                }
                let is_text = matches!(m.kind, MsgKind::Chat(Kind::Text, _));
                let mentioned = is_text && !words.is_empty() && {
                    let lower = m.text.to_lowercase();
                    words.iter().any(|w| lower.contains(w.as_str()))
                };
                let question = is_text && m.text.contains('?');
                let normal_bg = if mentioned {
                    Color32::from_rgba_unmultiplied(95, 35, 125, bga.max(170))
                } else if question {
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
            tr("EDIT MODE  -  drag here to move", "MODE EDIT  -  seret bagian ini untuk memindah"),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
        if bar.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }

        let (mut connect, mut sample, mut done, mut quit) = (false, false, false, false);
        let mut clear = false;
        let mut apply_hk = false;
        let mut lang_changed = false;
        Frame::none().inner_margin(Margin::symmetric(10.0, 6.0)).show(ui, |ui| {
            ui.label(tr("YouTube channel (@name) or live link:", "Channel YouTube (@nama) atau link live:"));
            ui.add(
                egui::TextEdit::singleline(&mut self.input_buf)
                    .hint_text(tr("@channelname", "@namachannel"))
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                connect = ui.button(tr("Connect", "Hubungkan")).clicked();
                sample = ui.button(tr("Sample messages", "Contoh pesan")).clicked();
                clear = ui.button(tr("Clear", "Bersihkan")).clicked();
                done = ui.button(tr("Done", "Selesai")).clicked();
                quit = ui.button(tr("Quit", "Keluar")).clicked();
            });
            ui.add(egui::Slider::new(&mut self.cfg.font_size, 12.0..=36.0).text(tr("Font size", "Ukuran huruf")));
            let never = tr("Never", "Tidak pernah");
            ui.add(
                egui::Slider::new(&mut self.cfg.fade_secs, 0.0..=120.0)
                    .text(tr("Hide messages after (seconds)", "Pesan hilang setelah (detik)"))
                    .custom_formatter(move |v, _| if v <= 0.0 { never.into() } else { format!("{v:.0}") }),
            );
            ui.add(egui::Slider::new(&mut self.cfg.max_msgs, 1..=30).text(tr("Max messages", "Jumlah pesan maksimal")));
            if self.cfg.fade_secs <= 0.0 {
                ui.small(tr(
                    "Messages never fade. Old ones are removed only when the max is exceeded.",
                    "Pesan tidak hilang sendiri. Pesan lama baru terhapus jika melebihi jumlah maksimal.",
                ));
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Highlight words:", "Sorot kata:"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.cfg.highlight)
                        .hint_text(tr("name, nickname (comma-separated)", "nama, panggilan (pisah koma)"))
                        .desired_width(180.0),
                )
                .on_hover_text(tr(
                    "Messages containing these words get a purple background.\nThe @channel entered above is highlighted automatically.",
                    "Pesan yang berisi kata ini diberi latar ungu.\n@channel yang diisi di atas otomatis ikut disorot.",
                ));
            });
            ui.add(egui::Slider::new(&mut self.cfg.bg_opacity, 0.0..=1.0).text(tr("Background darkness", "Gelap latar")));
            ui.checkbox(&mut self.cfg.hide_from_capture, tr("Hide from OBS Display Capture", "Sembunyikan dari Display Capture OBS"))
                .on_hover_text(tr(
                    "Not needed if OBS uses Game Capture.\nAvoid with games that use strict anti-cheat.",
                    "Tidak perlu jika OBS memakai Game Capture.\nHindari untuk game dengan anti-cheat ketat.",
                ));
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Show/hide hotkey:", "Hotkey tampil/sembunyi:"));
                ui.add(egui::TextEdit::singleline(&mut self.cfg.hotkey_toggle).desired_width(110.0));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Edit mode hotkey:", "Hotkey mode edit:"));
                ui.add(egui::TextEdit::singleline(&mut self.cfg.hotkey_edit).desired_width(110.0));
                apply_hk = ui
                    .button(tr("Apply", "Terapkan"))
                    .on_hover_text(tr("Examples: Ctrl+Alt+C, Alt+Shift+Z, Ctrl+Shift+F9", "Contoh: Ctrl+Alt+C, Alt+Shift+Z, Ctrl+Shift+F9"))
                    .clicked();
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Language / Bahasa:");
                let current = i18n::LANGS.iter().find(|l| l.0 == self.cfg.language).map_or("English", |l| l.1);
                egui::ComboBox::from_id_salt("language").selected_text(current).show_ui(ui, |ui| {
                    for (code, name) in i18n::LANGS {
                        if ui.selectable_label(self.cfg.language == code, name).clicked() && self.cfg.language != code {
                            self.cfg.language = code.into();
                            lang_changed = true;
                        }
                    }
                });
            });
            let ver = env!("CARGO_PKG_VERSION");
            if packaged::is_packaged() {
                ui.small(format!("v{ver} ({})", tr("Microsoft Store, updates automatically", "Microsoft Store, update otomatis")));
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.cfg.check_updates, tr("Check for updates on startup", "Cek versi baru saat dibuka"))
                        .on_hover_text(tr(
                            "Checks GitHub Releases once each time the app starts.",
                            "Mengecek GitHub Releases sekali setiap aplikasi dibuka.",
                        ));
                    ui.small(format!("v{ver}"));
                });
                if let Some(v) = self.new_version() {
                    ui.hyperlink_to(
                        format!("{} {v} - {}", tr("New version", "Versi baru"), tr("click to download", "klik untuk mengunduh")),
                        update::RELEASES_URL,
                    );
                }
            }
            ui.small(format!("Status: {}", self.status));
            ui.small(tr(
                "All menus are also in the system tray icon (bottom right).",
                "Semua menu juga ada di ikon system tray (pojok kanan bawah).",
            ));
        });
        if lang_changed {
            i18n::set(&self.cfg.language);
            if let Some(t) = &self.tray {
                t.set_texts();
            }
            self.cfg.save();
        }
        if apply_hk {
            self.register_hotkeys();
        }
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
        if clear {
            self.msgs.clear();
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
            if self.reg_toggle.map(|h| h.id()) == Some(ev.id) {
                self.hidden = !self.hidden;
            } else if self.reg_edit.map(|h| h.id()) == Some(ev.id) {
                self.set_edit(ctx, !self.edit);
            }
        }

        // Ikon system tray: klik kiri = mode edit, klik kanan = menu
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                self.set_edit(ctx, !self.edit);
            }
        }
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let Some(t) = &self.tray else { continue };
            if ev.id == *t.toggle.id() {
                if self.edit {
                    self.set_edit(ctx, false);
                }
                self.hidden = !self.hidden;
            } else if ev.id == *t.edit.id() {
                self.set_edit(ctx, true);
            } else if ev.id == *t.clear.id() {
                self.msgs.clear();
            } else if ev.id == *t.quit.id() {
                self.save_window(ctx);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }

        // Aplikasi dibuka lagi -> tampilkan instance yang sudah berjalan
        if single_instance::show_requested(self.show_event) {
            self.set_edit(ctx, true);
        }

        self.pump_events();
        self.check_update_notice();
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

#[cfg(test)]
mod tests {
    use global_hotkey::hotkey::HotKey;

    #[test]
    fn hotkey_formats_parse() {
        for s in ["Ctrl+Alt+C", "Ctrl+Alt+E", "Ctrl+Shift+F9", "Alt+Shift+Z", "ctrl+alt+c"] {
            assert!(s.parse::<HotKey>().is_ok(), "gagal parse {s}");
        }
        assert!("Ctrl+Alt+".parse::<HotKey>().is_err());
    }
}
