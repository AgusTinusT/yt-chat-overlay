<p align="center"><img src="assets/logo.svg" width="128" alt="Logo YT Chat Overlay"></p>

# YT Chat Overlay

Overlay live chat YouTube yang ringan dan transparan, tampil di atas game. Klik mouse tembus ke game.

## Download
- **Microsoft Store** (segera hadir): tanpa peringatan keamanan, dan update otomatis.
- **Portable**: ambil `YT-Chat-Overlay.exe` dari halaman **Releases**. Tidak perlu install, cukup jalankan. (Windows 10/11) Aplikasi memberi tahu kalau ada versi baru.

> File `.exe` portable belum ditandatangani secara digital, jadi Windows bisa menampilkan peringatan saat pertama kali dibuka. Baca [Peringatan keamanan](#peringatan-keamanan-saat-download--membuka).

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
1. Jalankan `YT-Chat-Overlay.exe`. Saat pertama kali dibuka, aplikasi langsung masuk **mode edit**. Mode edit punya beberapa tab yang diikuti berurutan:
2. **1. Channel**: isi **@namachannel** Anda (disarankan) atau link live, lalu klik **Connect**.
   - Kalau diisi @channel, aplikasi otomatis mencari live yang sedang berjalan dan mengecek ulang tiap 15 detik. Jadi aplikasi boleh dibuka sebelum mulai live.
3. **2. Position**: klik pojok pada gambar layar (misalnya *Top right*). Overlay langsung menempel di pojok itu, dan chat mulai dari arah yang sesuai. Cara lain: seret panel ke mana saja, lalu seret pojok kanan bawah untuk mengubah ukuran.
4. **3. Look**: atur ukuran huruf, berapa lama pesan tampil, dan kata yang disorot. Kotak **Chat preview** menunjukkan tampilan chat-nya.
5. Klik **Done** di kanan atas. Overlay sekarang transparan dan bisa diklik tembus.

Hotkey, bahasa, dan opsi lain ada di tab **More**.

## Kontrol
**Ikon di system tray** (pojok kanan bawah taskbar). Tidak perlu keyboard:
- Klik kiri: buka / tutup mode edit
- Klik kanan: menu (tampilkan/sembunyikan chat, mode edit, bersihkan chat, keluar)

**Hotkey** (bisa diubah di mode edit, lalu klik **Terapkan**):
| Tombol bawaan | Fungsi |
|---|---|
| Ctrl+Alt+C | Tampilkan / sembunyikan chat |
| Ctrl+Alt+E | Buka / tutup mode edit |

Format hotkey: `Ctrl+Alt+C`, `Alt+Shift+Z`, `Ctrl+Shift+F9`, dan sejenisnya.

Aplikasi hanya bisa berjalan satu kali. Kalau dibuka lagi, jendela yang sudah berjalan langsung masuk mode edit.

## Catatan
- Bahasa bawaan aplikasi adalah **English**. Untuk mengubahnya ke Bahasa Indonesia, buka mode edit lalu pilih **Language / Bahasa**.
- Set game ke **Borderless / Windowed Fullscreen**. Di mode *Exclusive Fullscreen*, tidak ada jendela yang bisa tampil di atas game.
- Pengaturan disimpan di `yt-chat-overlay.json`. Versi portable menyimpannya di folder yang sama dengan `.exe`. Versi Store menyimpannya di `%APPDATA%\YTChatOverlay`.
- Warna: nama kuning = pemilik channel, biru = moderator, hijau = member. Latar biru = pertanyaan, latar oranye = Super Chat, latar ungu = pesan yang menyebut kata dari **Sorot kata** atau @channel Anda.
- Supaya pesan tidak pernah hilang, geser slider **Pesan hilang setelah** ke kiri sampai muncul *Tidak pernah*. Pesan lama tetap dihapus kalau jumlahnya melebihi **Jumlah pesan maksimal**.

## Supaya overlay tidak ikut terekam OBS
Pakai source **Game Capture** di OBS, bukan Display Capture. Game Capture hanya merekam jendela game, jadi overlay hanya terlihat oleh Anda dan penonton tidak melihat chat dobel.

Kalau terpaksa memakai Display Capture, aktifkan opsi **"Sembunyikan dari Display Capture OBS"** di mode edit (bawaannya mati). Baca dulu catatan anti-cheat di bawah.

## Anti-cheat
Aplikasi ini **tidak** menyuntikkan kode ke game, tidak membaca memori game, tidak meng-hook DirectX, dan tidak mensimulasikan input. Aplikasi ini hanya jendela biasa yang mengambil chat dari YouTube.

Meski begitu, perlu diketahui:
- Jendela transparan yang selalu di atas dan bisa diklik tembus mirip dengan teknik yang dipakai cheat ESP eksternal. Beberapa anti-cheat ketat (Vanguard, FACEIT, dan lain-lain) bisa saja menandainya.
- Opsi "Sembunyikan dari Display Capture" memakai fungsi Windows yang juga dipakai cheat untuk bersembunyi dari screenshot anti-cheat. Karena itu bawaannya **mati**.

Untuk game kompetitif dengan anti-cheat kernel, sebaiknya tutup overlay saat bermain. Gunakan dengan risiko sendiri.

## Code signing policy
> Status: file `.exe` di Releases **belum ditandatangani secara digital**. Untuk memastikan keaslian file, gunakan checksum SHA-256 (lihat [Peringatan keamanan](#peringatan-keamanan-saat-download--membuka)).

- **Committers dan reviewers:** [AgusTinusT](https://github.com/AgusTinusT)
- **Approvers:** [AgusTinusT](https://github.com/AgusTinusT)
- Setiap file rilis dibuild otomatis oleh GitHub Actions dari source code di repo ini. Setiap rilis disetujui secara manual oleh approver.
- Kalau nanti file sudah ditandatangani, hanya file yang dibuild dari repo ini yang akan ditandatangani.

## Privacy
This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.

Dalam bahasa Indonesia:
- Aplikasi hanya terhubung ke **youtube.com** untuk membaca live chat publik dari channel atau link yang Anda isi sendiri.
- Versi portable mengecek versi terbaru ke **api.github.com** sekali setiap kali dibuka. Pengecekan ini tidak mengirim data pribadi dan bisa dimatikan di mode edit. Versi Store tidak melakukannya.
- Tidak ada login, tidak ada akun, tidak ada telemetri atau analitik, dan tidak ada data yang dikirim ke server lain.
- Pengaturan hanya disimpan di komputer Anda, di file `yt-chat-overlay.json`.

## Build ulang
```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\ytovl"; cargo build --release
```
(Folder target dibuat pendek karena path yang terlalu panjang membuat linker Windows gagal.)

## Rilis versi baru
Naikkan `version` di `Cargo.toml`, commit, lalu push tag. GitHub Actions akan build `.exe`, membuat Release, dan membuat paket MSIX untuk Microsoft Store. Langkah lengkapnya, termasuk cara update di Store, ada di [RELEASING.md](RELEASING.md).

## Logo
Sumbernya ada di `assets/logo.svg`. Setelah mengubahnya, buat ulang PNG dan `icon.ico` dengan:
```powershell
cargo run --example make_icon
```

## Lisensi
MIT
