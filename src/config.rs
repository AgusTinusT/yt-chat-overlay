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
    /// Versi portable: cek versi baru di GitHub saat aplikasi dibuka.
    /// Versi Microsoft Store tidak memakai ini (update lewat Store).
    pub check_updates: bool,
    /// Kata yang disorot, dipisah koma (mis. "agus, bang tinus")
    pub highlight: String,
    /// Bahasa tampilan: "en" (bawaan) atau "id"
    pub language: String,
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
            check_updates: true,
            highlight: String::new(),
            language: "en".into(),
        }
    }
}

const FILE: &str = "yt-chat-overlay.json";

/// Versi Store (MSIX): folder instalasi tidak bisa ditulisi, jadi pengaturan disimpan di
/// %APPDATA%\YTChatOverlay. Versi portable: di samping .exe, seperti sebelumnya.
fn path() -> PathBuf {
    if crate::packaged::is_packaged() {
        if let Some(dir) = std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("YTChatOverlay")) {
            let _ = std::fs::create_dir_all(&dir);
            return dir.join(FILE);
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(FILE)))
        .unwrap_or_else(|| PathBuf::from(FILE))
}

impl Config {
    /// Kata sorot (huruf kecil): isi `highlight` + @handle dari `input` jika ada.
    pub fn highlight_words(&self) -> Vec<String> {
        let mut words: Vec<String> = self
            .highlight
            .split(',')
            .map(|w| w.trim().to_lowercase())
            .filter(|w| w.chars().count() >= 2)
            .collect();
        let input = self.input.trim();
        let handle = input.strip_prefix('@').map(str::to_string).or_else(|| {
            input.split("/@").nth(1).map(|s| s.split(['/', '?']).next().unwrap_or("").to_string())
        });
        if let Some(h) = handle.filter(|h| !h.is_empty()) {
            words.push(format!("@{}", h.to_lowercase()));
        }
        words
    }

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

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn highlight_words() {
        let mut c = Config { highlight: " Agus , bang tinus,,x".into(), input: "@AgusTinusT".into(), ..Default::default() };
        assert_eq!(c.highlight_words(), ["agus", "bang tinus", "@agustinust"]);
        c.input = "https://www.youtube.com/@AgusTinusT/live".into();
        assert_eq!(c.highlight_words().last().unwrap(), "@agustinust");
        c.input = "https://youtu.be/abc".into();
        c.highlight.clear();
        assert!(c.highlight_words().is_empty());
    }
}
