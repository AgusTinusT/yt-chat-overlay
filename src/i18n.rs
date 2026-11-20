//! Bahasa tampilan: English (bawaan) atau Bahasa Indonesia.

use std::sync::atomic::{AtomicBool, Ordering};

static INDONESIAN: AtomicBool = AtomicBool::new(false);

/// Kode bahasa yang didukung, untuk pilihan di mode edit.
pub const LANGS: [(&str, &str); 2] = [("en", "English"), ("id", "Bahasa Indonesia")];

pub fn set(code: &str) {
    INDONESIAN.store(code == "id", Ordering::Relaxed);
}

pub fn is_id() -> bool {
    INDONESIAN.load(Ordering::Relaxed)
}

/// Pilih teks sesuai bahasa aktif.
pub fn tr(en: &'static str, id: &'static str) -> &'static str {
    if is_id() { id } else { en }
}

/// Bahasa yang diminta ke YouTube (mempengaruhi teks membership dari YouTube).
pub fn youtube_hl() -> &'static str {
    tr("en", "id")
}
