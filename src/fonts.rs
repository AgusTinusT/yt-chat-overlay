//! Font aplikasi: Segoe UI, ditambah font cadangan Windows untuk huruf lain
//! (Korea, Jepang/Mandarin, Thai, India).
//!
//! Font cadangan berukuran besar (sampai ~13 MB per file), jadi baru dimuat saat
//! sebuah nama atau pesan benar-benar memakai hurufnya. Aplikasi tetap ringan untuk
//! chat yang hanya memakai huruf Latin.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::collections::HashSet;
use std::path::Path;

const FONT_DIR: &str = r"C:\Windows\Fonts";

struct Fallback {
    key: &'static str,
    /// Dicoba berurutan; yang pertama ada yang dipakai
    files: &'static [&'static str],
    needs: fn(u32) -> bool,
}

/// Urutan di sini = urutan pencarian huruf. CJK sengaja sebelum Korea: font Korea juga punya
/// sebagian aksara Han, tapi dengan ukuran berbeda, sehingga hurufnya tampak naik.
const FALLBACKS: [Fallback; 4] = [
    Fallback {
        // Microsoft YaHei: aksara Han (Mandarin sederhana dan tradisional) + kana Jepang
        key: "cjk",
        files: &["msyh.ttc", "YuGothM.ttc", "msgothic.ttc"],
        needs: |c| {
            matches!(c, 0x3000..=0x30FF | 0x31F0..=0x31FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
        },
    },
    Fallback {
        key: "korean",
        files: &["malgun.ttf"],
        needs: |c| matches!(c, 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7AF),
    },
    Fallback {
        // Leelawadee UI: Thai, Lao, Khmer
        key: "thai",
        files: &["LeelawUI.ttf"],
        needs: |c| matches!(c, 0x0E00..=0x0EFF | 0x1780..=0x17FF),
    },
    Fallback {
        // Nirmala UI: Devanagari, Bengali, Tamil, Telugu, dan aksara India lain
        key: "indic",
        files: &["Nirmala.ttc", "Nirmala.ttf"],
        needs: |c| matches!(c, 0x0900..=0x0DFF),
    },
];

pub struct Fonts {
    defs: FontDefinitions,
    loaded: HashSet<&'static str>,
}

/// Baca file font sekali dan simpan selama aplikasi berjalan (tidak disalin saat set_fonts).
fn read_static(file: &str) -> Option<FontData> {
    let bytes = std::fs::read(Path::new(FONT_DIR).join(file)).ok()?;
    Some(FontData::from_static(Box::leak(bytes.into_boxed_slice())))
}

impl Fonts {
    pub fn setup(ctx: &egui::Context) -> Self {
        let mut defs = FontDefinitions::default();
        let mut prop = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
        if let Some(f) = read_static("segoeui.ttf") {
            defs.font_data.insert("segoe".into(), f);
            prop.insert(0, "segoe".into());
        }
        if let Some(f) = read_static("seguisym.ttf") {
            defs.font_data.insert("segoe_sym".into(), f);
            prop.push("segoe_sym".into());
        }
        let mut bold = prop.clone();
        if let Some(f) = read_static("segoeuib.ttf") {
            defs.font_data.insert("segoe_bold".into(), f);
            bold.insert(0, "segoe_bold".into());
        }
        defs.families.insert(FontFamily::Proportional, prop);
        defs.families.insert(FontFamily::Name("bold".into()), bold);
        ctx.set_fonts(defs.clone());
        Self { defs, loaded: HashSet::new() }
    }

    /// Muat font cadangan yang dibutuhkan `text` (kalau belum). Murah untuk teks Latin.
    pub fn ensure_for(&mut self, ctx: &egui::Context, text: &str) {
        let mut changed = false;
        for fb in &FALLBACKS {
            if self.loaded.contains(fb.key) || !text.chars().any(|c| (fb.needs)(c as u32)) {
                continue;
            }
            // Ditandai walaupun gagal, supaya tidak mencoba membaca file terus-menerus
            self.loaded.insert(fb.key);
            let Some(data) = fb.files.iter().find_map(|f| read_static(f)) else { continue };
            self.defs.font_data.insert(fb.key.into(), data);
            changed = true;
        }
        if changed {
            // Susun ulang font cadangan sesuai urutan FALLBACKS, apa pun urutan dimuatnya
            for family in [FontFamily::Proportional, FontFamily::Name("bold".into())] {
                if let Some(list) = self.defs.families.get_mut(&family) {
                    list.retain(|n| !FALLBACKS.iter().any(|f| f.key == n));
                    list.extend(
                        FALLBACKS.iter().filter(|f| self.defs.font_data.contains_key(f.key)).map(|f| f.key.to_string()),
                    );
                }
            }
            ctx.set_fonts(self.defs.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FALLBACKS;

    fn needs(key: &str, s: &str) -> bool {
        let fb = FALLBACKS.iter().find(|f| f.key == key).unwrap();
        s.chars().any(|c| (fb.needs)(c as u32))
    }

    #[test]
    fn script_detection() {
        assert!(needs("korean", "안녕하세요"));
        assert!(needs("cjk", "こんにちは"));
        assert!(needs("cjk", "你好"));
        assert!(needs("thai", "สวัสดี"));
        assert!(needs("indic", "नमस्ते"));
        for fb in &FALLBACKS {
            assert!(!"Hello, apa kabar? 😀 Ñandú".chars().any(|c| (fb.needs)(c as u32)), "{}", fb.key);
        }
    }
}
