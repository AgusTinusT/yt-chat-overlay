//! Render assets/logo.svg menjadi PNG dan icon.ico.
//! Jalankan: cargo run --example make_icon

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::ExtendedColorType;
use resvg::{tiny_skia, usvg};

fn pixmap(tree: &usvg::Tree, size: u32) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let scale = size as f32 / tree.size().width();
    resvg::render(tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap
}

fn render(tree: &usvg::Tree, size: u32) -> Vec<u8> {
    pixmap(tree, size).encode_png().expect("png")
}

/// Piksel RGBA biasa (tiny-skia menyimpan warna dalam bentuk premultiplied).
fn rgba(tree: &usvg::Tree, size: u32) -> Vec<u8> {
    pixmap(tree, size)
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

fn main() {
    let svg = std::fs::read("assets/logo.svg").expect("assets/logo.svg");
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default()).expect("svg");

    std::fs::write("assets/logo.png", render(&tree, 1024)).unwrap();
    std::fs::write("assets/icon-256.png", render(&tree, 256)).unwrap();

    let frames: Vec<IcoFrame> = [16, 24, 32, 48, 64, 128, 256]
        .iter()
        .map(|&s| IcoFrame::as_png(&rgba(&tree, s), s, s, ExtendedColorType::Rgba8).unwrap())
        .collect();
    let file = std::fs::File::create("assets/icon.ico").unwrap();
    IcoEncoder::new(file).encode_images(&frames).unwrap();

    // Logo untuk paket MSIX (Microsoft Store)
    let dir = std::path::Path::new("packaging/Assets");
    std::fs::create_dir_all(dir).unwrap();
    for (name, size) in [
        ("Square44x44Logo.png", 44),
        ("Square150x150Logo.png", 150),
        ("Square310x310Logo.png", 310),
        ("StoreLogo.png", 50),
    ] {
        std::fs::write(dir.join(name), render(&tree, size)).unwrap();
    }
    // Wide 310x150: logo 120px di tengah, latar transparan
    let mut wide = tiny_skia::Pixmap::new(310, 150).unwrap();
    let logo = pixmap(&tree, 120);
    wide.draw_pixmap(95, 15, logo.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
    std::fs::write(dir.join("Wide310x150Logo.png"), wide.encode_png().unwrap()).unwrap();

    // Gambar untuk halaman Store (Partner Center > Store listing > Store logos)
    let art = std::path::Path::new("packaging/StoreArt");
    std::fs::create_dir_all(art).unwrap();
    let bg = tiny_skia::Color::from_rgba8(0x1B, 0x1B, 0x22, 0xFF);
    for (name, w, h, logo_size) in [("BoxArt-2160x2160.png", 2160, 2160, 1500), ("PosterArt-1440x2160.png", 1440, 2160, 1100)] {
        let mut canvas = tiny_skia::Pixmap::new(w, h).unwrap();
        canvas.fill(bg);
        let logo = pixmap(&tree, logo_size);
        let (x, y) = (((w - logo_size) / 2) as i32, ((h - logo_size) / 2) as i32);
        canvas.draw_pixmap(x, y, logo.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
        std::fs::write(art.join(name), canvas.encode_png().unwrap()).unwrap();
    }

    println!("Selesai: assets/*, packaging/Assets/*.png, packaging/StoreArt/*.png");
}
