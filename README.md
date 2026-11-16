# t3dnyere

Kernel pemodelan solid prosedural dalam Rust. Saat ini kita mengerjakan kernel, CLI, dan MCP dulu.

At least for now, for the sake of rapid development.

Kita belum membuat editor desktop supaya perubahan geometri dan format aset bisa langsung diuji tanpa mengurus UI setiap kali. Targetnya tetap aplikasi modelling desktop, dengan kernel yang sama untuk pengguna dan agent AI. Game engine custom akan memakai aset hasil kompilasinya. Urutan pengerjaannya ada di [Roadmap](<Roadmap.md>).

Model disimpan sebagai program CSG, bukan daftar polygon. Sumber JSON dapat diedit manusia/agent; compiler menghasilkan format runtime custom `.t3d`. Satu core dipakai CLI, library Rust, preview CPU, dan server MCP stdio. Tidak memerlukan model AI, API key, atau layanan cloud.

## Coba

```sh
cargo test --locked
cargo build --release --locked
# Output berikut harus belum ada; CLI menolak overwrite.
./target/release/t3dnyere validate examples/bracket.t3d.json
./target/release/t3dnyere compile examples/bracket.t3d.json bracket.t3d
./target/release/t3dnyere inspect bracket.t3d
./target/release/t3dnyere sample bracket.t3d 0 0 0
./target/release/t3dnyere preview bracket.t3d bracket.ppm 128
./target/release/t3dnyere schema
```

`validate`, `inspect`, `sample`, dan `preview` menerima JSON sumber atau biner. `compile` hanya menerima JSON. Ganti input dengan `-` untuk stdin. CLI menghasilkan JSON pada stdout, termasuk error dengan exit code 1; `--help` berupa teks. Preview PPM dapat dibuka dengan viewer yang mendukung PPM. Kamera CLI tetap menghadap origin dari `(4,3,5)`; API library menerima kamera sendiri.

Contoh sumber: [bracket.t3d.json](<examples/bracket.t3d.json>). Satuan meter, Y-up, koordinat right-handed; box memakai **half_size**, bukan ukuran penuh.

Preview CPU hasil contoh (192×192; PNG hanya konversi PPM untuk dokumentasi):

![Bracket: box dengan cekungan sphere](<docs/bracket-preview.png>)

```json
{
  "version": 1,
  "root": "cut",
  "nodes": [
    {"id": "body", "op": "box", "center": [0,0,0], "half_size": [1,1,1]},
    {"id": "hole", "op": "sphere", "center": [0,0,1], "radius": 0.7},
    {"id": "cut", "op": "difference", "a": "body", "b": "hole"}
  ]
}
```

## Yang sudah diuji

- Sphere, axis-aligned box, union, intersection, difference; sharing subgraf melalui ID.
- Validasi referensi, ID duplikat, siklus, unknown fields, versi, angka dan batas alokasi.
- Compiler topologis deterministik: urutan node sumber tidak mengubah byte output. Semua node, termasuk unreachable, tetap divalidasi dan dikompilasi.
- Codec biner eksplisit little-endian; decoder tidak memakai `unsafe` atau dump layout Rust.
- Evaluator iteratif tanpa rekursi graph; CPU sphere-traced preview dengan work budget.
- CLI JSON dan MCP stdio dengan `validate_model`, `sample_model`, `compile_model`.
- 15 tes integrasi/core serta lint Clippy. Lihat [tes core](<tests/core.rs>) dan [tes CLI](<tests/cli.rs>).

Integrasi Codex/harness: [panduan agent](<docs/agents.md>). Aturan contributor: [AGENTS.md](<AGENTS.md>). Spesifikasi aset: [format v1](<docs/format.md>). Hasil pengujian lokal: [validation](<docs/validation.md>).

## Apa artinya “lebih ringan”?

**Derived:** biner v1 berukuran `20 + 32 × jumlah_instruksi` byte. Satu sphere = **52 byte**; contoh bracket 3 node = **116 byte**. Ini tidak termasuk source JSON atau overhead allocator runtime.

**Tested:** eksperimen penyimpanan menghasilkan buffer mesh sphere raw position-f32/index-u32 sebesar **4.056 byte** pada 8×16 dan **71.448 byte** pada 32×64. Jalankan:

```sh
cargo run --locked --release --example measure
```

Kode eksperimen: [measure.rs](<examples/measure.rs>). Bola analitik dan mesh tessellated mempunyai representasi permukaan berbeda. Pembanding **belum dikompresi**, tanpa normal/UV/material. Ini bukan bukti mengalahkan glTF+meshoptimizer, bukan perbandingan kualitas setara, dan bukan benchmark GPU/RSS. Runtime procedural dapat menukar ukuran file dengan biaya evaluasi lebih besar. Klaim keunggulan umum masih **Hypothesized**.

## Batas saat ini

- Bukan mesh editor/sculpting, belum ada GUI, GPU, rotasi/scale node, material, UV, tekstur, animasi, skinning, atau ekstraksi mesh.
- Field CSG menjaga konvensi tanda (`field < 0` = interior), tetapi umumnya **bukan jarak Euclidean eksak**. Preview menelusuri zero-set, bukan regularized CAD boundary: `A − A` memiliki interior kosong tetapi dapat menyisakan ghost surface. Jangan gunakan ini sebagai kernel CAD/physics yang sudah tervalidasi.
- f32; posisi maksimum ±1e6 m dan ukuran 1e-4..1e6 m adalah batas input, **bukan jaminan akurasi**. Detail kecil pada posisi besar hilang karena presisi float. Toleransi preview tetap, cocok untuk demo skala meter.
- Evaluasi O(N) per titik; `sample()` mengalokasikan scratch per panggilan, preview menggunakannya kembali. Tidak ada acceleration structure, pruning unreachable, SIMD/JIT, atau dedup isi.
- Input maksimum 2 MiB, 4096 node; preview maksimum 512² dan 100 juta evaluasi node worst-case. Model besar perlu preview lebih kecil.
- Output CLI tidak menimpa file yang ada. Penulisan belum transaksional/fsync; kegagalan I/O dapat meninggalkan output parsial.
- Format custom bukan enkripsi/DRM: engine kita adalah target runtime, tetapi pihak lain bisa membuat decoder. Tidak ada klaim proteksi aset.
- MCP diuji dengan client protokol subprocess lokal; belum diuji melalui aplikasi Codex/Harness sungguhan. Integrasi vendor masih perlu smoke test di host pilihan.

## Riset dan urutan pengembangan

Riset memakai **papers MCP dan pencarian web**: [catatan riset dan tradeoff](<docs/research.md>). Dasar distance-bound tracing: [Hart, 1996](https://doi.org/10.1007/s003710050084); tantangan deformasi: [Seyb dkk., 2019](https://doi.org/10.1145/3355089.3356502). Pembanding penting: [meshoptimizer](https://meshoptimizer.org/) dan [Fidget](https://github.com/mkeeter/fidget). Implementasi ini tidak mengadopsi kode keduanya.

Pekerjaan berikutnya adalah operasi edit dengan transaksi dan revision check, lalu editor desktop dengan viewport GPU. Rencana lengkap beserta kriteria selesai tiap tahap ada di [Roadmap](<Roadmap.md>).
