# PRD: Theme Dinamis Berbasis Waktu (`Sky` / `Horizon`)

## 1. Latar Belakang & Tujuan

Saat ini aplikasi memiliki lima theme (`Classic`, `Oyen`, `Glass`, `Pill`, `Diorama`). `Diorama` sudah memiliki konsep pergantian siang/malam berbasis pixel-art (`studio-diorama`), namun belum ada theme vektor/gradient modern yang:
1. Mengubah **warna latar island secara dinamis dengan gradient** sesuai 5 fase waktu (**Pagi**, **Siang**, **Sore**, **Malam**, **Tengah Malam**).
2. Menampilkan **elemen ikon langit vektor** yang ringan dan ekspresif (bintang titik berbagai ukuran, bintang jatuh, bulan, matahari, matahari terbit di gunung, dan matahari terbenam di horizon) di dalam Direct2D tanpa asset bitmap besar.

**Tujuan:** Menghadirkan theme baru (usulan nama: **`Sky`**) yang mengikuti waktu lokal dan jadwal terbit/terbenam matahari (terintegrasi dengan Prayer Times yang sudah built-in), tetap sangat ringan pada CPU/memori/ukuran biner sesuai prinsip `AGENTS.md`.

---

## 2. Lima Fase Waktu & Palet Gradient

Penentuan fase menggunakan menit lokal terhadap waktu `sunrise` (terbit) dan `sunset` (maghrib/terbenam) dari `AppState::prayer` (fallback default: `06:00` dan `18:00` jika lokasi belum tersedia):

| Fase | Rentang Waktu (Default / Relatif) | Nuansa Gradient Island (Top → Bottom / Diagonal) | Kontras Teks & Ikon | Elemen Visual Utama |
| :--- | :--- | :--- | :--- | :--- |
| **1. Pagi** (*Morning*) | `sunrise - 15m` s/d `09:30` | Gradient **kuning cerah keemasan** khas pagi hari (`#FFE27A` → `#FFB84D` / aksen hangat `#FFF3B0`) | Teks gelap (`#1A1A1A`) agar terbaca jelas di atas kuning cerah | **Matahari terbit di gunung** (siluet gunung + matahari naik + sinar tipis) |
| **2. Siang** (*Day*) | `09:30` s/d `15:30` | Gradient biru langit cerah (`#56CCF2` → `#2F80ED`) | Teks putih / gelap adaptif sesuai luminansi | **Matahari** bersinar di langit siang |
| **3. Sore** (*Dusk / Sunset*) | `15:30` s/d `sunset + 20m` | Gradient jingga senja ke ungu hangat (`#FF7E5F` → `#FEB47B` / `#8E44AD`) | Teks putih (`#FFFFFF`) | **Matahari terbenam di horizon** (garis cakrawala + setengah cakram matahari + pendar senja) |
| **4. Malam** (*Night*) | `sunset + 20m` s/d `23:30` | Gradient biru malam dalam (`#0F2027` → `#203A43` → `#2C5364`) | Teks putih (`#F5F7FA`) | **Bulan** (fase sabit/purnama) + **bintang titik berbagai ukuran** |
| **5. Tengah Malam** (*Midnight*) | `23:30` s/d `sunrise - 15m` | Gradient gelap pekat kosmik (`#05070E` → `#0D1326`) | Teks putih lembut (`#E2E8F0`) | **Bintang titik berbagai ukuran** + **bintang jatuh** (*shooting star*) berkala |

> **Transisi Halus:** Antara batas fase (jendela ±15 menit), warna gradient di-interpolasi secara linear/smoothstep sehingga perubahan warna tidak melompat kasar saat menit berganti.

---

## 3. Elemen Ikon & Komposisi Visual (Vektor Direct2D)

Seluruh elemen digambar secara prosedural/vektor di Direct2D (`crates/poni/src/gfx.rs` + `icons.rs`) atau path 24×24 agar tajam di semua skala DPI tanpa membengkakkan `poni.exe`:

1. **Bintang (Titik Berbagai Ukuran):**
   - Array posisi deterministik (pseudo-random berbasis seed tetap) dengan 3 ukuran radius (`0.8`, `1.3`, `2.0` DIP) dan variasi opacity (`0.35` – `0.95`).
   - Di fase *Malam* & *Tengah Malam*, bintang tampil statis saat island idle (0 frame redraw tambahan); kilau ringan hanya berjalan saat island sedang hover/terbuka.
2. **Bintang Jatuh (*Shooting Star*):**
   - Garis ekor gradient (`LinearGradientBrush` transparan → putih) + kepala titik terang yang melintas diagonal selama `~0.6s`.
   - **Aturan Hemat Daya (`AGENTS.md`):** Tidak memicu timer terus-menerus saat idle; dipicu hanya pada momen tertentu (misal saat island baru muncul, saat hover, atau sekali per interval menit yang jarang saat layar aktif, dan berhenti total saat `locked`, `display_off`, atau `power.saver`).
3. **Bulan:**
   - Bulan sabit / fase bulan berbasis hari kalender (`days_from_civil`) menggunakan kombinasi path lingkaran atau `Icon::Moon` di `icons.rs`.
4. **Matahari:**
   - Cakram matahari dengan pendar radial halus (*glow* transparan) dan/atau sinar vektor.
5. **Matahari Terbit di Gunung (*Sunrise over Mountain*):**
   - Komposisi 2–3 lapis siluet gunung (poligon/path rendah di sisi bawah pill) dengan cakram matahari muncul di celah puncak gunung pada fase *Pagi*.
6. **Matahari Terbenam di Horizon (*Sunset on Horizon*):**
   - Garis horizon tipis di sepertiga bawah pill, dengan setengah cakram matahari terpotong rapi di atas garis horizon dan refleksi pendar hangat pada fase *Sore*.

