//! Memuat gambar emoji dan Super Sticker dari YouTube.
//!
//! - Diunduh di thread latar belakang (beberapa sekaligus), jadi UI tidak pernah tersendat.
//! - Disimpan di cache disk (%LOCALAPPDATA%\YTChatOverlay\emoji), jadi emoji member yang
//!   sama tidak diunduh ulang setiap kali aplikasi dibuka.
//! - Di memori dibatasi; kalau terlalu banyak, cache memori dikosongkan dan dimuat ulang dari disk.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const WORKERS: usize = 3;
const MAX_IN_MEMORY: usize = 600;
/// Hanya host gambar resmi YouTube/Google yang diunduh
const ALLOWED_HOSTS: [&str; 4] = [
    "https://yt3.ggpht.com/",
    "https://fonts.gstatic.com/",
    "https://lh3.googleusercontent.com/",
    "https://yt4.ggpht.com/",
];

enum State {
    Loading,
    Ready(TextureHandle),
    Failed,
}

pub struct Cache {
    map: HashMap<String, State>,
    queue: Sender<String>,
    done: Receiver<(String, Option<ColorImage>)>,
}

impl Cache {
    pub fn new(ctx: egui::Context) -> Self {
        let (queue, jobs) = channel::<String>();
        let (done_tx, done) = channel();
        let jobs = Arc::new(Mutex::new(jobs));
        for _ in 0..WORKERS {
            let jobs = jobs.clone();
            let done_tx = done_tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
                loop {
                    let Ok(url) = jobs.lock().unwrap().recv() else { return };
                    let img = load(&agent, &url);
                    if done_tx.send((url, img)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
        }
        Self { map: HashMap::new(), queue, done }
    }

    /// Terima gambar yang selesai diunduh dan jadikan texture. Panggil sekali per frame.
    pub fn pump(&mut self, ctx: &egui::Context) {
        while let Ok((url, img)) = self.done.try_recv() {
            let state = match img {
                Some(img) => State::Ready(ctx.load_texture(&url, img, TextureOptions::LINEAR)),
                None => State::Failed,
            };
            self.map.insert(url, state);
        }
    }

    /// Texture untuk `url` kalau sudah siap; kalau belum, minta diunduh dan kembalikan None.
    pub fn get(&mut self, url: &str) -> Option<TextureHandle> {
        if let Some(s) = self.map.get(url) {
            return match s {
                State::Ready(t) => Some(t.clone()),
                _ => None,
            };
        }
        if !ALLOWED_HOSTS.iter().any(|h| url.starts_with(h)) {
            self.map.insert(url.to_string(), State::Failed);
            return None;
        }
        if self.map.len() >= MAX_IN_MEMORY {
            // Sederhana: kosongkan yang sudah selesai; yang masih dipakai dimuat ulang dari disk.
            self.map.retain(|_, s| matches!(s, State::Loading));
        }
        self.map.insert(url.to_string(), State::Loading);
        let _ = self.queue.send(url.to_string());
        None
    }
}

fn cache_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("YTChatOverlay").join("emoji");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn cache_file(url: &str) -> Option<PathBuf> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    Some(cache_dir()?.join(format!("{:016x}.img", h.finish())))
}

fn load(agent: &ureq::Agent, url: &str) -> Option<ColorImage> {
    let path = cache_file(url);
    let bytes = match path.as_ref().and_then(|p| std::fs::read(p).ok()) {
        Some(b) => b,
        None => {
            use std::io::Read;
            let mut b = Vec::new();
            let resp = agent.get(url).call().ok()?;
            resp.into_reader().take(4 * 1024 * 1024).read_to_end(&mut b).ok()?;
            if let Some(p) = &path {
                let _ = std::fs::write(p, &b);
            }
            b
        }
    };
    decode(&bytes)
}

fn decode(bytes: &[u8]) -> Option<ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}
