# YT Chat Overlay

Overlay live chat YouTube yang ringan dan transparan, tampil di atas game. Klik mouse tembus ke game.

## Download
Ambil `YT-Chat-Overlay.exe` dari halaman **Releases**. Tidak perlu install, cukup jalankan. (Windows 10/11)

## Cara pakai
1. Jalankan `YT-Chat-Overlay.exe`. Saat pertama kali dibuka, aplikasi langsung masuk **mode edit**.
2. Isi **@namachannel** Anda (disarankan) atau link live, lalu klik **Hubungkan**.
   - Kalau diisi @channel, aplikasi otomatis mencari live yang sedang berjalan dan mengecek ulang tiap 15 detik. Jadi aplikasi boleh dibuka sebelum mulai live.
3. Seret bilah merah untuk memindah overlay, dan seret pojok kanan bawah untuk mengubah ukuran.
4. Klik **Selesai**. Overlay sekarang transparan dan bisa diklik tembus.

## Hotkey
| Tombol | Fungsi |
|---|---|
| Ctrl+Shift+F9 | Tampilkan / sembunyikan overlay |
| Ctrl+Shift+F10 | Buka / tutup mode edit (atur posisi, ukuran, pengaturan, keluar) |

## Catatan
- Set game ke **Borderless / Windowed Fullscreen**. Di mode *Exclusive Fullscreen*, tidak ada jendela yang bisa tampil di atas game.
- Opsi **"Sembunyikan dari OBS"** (aktif secara bawaan) membuat overlay tidak ikut terekam *Display Capture*, jadi penonton tidak melihat chat dobel.
- Pengaturan disimpan di `yt-chat-overlay.json`, di folder yang sama dengan `.exe`.
- Warna: nama kuning = pemilik channel, biru = moderator, hijau = member. Latar biru = pertanyaan, latar oranye = Super Chat.

## Build ulang
```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\ytovl"; cargo build --release
```
(Folder target dibuat pendek karena path yang terlalu panjang membuat linker Windows gagal.)

## Rilis versi baru
Naikkan `version` di `Cargo.toml`, commit, lalu push tag:
```powershell
git tag v0.1.0; git push origin v0.1.0
```
GitHub Actions akan build `.exe` dan membuat Release otomatis.

## Lisensi
MIT
