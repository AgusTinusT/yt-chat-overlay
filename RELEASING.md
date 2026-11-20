# Rilis dan update

Aplikasi didistribusikan lewat dua jalur:

| Jalur | Untuk siapa | Cara update |
|---|---|---|
| **Microsoft Store** (MSIX) | Pengguna biasa. Tanpa peringatan SmartScreen. | Otomatis lewat Store |
| **GitHub Releases** (`.exe` portable) | Yang tidak memakai Store | Aplikasi memberi tahu kalau ada versi baru, lalu pengguna mengunduh sendiri |

Kedua jalur dibuat dari **satu tag** yang sama, jadi versinya selalu sama.

## Setup awal (sekali saja)

1. Daftar akun developer perorangan di [Partner Center](https://partner.microsoft.com/dashboard) (gratis untuk perorangan).
2. Buat aplikasi baru dan reservasi nama **YT Chat Overlay**.
3. Buka **Product management → Product identity**, lalu salin tiga nilai ini ke `packaging/identity.json`:
   - `Package/Identity/Name` → `IdentityName`
   - `Package/Identity/Publisher` → `Publisher` (bentuknya `CN=XXXXXXXX-XXXX-...`)
   - `Package/Properties/PublisherDisplayName` → `PublisherDisplayName`
4. Commit perubahan `identity.json`.

## Setiap rilis versi baru

1. Naikkan `version` di `Cargo.toml` (misalnya `0.2.0`), lalu commit.
2. Push tag:
   ```powershell
   git tag v0.2.0; git push origin main v0.2.0
   ```
3. GitHub Actions otomatis:
   - membuat GitHub Release berisi `.exe`, `.zip`, dan `SHA256SUMS.txt`. Pengguna portable akan melihat "Versi baru tersedia" saat membuka aplikasi;
   - membuat paket MSIX `YT-Chat-Overlay-0.2.0.0-x64.msix` sebagai **artifact**. Lokasinya di tab Actions → run rilis → bagian Artifacts.
4. Di Partner Center, buka aplikasi → **Start update** (atau *Update* pada submission) → **Packages**, upload file `.msix` tadi, lalu isi *What's new* dan klik **Submit**.
5. Microsoft memeriksa paketnya, biasanya 1–3 hari kerja. Setelah lolos, semua pengguna Store mendapat update otomatis.

Aturan versi:
- Versi MSIX = versi Cargo + `.0` (`0.2.0` → `0.2.0.0`). Store mewajibkan angka terakhir `0`.
- Setiap submission harus memakai versi **lebih tinggi** dari sebelumnya. Jangan pakai ulang nomor versi.
- Kalau ada perbaikan darurat, rilis versi patch (`0.2.1`).

## Catatan untuk submission pertama

- **runFullTrust**: aplikasi ini aplikasi desktop biasa (Win32), jadi Partner Center meminta alasan untuk capability ini. Contoh: *"Desktop Win32 app (Rust) that shows a transparent always-on-top YouTube live chat overlay with a tray icon and global hotkeys; packaged as MSIX for Store distribution."*
- Kategori: *Utilities & tools* (atau *Entertainment*).
- Privacy policy URL: `https://github.com/AgusTinusT/yt-chat-overlay#privacy`.
- Pengaturan versi Store tersimpan di `%APPDATA%\YTChatOverlay\yt-chat-overlay.json`. Versi portable tetap menyimpannya di samping `.exe`.
- Versi Store tidak mengecek GitHub untuk update, karena update ditangani Store.

## Tes paket di komputer sendiri (opsional)

Paket MSIX yang belum ditandatangani tidak bisa diinstall dengan dobel klik. Untuk mengetes:

1. Aktifkan **Developer Mode**: Settings → System → For developers.
2. Build lalu daftarkan folder paketnya:
   ```powershell
   cargo build --release
   .\packaging\build-msix.ps1
   Add-AppxPackage -Register "$env:TEMP\ytovl-msix-stage\AppxManifest.xml"
   ```
3. Buka **YT Chat Overlay** dari Start menu. Cek overlay, hotkey, ikon tray, dan apakah pengaturan tersimpan setelah aplikasi ditutup lalu dibuka lagi.
4. Untuk menghapusnya:
   ```powershell
   Get-AppxPackage *YTChatOverlay* | Remove-AppxPackage
   ```

## Nanti: otomatisasi submission

Langkah 4 bisa diotomatiskan dengan [Microsoft Store Developer CLI](https://github.com/microsoft/msstore-cli) di GitHub Actions. Syaratnya, akun Partner Center sudah terhubung ke Microsoft Entra ID (Azure AD) dan kredensialnya disimpan sebagai *secrets* di repo. Ini baru layak dikerjakan setelah submission pertama lolos.
