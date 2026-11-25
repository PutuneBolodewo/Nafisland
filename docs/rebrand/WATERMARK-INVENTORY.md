# Inventaris Watermark & Checklist Penggantian

Daftar ini memetakan lokasi referensi **Oyenza Studio**, **Oyen**, dan **Jatevo Labs** yang teridentifikasi pada branch `main` (v0.2.30) beserta rencana tindakannya.

> Gunakan perintah berikut untuk audit menyeluruh sebelum dan sesudah perubahan:
> ```powershell
> git grep -n -i -E "oyenza|oyen|jatevo" main -- crates/ installer/ site/
> ```

---

## 1. Metadata Binary & Installer

| File | Item Watermark | Tindakan | Status |
| :--- | :--- | :--- | :---: |
| `crates/poni/res/poni.rc` | `CompanyName` ("Jatevo Labs"), `ProductName` ("Oyenza Studio"), `FileDescription`, `LegalCopyright`, `InternalName` ("poni"), `OriginalFilename` ("poni.exe"), `1 ICON "poni.ico"` | Generate atau baca dari konfigurasi brand terpusat; ganti `poni.ico` | [ ] |
| `crates/poni/res/poni.manifest` | Identitas assembly aplikasi | Sesuaikan nama/deskripsi aplikasi baru | [ ] |
| `crates/poni/res/poni.ico` | Icon aplikasi Windows | Ganti dengan `.ico` brand baru | [ ] |
| `installer/poni.iss` | Nama app, publisher, URL, nama setup `.exe`, registry uninstall | Parameterisasi ke brand baru | [ ] |
| `installer/poni.nsi` | Nama app, publisher, shortcut, uninstall key | Parameterisasi ke brand baru | [ ] |
| `installer/msix/` | Manifest MSIX, publisher, aset logo | Sesuaikan jika mendistribusikan via MSIX | [ ] |
| `installer/mac/` | Bundle ID (`com.oyenza.poni`), Info.plist, DMG art | Sesuaikan jika menargetkan macOS | [ ] |

---

## 2. Teks UI & Output Aplikasi (`crates/poni-core` & `crates/poni`)

| File | Referensi yang Ditemukan | Tindakan | Status |
| :--- | :--- | :--- | :---: |
| `crates/poni-core/src/notes.rs` | Baris 530: `"\n---\nNotes by Oyen, in Oyenza Studio\n"`, pesan error mikrofon | Ganti footer & pesan izin dengan nama brand dari config | [ ] |
| `crates/poni-core/src/phonedrop/page.html` | `<title>Oyenza Studio · Phone drop</title>`, label header `"Oyenza Studio"` | Ganti dengan placeholder/template dari konfigurasi brand | [ ] |
| `crates/poni-core/src/update.rs` | Format headline `"Oyenza Studio {version}"`, docstring & test | Gunakan konstanta nama brand | [ ] |
| `crates/poni-core/src/ielts.rs` | Pesan izin mikrofon menyebut `"Oyenza Studio"` | Gunakan konstanta nama brand | [ ] |
| `crates/poni-core/src/addons.rs` | Deskripsi `"Oyenza Studio Recorder"` & akun Pro | Sesuaikan copy & kebijakan add-on | [ ] |
| `crates/poni-core/src/romanize/ja_dict.rs` | Pesan error kamus Jepang menyebut `"Oyenza Studio"` | Gunakan konstanta nama brand | [ ] |
| `crates/poni-core/src/downloads.rs` | Kontak `"mailto:hello@oyenza.com"` | Ganti dengan email support sendiri atau hapus | [ ] |
| `crates/poni/src/` (`app.rs`, `activities.rs`, `commands.rs`, `gfx.rs`, dll.) | Judul jendela, tooltip, teks halaman settings/about | Audit & ganti ke konstanta brand | [ ] |

---

## 3. Endpoint Eksternal & Layanan Cloud

| File | Endpoint / Simbol | Tindakan | Status |
| :--- | :--- | :--- | :---: |
| `crates/poni-core/src/update.rs` | `FEED_URL = "https://studio.oyenza.com/latest.json"` | Arahkan ke URL rilis sendiri atau tambahkan flag nonaktif | [ ] |
| `crates/poni-core/data/update-keys.txt` | Public key ECDSA milik Oyenza Studio | Ganti dengan public key sendiri jika pakai auto-update | [ ] |
| `crates/poni-core/src/pairdrop/mod.rs` | `DEFAULT_SERVER = "https://drop.oyenza.com"` | Arahkan ke server PairDrop sendiri atau wajibkan env/config | [ ] |
| `crates/poni-core/src/prayer/world.rs` | `BASE_URL = "https://studio.oyenza.com/places/1/"` | Mirror dataset `site/places/1/` ke repo/CDN sendiri | [ ] |
| `crates/poni-core/src/oyenza.rs` | Integrasi akun Oyenza & status Pro (`{"pro": bool}`) | Nonaktifkan atau ganti dengan backend akun sendiri | [ ] |

---

## 4. Aset Visual, Maskot & Dokumentasi

| Lokasi | Konten | Tindakan | Status |
| :--- | :--- | :--- | :---: |
| `crates/poni-core/src/oyen/` (`mod.rs`, `mark.rs`, `geometry.rs`) | Maskot kucing Oyen yang digambar prosedural | Ganti geometri/mark dengan logo atau maskot kustom | [ ] |
| `crates/poni/src/icons.rs` | Kumpulan icon SVG path 24×24 | Pertahankan icon standar atau perbarui dari Lucide / `line-md` | [ ] |
| `docs/brand/` | `oyen-mark*.svg`, `oyen-poni-logo*`, `art/poses/oyen-*.webp`, `social-preview.png` | Hapus atau ganti seluruhnya dengan aset brand baru | [ ] |
| `site/` | Landing page `studio.oyenza.com` | Ganti/hapus jika tidak menggunakan website bawaan | [ ] |
| `README.md`, `PRIVACY.md` | Dokumentasi publik Oyenza Studio | Tulis ulang sesuai produk baru | [ ] |
