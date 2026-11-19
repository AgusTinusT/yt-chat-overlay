<p align="center"><img src="assets/logo.svg" width="128" alt="Logo YT Chat Overlay"></p>

# YT Chat Overlay

Overlay live chat YouTube yang ringan dan transparan, tampil di atas game. Klik mouse tembus ke game.

## Download
Ambil `YT-Chat-Overlay.exe` dari halaman **Releases**. Tidak perlu install, cukup jalankan. (Windows 10/11)

> File `.exe` ini belum ditandatangani secara digital, jadi Windows bisa menampilkan peringatan saat pertama kali dibuka. Baca [Peringatan keamanan](#peringatan-keamanan-saat-download--membuka).

## Peringatan keamanan saat download / membuka
File ini aman: source code-nya terbuka, dan `.exe`-nya dibuild otomatis oleh GitHub Actions dari kode di repo ini. Peringatan muncul karena file belum ditandatangani secara digital dan masih jarang diunduh.

| Yang muncul | Solusi |
|---|---|
| Layar biru **"Windows protected your PC"** | Klik **More info**, lalu **Run anyway** |
| **Chrome/Edge** memblokir download | Buka daftar download (Ctrl+J), lalu pilih **Keep** / **Keep anyway** |
| **Windows Defender** menghapus file atau menandainya sebagai virus (misalnya *Wacatac*) | Itu salah deteksi (*false positive*). Pulihkan lewat **Windows Security → Protection history → Restore**, lalu bantu laporkan ke [Microsoft](https://www.microsoft.com/en-us/wdsi/filesubmission) sebagai *Incorrectly detected* |

**Cek keaslian file:** setiap Release menyertakan `SHA256SUMS.txt`. Jalankan perintah ini di PowerShell:
```powershell
Get-FileHash .\YT-Chat-Overlay.exe
```
Kolom *Hash* harus sama persis dengan angka di halaman Release. Kalau berbeda, jangan jalankan file tersebut.

## Cara pakai
1. Jalankan `YT-Chat-Overlay.exe`. Saat pertama kali dibuka, aplikasi langsung masuk **mode edit**.
2. Isi **@namachannel** Anda (disarankan) atau link live, lalu klik **Hubungkan**.
   - Kalau diisi @channel, aplikasi otomatis mencari live yang sedang berjalan dan mengecek ulang tiap 15 detik. Jadi aplikasi boleh dibuka sebelum mulai live.
3. Seret bilah merah untuk memindah overlay, dan seret pojok kanan bawah untuk mengubah ukuran.
4. Klik **Selesai**. Overlay sekarang transparan dan bisa diklik tembus.

## Kontrol
**Ikon di system tray** (pojok kanan bawah taskbar). Tidak perlu keyboard:
- Klik kiri: buka / tutup mode edit
- Klik kanan: menu (tampilkan/sembunyikan chat, mode edit, keluar)

**Hotkey** (bisa diubah di mode edit, lalu klik **Terapkan**):
| Tombol bawaan | Fungsi |
|---|---|
| Ctrl+Alt+C | Tampilkan / sembunyikan chat |
| Ctrl+Alt+E | Buka / tutup mode edit |

Format hotkey: `Ctrl+Alt+C`, `Alt+Shift+Z`, `Ctrl+Shift+F9`, dan sejenisnya.

Aplikasi hanya bisa berjalan satu kali. Kalau dibuka lagi, jendela yang sudah berjalan langsung masuk mode edit.

## Catatan
- Set game ke **Borderless / Windowed Fullscreen**. Di mode *Exclusive Fullscreen*, tidak ada jendela yang bisa tampil di atas game.
- Pengaturan disimpan di `yt-chat-overlay.json`, di folder yang sama dengan `.exe`.
- Warna: nama kuning = pemilik channel, biru = moderator, hijau = member. Latar biru = pertanyaan, latar oranye = Super Chat.

## Supaya overlay tidak ikut terekam OBS
Pakai source **Game Capture** di OBS, bukan Display Capture. Game Capture hanya merekam jendela game, jadi overlay hanya terlihat oleh Anda dan penonton tidak melihat chat dobel.

Kalau terpaksa memakai Display Capture, aktifkan opsi **"Sembunyikan dari Display Capture OBS"** di mode edit (bawaannya mati). Baca dulu catatan anti-cheat di bawah.

## Anti-cheat
Aplikasi ini **tidak** menyuntikkan kode ke game, tidak membaca memori game, tidak meng-hook DirectX, dan tidak mensimulasikan input. Aplikasi ini hanya jendela biasa yang mengambil chat dari YouTube.

Meski begitu, perlu diketahui:
- Jendela transparan yang selalu di atas dan bisa diklik tembus mirip dengan teknik yang dipakai cheat ESP eksternal. Beberapa anti-cheat ketat (Vanguard, FACEIT, dan lain-lain) bisa saja menandainya.
- Opsi "Sembunyikan dari Display Capture" memakai fungsi Windows yang juga dipakai cheat untuk bersembunyi dari screenshot anti-cheat. Karena itu bawaannya **mati**.

Untuk game kompetitif dengan anti-cheat kernel, sebaiknya tutup overlay saat bermain. Gunakan dengan risiko sendiri.

## Build ulang
```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\ytovl"; cargo build --release
```
(Folder target dibuat pendek karena path yang terlalu panjang membuat linker Windows gagal.)

## Rilis versi baru
Naikkan `version` di `Cargo.toml`, commit, lalu push tag:
```powershell
git tag v0.1.1; git push origin v0.1.1
```
GitHub Actions akan build `.exe` dan membuat Release otomatis.

## Logo
Sumbernya ada di `assets/logo.svg`. Setelah mengubahnya, buat ulang PNG dan `icon.ico` dengan:
```powershell
cargo run --example make_icon
```

## Lisensi
MIT
