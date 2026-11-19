// Menanamkan ikon dan info versi ke dalam YT-Chat-Overlay.exe
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("FileDescription", "YT Chat Overlay")
            .set("ProductName", "YT Chat Overlay");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Gagal menanamkan ikon: {e}");
        }
    }
}
