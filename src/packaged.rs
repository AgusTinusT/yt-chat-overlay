//! Deteksi apakah aplikasi berjalan sebagai paket MSIX (versi Microsoft Store).

use std::sync::OnceLock;

/// True jika berjalan dari paket MSIX (punya package identity).
pub fn is_packaged() -> bool {
    static PACKAGED: OnceLock<bool> = OnceLock::new();
    *PACKAGED.get_or_init(|| {
        use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
        // APPMODEL_ERROR_NO_PACKAGE (15700) = bukan paket. ERROR_INSUFFICIENT_BUFFER (122) = paket.
        let mut len = 0u32;
        let rc = unsafe { GetCurrentPackageFullName(&mut len, std::ptr::null_mut()) };
        rc != 15700
    })
}
