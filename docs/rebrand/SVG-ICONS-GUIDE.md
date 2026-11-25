# Panduan Icon & Animasi SVG Open-Source (Free License)

Dokumen ini berisi pilihan library icon & animasi SVG open-source berlisensi permisif (**MIT**, **no-attribution**) serta cara integrasinya ke arsitektur aplikasi ini.

---

## 1. Konteks Teknis Aplikasi

Aplikasi utama (`crates/poni`) **bukan** aplikasi web/React/Electron.
- `crates/poni-core/src/svgpath.rs` mem-parsing SVG path string (`M L H V C S Q T A Z`) dalam koordinat **24×24** dan mengubahnya menjadi **Direct2D PathGeometry**.
- `crates/poni/src/icons.rs` menyimpan icon sebagai `&str` path data dengan konstanta `STROKE: f32 = 2.0` (round-capped stroke atau non-zero fill).
- Animasi pada UI native dijalankan oleh render loop Direct2D di Rust (`gfx.rs`, `crates/poni-core/src/oyen/`), sehingga **tidak** mengeksekusi JavaScript atau CSS animasi browser di dalam island.

Oleh karena itu, pemilihan aset dibagi dua: **untuk UI Native (Rust/Direct2D)** dan **untuk Web/HTML (`phonedrop/page.html`, `site/`)**.

---

## 2. Rekomendasi Library Icon & Animasi SVG

### A. Untuk UI Native Rust (`crates/poni/src/icons.rs`)

| Library | Lisensi | Format & Kompatibilitas | Kegunaan |
| :--- | :--- | :--- | :--- |
| **Lucide Icons** (`lucide-icons/lucide`) | **MIT** | SVG 24×24, `stroke-width="2"`, round cap/join — **100% cocok** dengan `svgpath.rs` | Icon UI utama (navigasi, media, settings, status) |
| **Tabler Icons** (`tabler/tabler-icons`) | **MIT** | SVG 24×24, `stroke-width="2"` — **100% cocok** dengan `svgpath.rs` | Alternatif/pelengkap icon UI (5.000+ icon) |
| **Material Line Icons (`line-md`)** | **MIT** (tanpa wajib atribusi) | 1.200+ icon 24×24 berbasis stroke; path-nya dapat diekstrak ke `icons.rs` | Icon status & transisi garis |
| **SVG Spinners** (`n3r4zzur0/svg-spinners`) | **MIT** | SVG 24×24 sangat ringan (<400 byte) | Indikator loading (path diekstrak + rotasi/dash di Rust) |

### B. Untuk Halaman Web / HTML / Dokumentasi (`phonedrop/page.html`, `site/`)

| Library | Lisensi | Teknologi | Kegunaan |
| :--- | :--- | :--- | :--- |
| **SVG Spinners** (`n3r4zzur0/svg-spinners`) | **MIT** | Pure SVG + CSS/SMIL (tanpa JS) | Loader ringan di `phonedrop/page.html` (tanpa bundle eksternal) |
| **Material Line Icons (`line-md`)** | **MIT** | Pure SVG + SMIL/CSS (atau via Iconify) | Icon animasi di halaman web lokal atau landing page |
| **Lucide Animated** (`lucide-animated`) | **MIT** | Komponen web + motion | Cocok jika `site/` atau halaman web memakai stack JS modern |
| **AnimateIcons** (`Avijit07x/animateicons`) | **MIT** | 1.500+ icon animasi (React/Vue/Vanilla JS) | Pilihan jika membangun web dashboard pendamping |
| **Anidiagram** (`coolbat/anidiagram`) | **Open Source** | Pure CSS keyframes di dalam `.svg` | Ilustrasi/diagram animasi di README atau docs |

---

## 3. Cara Mengadopsi Icon Baru ke `crates/poni/src/icons.rs`

1. Ambil file `.svg` dari **Lucide** atau **Tabler** (`viewBox="0 0 24 24"`).
2. Gabungkan atribut `d="..."` dari `<path>` menjadi string path tunggal.
   - Jika ada `<circle>`, `<rect>`, atau `<line>`, konversi ke perintah path (`M`, `L`, `A`, `Z`) karena `svgpath.rs` hanya mem-parsing path data (`M L H V C S Q T A Z`).
   - Pastikan setiap sub-figur setelah `Z` dimulai dengan `M` eksplisit (aturan yang tertulis di header `crates/poni/src/icons.rs`).
3. Tambahkan varian ke `enum Icon` dan mapping path-nya di `icons.rs`.
4. Untuk **animasi** pada UI native (misal spinner berputar, ikon berdenyut, atau equalizer):
   - Jangan gunakan timer terus-menerus saat island idle (patuhi aturan hemat CPU `AGENTS.md`).
   - Animasikan transformasi Direct2D (rotasi, skala, opacity) di `gfx.rs` hanya saat komponen sedang aktif/terlihat.

---

## 4. Opsi Mengganti Maskot `Oyen` (`crates/poni-core/src/oyen/`)

Saat ini maskot Oyen digambar melalui kode geometri prosedural di `crates/poni-core/src/oyen/geometry.rs` dan `mark.rs`. Untuk membuatnya mudah dikustomisasi:
1. **Opsi Paling Ringan (Disarankan untuk MVP):** Buat mark/logo sederhana berbasis **SVG path 24×24** (atau beberapa frame ekspresi sederhana seperti *idle*, *active*, *sleep*) yang dirender lewat `svgpath.rs` + animasi posisi/skala ringan di Rust.
2. **Opsi Karakter Kustom:** Pertahankan antarmuka publik modul maskot (misal rename modul `oyen` menjadi `mascot`), lalu isi implementasi frame-nya dengan path SVG karakter baru.
