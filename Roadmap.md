# Roadmap

Kita ingin membuat aplikasi 3D modelling desktop dalam Rust, dengan format aset sendiri untuk game engine custom kita. Pengguna bisa mengedit lewat viewport; Codex atau harness lain bisa mengerjakan model lewat operasi yang sama.

Sekarang kita fokus ke kernel, CLI, dan MCP. At least for now, for the sake of rapid development. Kita perlu bisa mengubah geometri, menjalankan tes, lalu melihat hasilnya tanpa harus membenahi editor setiap kali.

Roadmap ini mengatur urutan kerja, bukan menjanjikan tanggal rilis. Bagian yang sudah berjalan dicatat di bawah. Sisanya masih rencana.

## Posisi sekarang

Kernel sudah bisa menyusun sphere dan box dengan union, intersection, dan difference. Sumbernya JSON ber-ID stabil, lalu compiler menulis instruksi biner `.t3d`. Ada query titik, preview CPU, CLI JSON, dan tiga tool MCP yang bekerja di memori.

Validasi lokal mencakup 15 tes serta pemeriksaan format dan Clippy. Detailnya ada di [catatan pengujian](<docs/validation.md>). Belum ada editor desktop, renderer GPU, material, atau animasi. Integrasi MCP belum dicoba di aplikasi Codex atau Harness sungguhan.

## 1. Bikin model enak diedit

Mengganti isi JSON cukup untuk membuktikan kernel bekerja. Untuk editor dan agent, kita butuh operasi edit yang punya batas jelas.

- Tambahkan perintah untuk membuat, mengubah, dan menghapus node; mengganti operand; serta memilih root.
- Terapkan satu kumpulan perubahan sebagai transaksi. Kalau hasil graph tidak valid, model lama tetap utuh.
- Tambahkan revision check agar agent tidak menimpa perubahan pengguna dari snapshot yang sudah basi.
- Simpan riwayat untuk undo/redo. Pertahankan ID node yang tidak berubah.
- Tentukan kontrak transform dan instance subgraf. Mulai dari translasi, rotasi, dan uniform scale; non-uniform scale perlu aturan distance bound tersendiri.
- Siapkan corpus kecil berisi lubang, bagian berulang, dinding tipis, dan permukaan berimpit. Pakai sejak tahap ini untuk menangkap regresi.
- Coba alur edit nyata lewat satu client Codex dan satu harness lain. Catat konfigurasi yang berhasil dan kegagalannya.

Tahap ini selesai ketika pengguna atau agent bisa mengubah satu bagian, membatalkan perubahan, dan mengulanginya tanpa mengubah node lain. Dua edit dari revisi yang sama harus menghasilkan konflik yang jelas jika keduanya saling menimpa.

Transform mungkin membutuhkan instruksi biner baru. Kita tidak akan mengubah arti format v1 diam-diam; aturan versi mengikuti [spesifikasi format](<docs/format.md>).

## 2. Buka model di aplikasi desktop

Buat satu jendela editor dengan viewport GPU. Kandidat awalnya `wgpu`, `winit`, dan `egui`; pilih setelah percobaan kecil untuk rendering field, input, dan pemilihan objek. Belum ada keputusan dependensi UI saat ini.

Editor awal perlu orbit/pan/zoom, frame selection, daftar node, panel parameter, dan gizmo transform. Pengguna juga harus bisa membuka sumber, menyimpan perubahan, lalu mengompilasi aset tanpa meninggalkan editor.

Target pengujiannya sederhana: buat model bracket, tambah lubang, pindahkan bagian, undo, simpan, tutup aplikasi, lalu buka lagi. Hasilnya harus tetap sama. Jika agent mengedit model yang sedang terbuka, viewport harus menampilkan revisi yang sudah diterima core.

CLI dan MCP tetap bisa berjalan tanpa jendela. Tombol editor memanggil operasi edit dari tahap pertama, bukan menyimpan logika geometri sendiri.

