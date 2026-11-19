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
    /// Contoh format: "Ctrl+Alt+C", "Ctrl+Shift+F9", "Alt+Shift+Z"
    pub hotkey_toggle: String,
    pub hotkey_edit: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: String::new(),
            font_size: 18.0,
            fade_secs: 30.0,
            max_msgs: 8,
            bg_opacity: 0.55,
            // Mati secara bawaan: fungsi ini juga dipakai cheat untuk bersembunyi dari
            // screenshot anti-cheat. Pakai Game Capture di OBS agar overlay tidak terekam.
            hide_from_capture: false,
            pos: None,
            size: [400.0, 460.0],
            // Tanpa tombol F supaya mudah di keyboard compact
            hotkey_toggle: "Ctrl+Alt+C".into(),
            hotkey_edit: "Ctrl+Alt+E".into(),
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
