# PRD: Koreksi Palet Pagi & Pemandangan Landscape Dinamis (`Horizon v2`)

## 1. Latar Belakang & Koreksi

Berdasarkan evaluasi implementasi awal `Theme::Horizon`:
1. **Palet Pagi terlalu kuning polos:** Warna pagi perlu mengikuti spektrum gradien langit fajar berlapis pada gambar referensi: biru langit lembut di puncak, memudar ke krem pucat, kuning keemasan lembut, persik/jingga hangat, merah muda koral, hingga ungu-plum di siluet pegunungan bawah.
2. **Latar Pemandangan di Halaman Island Terbuka (*Expanded Page*):** Selain pill tertutup, area *expanded page* pada theme `Horizon` membutuhkan pemandangan *landscape* dinamis yang terinspirasi dari sketsa kabin pegunungan:
   - **Proporsi 20–30% bawah:** Siluet/vektor **gunung berlapis**, **hutan pinus**, **kabin/rumah** (dengan jendela, pintu, dan cerobong asap), serta **garis tepi danau/refleksi air** di bagian bawah halaman.
   - **Proporsi 70–80% atas:** Ruang **langit yang luas dan bersih** agar seluruh teks, tombol, dan konten halaman (*Media*, *Today*, *Prayer*, *Tasks*, *Settings*, dll.) tetap terbaca jelas tanpa tertutup detail pemandangan.

---

## 2. Koreksi Palet Gradient Multi-Stop (5 Fase)

Agar warna pagi dan fase lainnya terasa natural (tidak datar), `gfx.rs` mendukung *multi-stop vertical linear gradient* (`fill_linear_gradient_stops`):

| Fase | Stop Gradient Langit (Atas `0.0` → Bawah `1.0`) | Kontras Teks (`ink`) |
| :--- | :--- | :--- |
| **Pagi** (*Morning*) | `0.00`: `#86D7EA` (biru fajar) → `0.25`: `#F4F7E6` (krem) → `0.48`: `#ECD97A` (kuning lembut) → `0.70`: `#FAA15D` (persik) → `0.86`: `#E88078` (koral) → `1.00`: `#9C6582` (plum gunung) | Gelap (`#1D1A22`) di area atas/tengah dengan *scrim* halus jika diperlukan, agar teks tetap tajam |
| **Siang** (*Day*) | `0.00`: `#3A8EE6` → `0.55`: `#6EC3F4` → `1.00`: `#BDE4F7` | Putih/Gelap adaptif dengan keterbacaan tinggi |
| **Sore** (*Dusk*) | `0.00`: `#2E234E` → `0.45`: `#9B4D6A` → `0.78`: `#E87A5D` → `1.00`: `#F5B06B` | Putih (`#FFFFFF`) |
| **Malam** (*Night*) | `0.00`: `#0B1328` → `0.55`: `#162647` → `1.00`: `#223A63` | Putih lembut (`#F5F7FA`) |
| **Tengah Malam** (*Midnight*) | `0.00`: `#04060E` → `0.60`: `#0A1022` → `1.00`: `#131E3A` | Putih lembut (`#E8EEF8`) |

---

## 3. Dinamika Elemen Pemandangan Per Fase

Seluruh elemen disinkronkan dengan waktu lokal & jadwal **Prayer Times** (`terbit`, `dzuhur`, `ashar`, `maghrib`, `isya`):

| Fase | Langit & Benda Langit | Gunung, Pinus & Danau | Kabin & Lampu Jendela |
| :--- | :--- | :--- | :--- |
| **Pagi** | **Matahari terbit dari balik gunung** (cakram hangat muncul di celah puncak gunung) | Gunung berlapis bernuansa ungu-plum & pinus siluet gelap lembut | Lampu mati, kepulan asap tipis opsional |
| **Siang** | **Tanpa matahari** — mengandalkan kesan warna langit siang yang cerah | Gunung, pinus, dan refleksi danau bernuansa siang cerah | Lampu mati |
| **Sore** | **Tanpa matahari** — mengandalkan kesan warna senja + **bintang jatuh sangat jarang** (15–25 menit) | Siluet gunung & pinus menghangat/gelap | **Lampu kabin mulai menyala** (pendar kuning hangat lembut di jendela) |
| **Malam** | **Bulan** + **bintang titik multi-ukuran** + **bintang jatuh cukup sering** (2–4 menit) | Siluet gelap kontras + pantulan cahaya bulan/lampu di air danau | **Lampu kabin menyala terang** (`#FFD875` + glow lembut) |
| **Tengah Malam** | **Bintang titik multi-ukuran** + **bintang jatuh sedikit lebih sering** (1–2.5 menit) | Siluet malam pekat | Lampu kabin redup/hangat atau padam larut malam |

---

## 4. Aturan Tata Letak & Performa (`AGENTS.md`)