## 3. Tambah alat modelling yang memang dipakai

Setelah alur editor berjalan, perluas bentuk dan operasi berdasarkan aset yang kita buat. Cylinder dan capsule menjadi kandidat berikutnya. Tambahkan duplicate, group, instance, pengaturan pivot, serta boolean dengan hasil yang bisa diperiksa sebelum diterapkan.

Warna atau material sederhana per bagian cukup untuk tahap awal. UV, tekstur kompleks, rigging, dan animasi belum masuk pekerjaan ini.

Kita perlu beberapa aset uji buatan sendiri agar keputusan tidak cuma berdasar sphere dan box. Kalau suatu bentuk membutuhkan terlalu banyak node atau sulit diedit, catat kasusnya. Hasil itu bisa menjadi alasan menambah operasi, menyediakan hasil bake, atau membatasi jenis aset yang kita dukung.

Tahap ini selesai ketika aset uji tersebut bisa dibuat lewat editor dan diedit lewat agent tanpa harus mengubah kode Rust.

## 4. Ukur biaya, baru pilih optimasi

Simpan pengukuran sejak awal; tahap ini memperluas pembanding dan mengerjakan bottleneck yang sudah terlihat.

Ukur ukuran aset, memori setelah load, waktu compile, biaya edit, query field, dan waktu frame pada perangkat uji yang dicatat. Bandingkan juga dengan mesh terkompresi pada kualitas permukaan yang sebanding. Angka sphere 52 byte belum cukup untuk menyimpulkan format kita lebih ringan untuk semua aset.

Pilihan optimasinya bergantung hasil: buang node yang tidak terjangkau, dedup ekspresi, evaluasi batch, pemangkasan spasial, atau cache hasil sampling. JIT dan cache voxel belum perlu dipasang sebelum ada beban kerja yang membenarkannya.

Setiap optimasi harus tetap lulus corpus correctness. Untuk aproksimasi, laporkan galat dan fitur yang hilang, terutama dinding tipis. Prosedur pembanding ada di [catatan riset](<docs/research.md>).

## 5. Pakai asetnya di game engine custom

Mulai dengan aplikasi runtime kecil yang memakai loader bersama dan menampilkan aset hasil editor. Jangan menunggu seluruh game engine selesai untuk menguji jalur ini.

Tentukan kebutuhan render, material, instance, dan query geometri dari runtime tersebut. Pilih evaluasi field langsung atau hasil bake berdasarkan profil. Query collision perlu pengujian sendiri; field saat ini bukan jaminan jarak penetrasi yang akurat.

Tahap ini selesai ketika aset bisa dibuat di editor, dikompilasi, lalu dimuat runtime dengan bentuk dan penempatan yang konsisten. Loader harus menolak versi yang tidak didukung dengan pesan yang jelas.

Format custom adalah kontrak antara tool dan engine kita. Itu bukan proteksi agar orang lain tidak bisa membaca asetnya.

## Sebelum dipakai untuk pekerjaan penting

Tambahkan autosave dan pemulihan setelah crash, penulisan file atomik, migrasi dokumen, serta fuzzing parser dan decoder. Uji juga izin file, batas penggunaan sumber daya, dan konflik ketika agent bekerja bersamaan dengan pengguna.

Target sistem operasi, kebutuhan GPU minimum, dan format rilis perlu diputuskan setelah editor awal bisa diuji. Belum ada janji dukungan platform atau tanggal rilis.

## Yang sengaja belum dikejar

Sculpting umum, impor scan, neural field sebagai format utama, kolaborasi cloud, dan fitur CAD presisi manufaktur masih di luar rencana pengerjaan awal. Begitu juga membangun game engine lengkap di dalam repo modelling ini.

Pekerjaan terdekat adalah tahap pertama: operasi edit, transaksi, dan revision check. Setelah itu kita punya dasar untuk membuat editor desktop tanpa memberi UI dan agent dua cara berbeda untuk mengubah model.
