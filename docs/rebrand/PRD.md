# PRD: Rebranding & Kustomisasi (White-Label Ready)

## 1. Latar Belakang & Tujuan

Repositori ini merupakan fork dari **Oyenza Studio** (`lucacadalora/oyenza-studio`, basis `main` v0.2.30). Saat ini seluruh identitas visual, teks UI, metadata installer, aset maskot (`Oyen`), dan endpoint layanan masih terikat ke merek **Oyenza Studio / Jatevo Labs**.

**Tujuan akhir:** Menghasilkan basis kode yang bersih dari seluruh watermark Oyenza/Oyen/Jatevo dan memiliki lapisan konfigurasi brand terpusat sehingga software siap dikustomisasi ulang (nama produk, logo, icon, tema, dan endpoint) dengan perubahan minimal.

---

## 2. Ruang Lingkup

### Termasuk dalam Ruang Lingkup
1. **Sentralisasi Identitas Brand:** Menyediakan satu sumber konfigurasi brand (nama aplikasi, vendor, copyright, domain, email, identifier sistem) di `crates/poni-core` yang juga diteruskan ke metadata Windows (`poni.rc`) dan installer.
2. **Penghapusan Watermark Teks & Metadata:** Mengganti seluruh string `Oyenza Studio`, `Oyen`, `Jatevo Labs`, dan URL `oyenza.com` pada UI, output catatan (`notes.rs`), halaman lokal (`phonedrop/page.html`), resource binary (`poni.rc`), dan skrip installer (`installer/`).
3. **Penggantian Aset Visual, Icon & Maskot:** Mengganti `poni.ico`, aset di `docs/brand/`, maskot prosedural `crates/poni-core/src/oyen/`, serta menyiapkan katalog icon/animasi SVG open-source berlisensi permisif (MIT) yang kompatibel dengan renderer Direct2D (`svgpath.rs`) dan web.
4. **Pemutusan / Pengalihan Endpoint Eksternal:** Mengalihkan atau menonaktifkan endpoint milik pengembang asli (`studio.oyenza.com`, `drop.oyenza.com`, `hello@oyenza.com`, serta fitur akun/Pro Oyenza).

### Di Luar Ruang Lingkup (Untuk Fase Terpisah)
- Penambahan fitur produk baru di luar kebutuhan rebranding dan kustomisasi.
- Pengubahan arsitektur rendering native Win32/Direct2D menjadi webview/Electron (harus tetap ringan sesuai prinsip `AGENTS.md`).

---

## 3. Kebutuhan Fungsional (Functional Requirements)

| ID | Kebutuhan | Detail |
| :--- | :--- | :--- |
| **FR-1** | Konfigurasi Brand Terpusat | Semua nama produk, vendor, copyright, email, dan domain dibaca dari satu modul/konfigurasi terpusat (`brand`), bukan hardcode tersebar. |
| **FR-2** | Nol Watermark Oyenza di UI & Output | Tidak ada teks `Oyenza`, `Oyen`, atau `Jatevo` pada jendela aplikasi, tray, toast, footer Markdown (`notes.rs`), maupun halaman web lokal (`phonedrop/page.html`). |
| **FR-3** | Metadata Binary & Installer Kustom | `poni.rc`, `poni.manifest`, dan skrip `installer/` (`poni.iss`, `poni.nsi`, `mac/`, `msix/`) mencerminkan nama produk dan vendor baru. |
| **FR-4** | Icon & Maskot Mudah Diganti | Aset icon di `crates/poni/src/icons.rs` menggunakan SVG path 24×24 berlisensi MIT; modul maskot `oyen/` diganti atau diabstraksi agar mudah diswap. |
| **FR-5** | Endpoint Independen | Auto-update (`update.rs`), PairDrop (`pairdrop/mod.rs`), dan data lokasi (`prayer/world.rs`) dapat diarahkan ke server sendiri atau dimatikan secara bersih. |
| **FR-6** | Pemisahan Identitas Runtime (Opsional/Konfigurabel) | Nama executable, mutex (`Local\Poni.Island`), registry autostart/uninstall, dan macOS bundle ID (`com.oyenza.poni`) dapat diubah agar tidak bentrok dengan instalasi Oyenza Studio asli. |

---

## 4. Kebutuhan Non-Fungsional (Non-Functional Requirements)

- **Ringan & Hemat Resource (`AGENTS.md`):** Tidak menambah background thread/timer saat idle, tidak menambah dependency crate berat untuk animasi SVG (manfaatkan `svgpath.rs` + Direct2D yang sudah ada).
- **Lisensi Aman:** Seluruh icon dan aset pengganti wajib menggunakan lisensi open-source permisif (**MIT**, **ISC**, atau **CC0**) tanpa watermark komersial.
- **Dapat Diverifikasi:** Harus lulus `cargo test`, build rilis, dan pemeriksaan `git grep -i -E "oyenza|oyen|jatevo"` tanpa temuan di kode produksi.

---

## 5. Keputusan yang Perlu Dikonfirmasi Sebelum Eksekusi

1. **Identitas Baru:** Nama produk final, nama vendor/copyright, dan domain/repo pengganti.
2. **Pemisahan Total vs Update In-Place:** Apakah identifier internal (`poni.exe`, `Local\Poni.Island`, `com.oyenza.poni`) diubah total sebagai aplikasi baru (disarankan untuk fork kustom) atau dipertahankan.
3. **Basis Branch Kerja:** Memulai branch rebrand baru dari `main` (`0.2.30`) lalu memindahkan perubahan lokal yang dibutuhkan dari `develop` (`tupat-*`, `.cargo/config.toml`).
