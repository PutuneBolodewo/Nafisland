# Rencana Teknis & Arsitektur Kustomisasi

Dokumen ini menjelaskan langkah teknis untuk mengubah fork dari kondisi saat ini menjadi basis kode yang mudah dikustomisasi (*white-label*).

---

## Fase 0 — Penyelarasan Branch Kerja

Saat ini `main` sudah tersinkronisasi dengan `lucacadalora/oyenza-studio` v0.2.30 (`0ab10d1`), sedangkan working tree berada di branch `develop` (basis lama v0.2.23) dengan file lokal `.cargo/config.toml.bak` dan `docs/brand/tupat-*`.

**Langkah:**
1. Amankan perubahan lokal di `develop` (commit sementara di `develop` atau pindahkan aset `tupat-*` ke folder cadangan).
2. Buat branch kerja baru dari `main`, misalnya `feat/rebrand-base`:
   ```powershell
   git checkout -b feat/rebrand-base main
   ```
3. Ambil file konfigurasi yang masih relevan (misal `.cargo/config.toml` jika diperlukan untuk toolchain lokal).

---

## Fase 1 — Modul Konfigurasi Brand Terpusat

Buat `crates/poni-core/src/brand.rs` (atau baca dari file `brand.toml` saat kompilasi via `build.rs` jika ingin kustomisasi tanpa mengedit kode Rust):

```rust
// Contoh struktur konstanta terpusat di crates/poni-core/src/brand.rs
pub const APP_NAME: &str = "Nafisland";
pub const APP_SHORT_NAME: &str = "Nafisland";
pub const VENDOR_NAME: &str = "Nama Vendor Anda";
pub const COPYRIGHT: &str = "Copyright (C) 2026 Nama Vendor Anda";
pub const SUPPORT_EMAIL: &str = "support@domain-anda.com";

// Endpoint (set None atau arahkan ke server sendiri)
pub const UPDATE_FEED_URL: Option<&str> = None;
pub const DROP_SERVER_URL: &str = "https://drop.domain-anda.com";
pub const PLACES_BASE_URL: &str = "https://cdn.domain-anda.com/places/1/";

// Identifier runtime Windows / macOS
pub const EXE_NAME: &str = "nafisland.exe";
pub const MUTEX_NAME: &str = "Local\\Nafisland.Island";
pub const MAC_BUNDLE_ID: &str = "com.vendoranda.nafisland";
```

**Keuntungan pendekatan `brand.toml` + `build.rs`:**
- `crates/poni/build.rs` dapat menghasilkan file resource `.rc` dan menyuntikkan variabel environment kompilasi (`env!("BRAND_APP_NAME")`) ke Rust maupun metadata `.exe` secara otomatis dari satu file `brand.toml`.

---

## Fase 2 — Refactor Referensi Tersebar ke Modul `brand`

1. Ganti seluruh string literal `"Oyenza Studio"`, `"Oyen"`, `"Jatevo Labs"`, dan email/URL di `crates/poni-core` dan `crates/poni` agar merujuk ke `poni_core::brand::*`.
2. Ubah `crates/poni-core/src/phonedrop/page.html` menggunakan placeholder (misal `{{APP_NAME}}`) yang di-replace saat runtime atau kompilasi.
3. Perbarui `installer/poni.iss` dan `installer/poni.nsi` agar membaca nama produk, publisher, dan nama output binary dari variabel/konfigurasi yang konsisten.

---

## Fase 3 — Penggantian Aset Visual & Maskot

1. Ganti `crates/poni/res/poni.ico` dengan icon aplikasi baru (multi-resolution: 16, 32, 48, 256 px).
2. Bersihkan `docs/brand/` dari aset `oyen-*` dan isi dengan aset brand baru (atau gunakan `docs/brand/tupat-*` yang sudah disiapkan sebagai dasar).
3. Refaktor `crates/poni-core/src/oyen/` menjadi `crates/poni-core/src/mascot/` dengan implementasi SVG path 24×24 atau geometri baru.
4. Perbarui `crates/poni/src/icons.rs` dengan icon dari **Lucide** / **Tabler** / **`line-md`** (MIT) sesuai `SVG-ICONS-GUIDE.md`.

---

## Fase 4 — Isolasi Layanan Eksternal Oyenza

1. **Auto-Update (`update.rs`):** Jika belum memiliki server update & key penandatangan ECDSA P-256 (`data/update-keys.txt`), nonaktifkan pemeriksaan update otomatis saat `UPDATE_FEED_URL` bernilai `None`.
2. **Akun & Pro (`oyenza.rs`):** Pisahkan fitur yang bergantung pada API autentikasi Oyenza agar tidak melakukan request ke server pengembang asli.
3. **World Places (`prayer/world.rs`) & PairDrop (`pairdrop/mod.rs`):** Sediakan fallback atau konfigurasi URL sendiri.

---

## Fase 5 — Verifikasi & Gate Rilis

Setelah setiap fase selesai, jalankan pemeriksaan berikut:

```powershell
# 1. Pastikan tidak ada watermark tersisa di kode produksi
git grep -n -i -E "oyenza|oyen|jatevo" -- crates/ installer/

# 2. Jalankan unit test workspace
cargo test

# 3. Uji build Windows release
cargo build --release
```

Checklist akhir:
- [ ] `git grep` bersih dari watermark pengembang asli pada UI, metadata, dan binary.
- [ ] `cargo test` seluruh crate lulus tanpa test yang di-skip.
- [ ] Installer Windows menghasilkan `.exe` dengan nama, icon, dan publisher baru.
- [ ] Tidak ada network call diam-diam ke domain `*.oyenza.com`.
