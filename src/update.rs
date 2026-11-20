//! Cek versi baru untuk versi portable (GitHub Releases).
//! Versi Microsoft Store tidak memakai ini: Store memperbarui aplikasi otomatis.

use std::sync::{Arc, Mutex};

pub const RELEASES_URL: &str = "https://github.com/AgusTinusT/yt-chat-overlay/releases/latest";
const API_URL: &str = "https://api.github.com/repos/AgusTinusT/yt-chat-overlay/releases/latest";

/// Versi terbaru (mis. "0.2.0") jika lebih baru dari versi yang sedang berjalan.
pub type Latest = Arc<Mutex<Option<String>>>;

/// Cek sekali di thread terpisah. Gagal (offline, rate limit) = diam saja.
pub fn spawn_check(ctx: eframe::egui::Context) -> Latest {
    let latest: Latest = Arc::new(Mutex::new(None));
    let out = latest.clone();
    std::thread::spawn(move || {
        let Ok(resp) = ureq::get(API_URL)
            .set("Accept", "application/vnd.github+json")
            .set("User-Agent", "yt-chat-overlay-update-check")
            .timeout(std::time::Duration::from_secs(10))
            .call()
        else {
            return;
        };
        let Ok(body) = resp.into_string() else { return };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else { return };
        let Some(tag) = json["tag_name"].as_str() else { return };
        if is_newer(tag, env!("CARGO_PKG_VERSION")) {
            *out.lock().unwrap() = Some(tag.trim_start_matches('v').to_string());
            ctx.request_repaint();
        }
    });
    latest
}

fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u64>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

/// True jika `remote` lebih baru dari `current`. Format tidak dikenal = false.
pub fn is_newer(remote: &str, current: &str) -> bool {
    matches!((parse(remote), parse(current)), (Some(r), Some(c)) if r > c)
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn version_compare() {
        assert!(is_newer("v0.2.0", "0.1.2"));
        assert!(is_newer("v0.1.10", "0.1.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("v0.1.2", "0.1.2"));
        assert!(!is_newer("v0.1.1", "0.1.2"));
        assert!(!is_newer("nightly", "0.1.2"));
    }
}
