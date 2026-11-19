use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    /// @handle channel, link channel, atau link video live
    pub input: String,
    pub font_size: f32,
    /// Pesan hilang setelah N detik (0 = tidak pernah)
    pub fade_secs: f32,
    pub max_msgs: usize,
    /// Kegelapan latar setiap pesan (0..1)
    pub bg_opacity: f32,
    /// Jangan ikut terekam Display Capture OBS / screenshot
    pub hide_from_capture: bool,
    pub pos: Option<[f32; 2]>,
    pub size: [f32; 2],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: String::new(),
            font_size: 18.0,
            fade_secs: 30.0,
            max_msgs: 8,
            bg_opacity: 0.55,
            hide_from_capture: true,
            pos: None,
            size: [400.0, 460.0],
        }
    }
}

fn path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("yt-chat-overlay.json")))
        .unwrap_or_else(|| PathBuf::from("yt-chat-overlay.json"))
}

impl Config {
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path(), s);
        }
    }
}