1. **Keterbacaan Konten Halaman:**
   - Pemandangan berada di 20–30% terbawah *expanded page* dengan opacity terkontrol agar tidak mengganggu kontrol UI bagian bawah (tombol/tab footer).
   - Pada mode *collapsed pill* (saat tertutup di atas layar), pemandangan disederhanakan menjadi siluet rendah + bintang/bintang jatuh agar jam dan judul lagu tetap bersih.
2. **Zero Extra Overhead:**
   - Seluruh pemandangan digambar menggunakan path vektor Direct2D native (`Canvas::path`, `fill_circle`, `fill_linear_gradient_stops`) — tanpa aset bitmap eksternal dan tanpa timer animasi baru saat idle (kecuali durasi singkat `0.7s` saat bintang jatuh melintas).
   - Bintang jatuh tetap patuh pada `App::locked`, `App::display_off`, `st.power.saver`, dan `fullscreen_hidden`.

---

## 5. Rencana Implementasi Per Fase

### Fase 1 — Palet Multi-Stop & State Pencahayaan Kabin (`crates/poni-core`)
- Perbarui `crates/poni-core/src/horizon.rs`:
  - Ganti `HorizonPalette` dari 2-warna (`top`, `bottom`) menjadi `stops: [(f32, Color); 6]` (atau `Vec`/fixed array) + interpolasi warna antar-stop saat transisi fase.
  - Terapkan palet pagi 6-stop sesuai gambar referensi (`#86D7EA` → `#F4F7E6` → `#ECD97A` → `#FAA15D` → `#E88078` → `#9C6582`).
  - Tambah properti `cabin_light: f32` (`0.0` di Pagi/Siang, `0.45` di Sore, `1.0` di Malam, `0.25` di Tengah Malam) dan `has_sun: bool` (`true` hanya di Pagi saat terbit; `false` di Siang & Sore) serta `has_moon: bool` (`true` di Malam).
- Tambah unit test untuk interpolasi multi-stop, `cabin_light`, dan `has_sun`/`has_moon`.

### Fase 2 — Multi-Stop Gradient Brush di Direct2D (`crates/poni/src/gfx.rs`)
- Tambah `Canvas::fill_linear_gradient_stops(&mut self, r: Rect, stops: &[(f32, Color)])` di `crates/poni/src/gfx.rs` menggunakan `ID2D1LinearGradientBrush` tanpa remapping `Look::paint`.
- Verifikasi kompilasi dan rendering gradient halus.

### Fase 3 — Komposisi Pemandangan Landscape 20–30% Bawah (`crates/poni/src/pill/horizon.rs`)
- Buat fungsi `draw_landscape(c: &mut Canvas, st: &AppState, rect: Rect, expanded: bool)`:
  - **Gunung berlapis (2–3 layer):** Path poligon gunung di sisi kanan-tengah dalam area 20–30% terbawah.
  - **Hutan pinus & pohon tinggi kiri:** Path siluet pohon pinus bertingkat di sisi kiri (di belakang/samping kabin) dan deretan pinus jauh di kaki gunung kanan.
  - **Kabin/Rumah & Jendela Lampu:** Bentuk kabin kecil dengan atap miring, cerobong asap, dan jendela yang memancarkan cahaya hangat (`cabin_light > 0`) beserta pendar lembut di sekitar jendela.
  - **Danau & Refleksi:** Garis tepi air tipis di bagian bawah kanan dengan pantulan garis cahaya bulan/lampu kabin.
  - **Matahari Terbit (khusus Pagi):** Cakram matahari terbit di balik celah gunung; **Siang & Sore:** tidak menggambar matahari; **Malam:** bulan sabit/purnama + bintang; **Tengah Malam:** bintang + bintang jatuh.

### Fase 4 — Integrasi ke Expanded Page & Penyesuaian Kontras UI (`crates/poni/src/app.rs` & `look.rs`)
- Panggil `pill::horizon::draw_sky` / `draw_landscape` baik saat `collapsed` maupun `Mode::Expanded(_)` di `crates/poni/src/app.rs` (di-clip ke outline notch).
- Pastikan warna teks, wash card, dan border pada fase **Pagi** dan **Siang** tetap kontras di atas langit multi-stop (sesuaikan `Look` untuk `Theme::Horizon` di `crates/poni-core/src/look.rs`).
- Perbarui preview card `Theme::Horizon` di `crates/poni/src/settings_window.rs` agar menampilkan gradient fajar multi-stop + siluet gunung & kabin mini.

### Fase 5 — Verifikasi & Uji Menyeluruh
- Jalankan `cargo test --workspace` dan `cargo build -p poni --release`.
- Verifikasi keterbacaan UI di seluruh 5 fase waktu (*Pagi*, *Siang*, *Sore*, *Malam*, *Tengah Malam*).