---

## 4. Kebutuhan Non-Fungsional (`AGENTS.md`)

- **Zero Idle Overhead:** Saat island diam (*idle*), theme `Sky` hanya me-redraw saat minute-tick (yang sudah ada untuk jam), perubahan state media/notifikasi, atau transisi hover/buka kartu.
- **Render Ringan:** Menggunakan primitive Direct2D native (`FillRoundedRectangle` dengan `ID2D1LinearGradientBrush`, `FillEllipse`, `DrawLine`, dan `PathGeometry`), tanpa menambah dependency crate eksternal maupun aset gambar raster.
- **Keterbacaan (Accessibility):** Kontras warna teks jam, judul lagu, dan ikon status pada setiap fase (terutama *Pagi* yang berwarna kuning cerah) wajib memenuhi rasio keterbacaan yang baik.
- **Lintas Platform (Opsional / Parity):** Model penentuan fase waktu & palet warna diletakkan di `crates/poni-core` (pure Rust, dapat di-unit-test di OS mana saja), sedangkan renderer Direct2D berada di `crates/poni`.

---

## 5. Rencana Implementasi Per Fase

### Fase 1 — Model Waktu, Palet & Kontrak Core (`crates/poni-core`)
- Tambah varian `Theme::Sky` pada `crates/poni-core/src/settings.rs` (beserta serialisasi `"sky"`).
- Buat modul baru `crates/poni-core/src/sky.rs`:
  - `pub enum DayPhase { Morning, Day, Dusk, Night, Midnight }`
  - `pub struct SkyPalette { top: Color, bottom: Color, accent: Color, ink: Color, ink_dim: Color }`
  - Fungsi murni `phase_at(minute: f32, sunrise: f32, sunset: f32) -> DayPhase` dan `palette_at(minute: f32, sunrise: f32, sunset: f32) -> SkyPalette` dengan interpolasi antar fase.
  - Generator posisi bintang deterministik & parameter elemen langit (posisi matahari/bulan).
- Tambah `Look` untuk `Theme::Sky` di `crates/poni-core/src/look.rs` (atau dynamic look resolver).
- **Verifikasi Fase 1:** Unit test lengkap di `poni-core` untuk batas waktu 5 fase, interpolasi warna, dan serialisasi `Settings`.

### Fase 2 — Dukungan Gradient & Primitive Langit di Direct2D (`crates/poni/src/gfx.rs`)
- Tambah helper `Canvas::fill_linear_gradient_capsule` / `fill_linear_gradient_rect` di `crates/poni/src/gfx.rs` menggunakan `ID2D1LinearGradientBrush` (cache atau buat ringan per frame aktif).
- Tambah/lengkapi ikon 24×24 di `crates/poni/src/icons.rs` untuk kebutuhan UI picker/ikon jika diperlukan (matahari, bulan, gunung terbit, horizon terbenam, bintang).
- **Verifikasi Fase 2:** `cargo check -p poni` lulus tanpa warning baru dan capsule gradient ter-clip rapi mengikuti bentuk pill/island.

### Fase 3 — Renderer Theme `Sky` (`crates/poni/src/pill/sky.rs` & `pill/mod.rs`)
- Buat `crates/poni/src/pill/sky.rs` untuk menggambar latar gradient + 6 elemen langit (matahari terbit di gunung, matahari siang, matahari terbenam di horizon, bulan, bintang multi-ukuran, dan bintang jatuh) pada pill tertutup maupun kartu terbuka.
- Integrasikan ke `crates/poni/src/pill/mod.rs`, `crates/poni/src/pill/status.rs`, dan `crates/poni/src/activities.rs` (menggunakan `ink` & `ink_dim` dinamis agar teks jam/lagu tetap kontras di atas background kuning pagi maupun gelap malam).
- Terapkan aturan hemat daya (`App::locked`, `App::display_off`, `st.power.saver`) untuk animasi bintang jatuh.
- **Verifikasi Fase 3:** Render test / unit test untuk semua 5 fase waktu dan seluruh `IdleStyle`.

### Fase 4 — Integrasi Settings, Tray, & Self-Test
- Daftarkan `Theme::Sky` di:
  - `crates/poni/src/settings_window.rs` (kartu theme baru + preview mini gradient & ikon langit)
  - `crates/poni/src/pages/settings.rs`
  - `crates/poni/src/tray.rs` (submenu Theme)
  - `crates/poni/src/app.rs` & `crates/poni/src/selftest.rs`
- **Verifikasi Fase 4:** Seluruh test `pages`, `settings_window`, `tray`, dan `activities` lulus.

### Fase 5 — Audit Performa, Biaya Resource, & Polish
- Uji visual di semua 5 fase waktu (Pagi, Siang, Sore, Malam, Tengah Malam) dan transisinya.
- Pastikan tidak ada redraw loop saat idle dan ukuran biner `poni.exe` tetap minimal.
- Jalankan `cargo test --workspace` dan `cargo build -p poni --release`.

---

## 6. Keputusan yang Perlu Dikonfirmasi Sebelum Eksekusi Fase 1
1. **Nama Theme:** Menggunakan `"Sky"` (atau `"Horizon"` / nama lain pilihan Anda)?
2. **Bentuk Island:** Apakah theme ini mengadopsi bentuk **mengambang (`Pill`)** di bawah tepi atas layar (seperti `Pill` dan `Diorama`), atau menempel di tepi atas layar (seperti `Classic`)?
3. **Kapan Bintang Jatuh Muncul:** Hanya saat hover/membuka island (paling hemat baterai), atau juga sesekali otomatis saat malam/tengah malam ketika layar aktif?
