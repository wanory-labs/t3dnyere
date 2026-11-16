# Riset representasi untuk mesin pemodelan 3D Rust ringan

Tanggal verifikasi sumber: 17 November 2026. Dokumen ini menilai rancangan, bukan melaporkan mesin 3D yang sudah selesai.

## Ringkasan keputusan

[Hypothesized] Pilih **CSG prosedural berbasis fungsi jarak sebagai format authoring**, dengan DAG ber-ID stabil dalam JSON dan hasil kompilasi berupa format biner runtime. Ini paling dekat dengan kebutuhan AI yang mengubah parameter dan bagian model, bukan jutaan sampel atau vertex. Klaim “AI-friendly” masih hipotesis yang perlu eksperimen penyuntingan.

[Derived] Pisahkan model sumber dari representasi pengiriman. Mesh terkompresi, octree, dan bahkan neural field dapat menjadi hasil bake atau cache di masa depan; tidak harus menggantikan bahasa authoring. File biner tidak otomatis berarti kompresi terbaik, dan representasi kecil tidak otomatis berarti evaluasi cepat.

[Hypothesized] Tiga arah yang dipertahankan: (A) inti CSG analitik minimal sekarang; (B) compiler DAG deterministik dan incremental setelah baseline; (C) cache spasial adaptif opsional jika biaya query terbukti menjadi penghambat. Mesh terkompresi menjadi pembanding dan calon ekspor, bukan pekerjaan implementasi saat ini. Neural field ditunda.

## Lingkup dan kontrak bukti

Pengguna meminta mesin kustom berbasis Rust yang ringan dan ramah AI. Rancangan awal yang dipilih asisten: sphere dan box; union/intersection/difference dalam DAG; translasi melalui pusat primitif; JSON dengan ID stabil; format biner runtime kustom. Belum ada implementasi UI, GPU, atau mesh yang menjadi dasar kesimpulan riset ini. Dokumen ini tidak mengaudit kode mesin dan tidak mengklaim benchmark Rust sudah dijalankan.

Label berlaku pada klaim dan usulan:

- **Tested**: langkah benar-benar dijalankan dalam sesi ini. Cakupannya hanya pemeriksaan sumber dan eksperimen aritmetika Python kecil, bukan validasi engine atau reproduksi paper.
- **Derived**: kesimpulan dari rumus, struktur data, atau isi sumber terverifikasi. Hasil yang dilaporkan penulis paper tetap bukti eksternal; bukan hasil uji lokal.
- **Hypothesized**: manfaat, skor keputusan, target, dan rencana eksperimen yang belum terbukti pada proyek ini.

[Derived] Batas keras: floating point tidak sama dengan geometri eksak; penyimpanan sampel memerlukan resolusi; evaluasi program tetap memerlukan komputasi. Asumsi yang boleh diubah: satu format harus melayani editing sekaligus rendering, semua operasi harus tersedia sejak versi pertama, atau “AI” berarti wajib memakai jaringan saraf.

## Perbandingan empat keluarga representasi

| Keluarga | Kekuatan yang relevan | Biaya dan batas | Peran yang disarankan |
|---|---|---|---|
| CSG/fungsi jarak prosedural | [Derived] Parameter dan operasi menyimpan konstruksi, bukan seluruh permukaan; perubahan radius atau operand bisa lokal pada sumber. Hart [1] memberi dasar distance-bound tracing. | [Derived] Evaluasi naif setiap titik O(V+E) pada DAG yang dievaluasi sekali per node; banyak titik mengulang kerja. Boolean min/max umumnya bukan jarak Euclidean eksak. | [Hypothesized] Sumber utama untuk objek hard-surface sederhana, parametrik, berulang. |
| Voxel padat, sparse octree, voxel DAG | [Derived] Grid memberi alamat spasial langsung; octree melewati ruang kosong, voxel DAG berbagi subpohon identik [3]. | [Derived] Grid padat O(n³); detail tipis bisa hilang. Octree tidak menjamin kompresi pada data rumit. Edit DAG terdeduplikasi perlu copy-on-write atau rebuild. | [Hypothesized] Cache, sculpting, atau volume hasil sampling, bukan sumber awal. |
| Mesh terkompresi | [Derived] meshoptimizer menyediakan optimasi, kuantisasi dan codec vertex/index; mendokumentasikan binding Rust melalui crate meshopt [5]. | [Derived] Mesh menyimpan hasil permukaan, bukan maksud parametrik; perlu ekstraksi/tessellation sebelum codec. Codec lossless terhadap buffer masukan tidak membatalkan galat kuantisasi sebelumnya. | [Hypothesized] Ekspor/runtime untuk aset statis jika kelak diperlukan, plus baseline pembanding. |
| Neural field | [Derived] DeepSDF belajar fungsi kontinu untuk kelas bentuk, interpolasi dan completion [4]. | [Derived] Bobot, latent, pelatihan/fitting, dependensi inferensi dan galat aproksimasi harus dihitung. Kontinuitas saja tidak membuktikan sifat SDF metrik atau topology yang benar. | [Hypothesized] Opsional untuk impor/reconstruction organik, bukan inti editor deterministik. |

[Derived] Tidak ada pemenang kompresi universal. Bola bisa dideskripsikan oleh pusat dan radius; scan bertekstur dengan detail acak mungkin tidak punya deskripsi CSG kecil. Perbandingan yang adil harus menyamakan kualitas, domain, dan fitur yang disimpan, bukan hanya ekstensi file.

## Dua belas kandidat melalui empat lensa

Semua manfaat implementasi pada tabel ini berstatus **Hypothesized**; kolom mekanisme menjelaskan derivasi atau rujukan, bukan hasil benchmark proyek.

| # | Lensa dan ide | Mekanisme yang dipinjam | Tempat gagalnya / eksperimen pembunuh |
|---|---|---|---|
| 1 | Geometri: CSG analitik sphere/box | [Derived] Ubah koordinat ke pusat primitif; gabungkan field dengan min/max/negasi [1]. | Detail organik membutuhkan banyak node; uji benda berlubang, perpotongan dan bentuk hampir bersinggungan. |
| 2 | Geometri: instancing dan simetri | [Derived] Satu subgraf dapat direferensikan banyak kali; transform harus menjadi konteks evaluasi, bukan mutasi pusat bersama. | Primitive-center saja belum memberi transform subgraf murah; uji ribuan instance dengan pusat berbeda. Tunda transform node. |
| 3 | Geometri: cache SDF narrow-band adaptif | [Derived] Alokasikan sampel dekat permukaan, simpan sumber prosedural sebagai otoritas. | Interpolasi bisa menghilangkan dinding tipis atau melanggar bound; bandingkan galat dan rebuild pada edit lokal. |
| 4 | CS: JSON ber-ID stabil dan patch terarah | [Derived] Identitas objek dipisah dari indeks penyimpanan; validasi referensi sebelum evaluasi. | ID stabil tidak otomatis berarti edit AI benar; ukur keberhasilan patch dan jumlah perubahan tak terkait. |
| 5 | CS: sparse voxel DAG immutable | [Derived] Intern subpohon identik untuk berbagi region spasial [3]. | Atribut unik mengurangi sharing; edit memerlukan salinan jalur. Uji occupancy berulang versus noise dengan warna unik. |
| 6 | CS: mesh terkompresi sebagai hasil bake | [Derived] Pisahkan konstruksi dari buffer siap kirim; ukur codec terhadap mesh yang sama [5]. | Kehilangan provenance/parameter; bandingkan waktu bake+decode dan ukuran total, bukan codec saja. |
| 7 | Compiler: DAG menjadi tape topologis | [Derived] Topological sort menghasilkan instruksi berindeks; evaluasi setiap node sekali per titik. Fidget memberi preseden [6]. | Tape dapat menghitung node yang tidak relevan; ukur per titik dan batch terhadap interpreter rekursif. |
| 8 | Compiler: hash-consing dan incremental rebuild | [Derived] Bedakan ID authoring dari hash isi; invalidasi ancestor dari node yang berubah. | DAG sharing bisa menciptakan perubahan tak disengaja; uji satu operand dipakai banyak root dan biaya patch. |
| 9 | Compiler: interval/bound-aware simplification | [Derived] Buktikan cabang tak berpengaruh pada satu sel sebelum memangkasnya; Fidget memakai interval dan tape simplification [6]. | Bound salah berarti permukaan hilang; interval terlalu longgar berarti overhead tanpa manfaat. Cocokkan ke evaluator referensi. |
| 10 | Science: kontrak galat dan metamorphic testing | [Derived] Invariansi translasi, hukum tanda boolean, dan batas Lipschitz menjadi oracle tanpa renderer. | Oracle yang sama dengan implementasi bisa menyalin bug; gunakan titik analitik dan implementasi referensi independen. |
| 11 | Science: neural residual di atas primitif | [Derived] Pisahkan bentuk kasar analitik dari residual terpelajar; DeepSDF [4] mendukung kelayakan field terpelajar, bukan metode hybrid ini. | [Hypothesized] Residual bisa merusak jarak, topology dan interpretabilitas; hentikan bila fitting/dependensi melebihi manfaat. |
| 12 | Science: eksperimen rate-distortion dan editing AI | [Derived] Ukur ukuran versus galat sekaligus keberhasilan perubahan; pisahkan error representasi dari error agen. | Dataset yang hanya berisi bola/box menguntungkan CSG; sertakan kasus adversarial dan scan organik, laporkan kegagalan. |

[Derived] Lensa compiler bukan sekadar metafora: model adalah program, DAG adalah IR, runtime biner adalah artefak kompilasi, dan invalidasi adalah dependency tracking. Lensa science mengubah “ringan” menjadi klaim yang dapat dibantah oleh pengukuran ukuran, waktu, memori dan galat.

## Konvergensi ke tiga arah

[Hypothesized] Skor berikut adalah penilaian rancangan skala 1 sampai 5, dengan 5 paling baik. Kolom murah, cepat, dan aman berarti biaya lebih rendah, waktu uji lebih singkat, dan risiko lebih kecil. Total tidak boleh dibaca sebagai probabilitas keberhasilan.

| Arah | Layak saat ini | Bukti mekanisme | Murah | Cepat diuji | Sederhana | Aman | Total |
|---|---:|---:|---:|---:|---:|---:|---:|
| A. CSG sphere/box minimal + JSON/biner | 5 | 4 | 5 | 5 | 5 | 4 | 28 |
| B. Tape deterministik + dedup/incremental | 4 | 4 | 4 | 4 | 4 | 3 | 23 |
| C. Cache spasial adaptif turunan | 3 | 3 | 3 | 3 | 3 | 2 | 17 |
| D. Voxel DAG sebagai sumber utama | 2 | 4 | 2 | 2 | 2 | 2 | 14 |
| E. Mesh terkompresi sebagai sumber utama | 2 | 4 | 2 | 2 | 2 | 3 | 15 |
| F. Neural field sebagai sumber utama | 1 | 3 | 1 | 1 | 2 | 1 | 9 |

### A. Inti geometri yang kecil dan dapat diperiksa

[Hypothesized] Implementasi awal cukup mencakup primitif, operasi boolean, validasi DAG, evaluasi titik, pembacaan/penulisan JSON dan biner. Tetapkan satuan, handedness, tanda negatif di dalam, root eksplisit dan batas ukuran input. Translasi hanya melalui pusat primitif sesuai rencana. Jangan menyisipkan UI, GPU, smooth blending, rotasi, deformasi, atau ekstraksi mesh ke milestone ini.

[Derived] Untuk sphere berpusat `c`, radius `r`, gunakan `length(p-c)-r`. Untuk box axis-aligned dengan half-extents positif `b`, gunakan `q=abs(p-c)-b`, lalu `length(max(q,0))+min(max(q.x,q.y,q.z),0)`. Bedakan half-extents dari ukuran penuh dalam schema. Tolak NaN/Infinity, radius nonpositif dan extents nonpositif sampai ada kontrak eksplisit untuk degenerasi.

[Derived] Jika negatif berarti interior, union adalah `min(a,b)`, intersection `max(a,b)`, dan difference A\\B `max(a,-b)`. Rumus tersebut menggabungkan tanda, tetapi **bukan jaminan jarak Euclidean eksak terhadap hasil boolean**. Dengan leaf 1-Lipschitz dan translasi saja, min/max/negasi tetap 1-Lipschitz. Ini mendukung pembahasan bound konservatif; bukan lisensi mengklaim semua query jarak, normal atau collision sudah eksak.

[Derived] Bahkan zero set butuh kontrak: A\\A menghasilkan `abs(dA)`, sehingga interior negatif kosong tetapi zero set masih memuat permukaan A. Kontak tangensial dan muka berimpit juga perlu keputusan regularized-solid versus zero-set. Definisikan `inside = field < 0`, toleransi permukaan dan semantik hit secara terpisah. Jangan menyebut kernel CAD eksak atau manifold-safe.

[Hypothesized] Kriteria gagal awal: oracle tanda salah pada titik yang jauh dari batas; input siklik bisa mencapai evaluator; round-trip mengubah parameter; atau semantik difference tidak terdokumentasi. Perbaiki correctness sebelum optimasi.

### B. Compiler model dan protokol authoring

[Hypothesized] JSON menjadi sumber kebenaran. ID authoring tetap stabil saat parameter berubah, sementara compiler mengubah referensi ke indeks padat dalam urutan topologis deterministik. Simpan pemetaan ID ke indeks untuk diagnostik. Bedakan hash isi untuk dedup dari identitas untuk editing; menyatukannya membuat edit radius mengganti identitas objek.

[Derived] Hash-consing menghemat struktur identik, tidak menghapus kebutuhan evaluasi operand yang berbeda. Memoization per titik pada DAG dapat menghindari ekspansi rekursif berulang; memoization antartitik tidak berlaku tanpa memasukkan posisi dalam key. Transform subgraf nantinya juga membuat konteks posisi penting.

[Hypothesized] Format biner perlu magic, versi, endian eksplisit, jumlah node, root index, tag operasi, parameter dan indeks operand. Jangan dump layout struct Rust yang bergantung padding/layout. Decoder perlu memeriksa panjang, tag, overflow, batas node/depth dan validitas referensi sebelum alokasi/evaluasi. Urutan instruksi harus tidak bergantung iterasi hash map. Tambahkan checksum hanya jika ada kebutuhan integritas; checksum bukan autentikasi.

[Hypothesized] Uji pembunuh: bandingkan interpreter referensi dengan tape pada titik identik, termasuk DAG berbagi yang dalam. Jika bytecode tidak memberi manfaat runtime atau ukuran terukur, pertahankan serialisasi sederhana dahulu; tunda JIT/SIMD. Fidget [6] menunjukkan preseden arsitektur, bukan bukti bahwa menyalin kompleksitasnya cocok bagi proyek ini.

### C. Cache spasial adaptif, bukan sumber model

[Hypothesized] Setelah bottleneck terbukti, bangun octree/narrow-band dari field prosedural dan invalidasi sel terkait saat edit. Cache boleh dibuang dan dibangun ulang; JSON tidak boleh kehilangan maksud konstruksi. Ini menjaga jalur ke voxel tanpa mengunci seluruh editor ke resolusi grid.

[Derived] AABB geometri bukan otomatis bound evaluasi semua operasi; difference/intersection dan ruang kosong memerlukan aturan yang benar. Interpolasi grid SDF tidak otomatis konservatif untuk sphere tracing. Gunakan interval atau margin galat terbukti, atau fallback ke evaluasi analitik ketika kepastian tidak tersedia.

[Hypothesized] Kriteria masuk: query berulang mendominasi waktu. Target eksperimen, bukan hasil: setidaknya 2× pengurangan p95 query pada dataset yang ditentukan, tanpa pelanggaran bound/galat yang disepakati, dan total build+edit tidak lebih buruk pada workload nyata. Bila gagal, jangan tambahkan cache ke jalur default.

## Eksperimen kecil yang benar-benar dijalankan

[Tested] Sesi ini menjalankan Python 3 inline, tanpa menulis berkas pengujian dan tanpa memakai engine Rust. Pada titik `(2,2,0)`:

- Box A: pusat `(0,1,0)`, half-extents `(1,10,10)`, jarak = `1`.
- Box B: pusat `(1,0,0)`, half-extents `(10,1,10)`, jarak = `1`.
- Field intersection `max(A,B)` = `1`.
- Titik terdekat pada intersection adalah `(1,1,0)`, jarak Euclidean = `sqrt(2)` = `1.414213562373`.

[Derived] Ini counterexample eksplisit untuk klaim “hasil max selalu SDF eksak”. Selisih tersebut tidak membuktikan renderer rusak; justru menunjukkan perlunya membedakan field konservatif, sign test, dan true distance.

[Tested] Aritmetika alokasi grid `n³ × 4 byte` menghasilkan 8 MiB pada 128³, 64 MiB pada 256³, dan 512 MiB pada 512³. Angka ini hanya buffer scalar f32 padat, tanpa atribut, metadata, allocator atau cache. Ini bukan hasil pengukuran RSS, bukan perbandingan SVO, dan bukan bukti ukuran scene prosedural tertentu.

## Kriteria benchmark sebelum klaim “ringan” atau “AI-friendly”

[Hypothesized] Preregister dataset dan ambang sebelum membandingkan implementasi. Simpan seed dan manifest lingkungan. Pisahkan benchmark inti yang dapat dijalankan sekarang dari bake/render yang belum tersedia.

| Dimensi | Prosedur dan metrik yang direncanakan | Batas interpretasi |
|---|---|---|
| Corpus | Sphere tunggal; box; union terpisah; intersection bertumpuk; subtraction berlubang; DAG berbagi; rantai dalam; kisi berulang; kasus tangensial/berimpit; fitur tipis; scan/organik untuk tahap lanjut. Skala 10², 10³, 10⁴ node. | [Hypothesized] Angka skala adalah target workload, bukan kemampuan mesin yang telah terbukti. |
| Correctness inti | Oracle analitik primitif; uji tanda boolean di luar epsilon; invariansi translasi; round-trip JSON/biner; kesetaraan evaluator referensi/tape; input acak dengan seed tetap. | [Derived] Selisih terhadap true distance pada CSG tidak otomatis bug bila kontraknya bound. |
| Robustness | ID duplikat, child/root hilang, siklus, unreachable node, truncated binary, tag/versi tidak dikenal, ukuran berlebihan, NaN/Infinity, extent buruk dan depth ekstrem harus menghasilkan error terstruktur, bukan panic. | [Hypothesized] Lakukan fuzzing parser/decoder kemudian; jangan mengklaim sudah aman sebelum dijalankan. |
| Ukuran | Byte JSON mentah; JSON terkompresi dengan codec/level dicatat; biner mentah dan terkompresi; metadata, atribut, bobot neural dan model bersama dihitung terpisah maupun total. | [Derived] Biner versus JSON mentah saja bukan tes kompresi yang adil. Laporkan amortisasi bobot neural per corpus. |
| Memori | Peak RSS, jumlah alokasi dan resident runtime setelah load; grid padat versus sparse versus program pada kualitas sama. | [Derived] Byte file tidak sama dengan resident memory. |
| Waktu inti | Parse, validasi, compile, encode/decode; ns/query dan points/s scalar/batch; jumlah node dikunjungi; cold/warm dipisahkan; 10⁶ titik seeded sebagai batch uji. | [Hypothesized] Ukur release build, compiler/flags, CPU/OS/thread count; laporkan median, p95 dan variasi dari minimal 30 run. Ini protokol, bukan hasil. |
| Editing | Ubah satu radius/pusat/operand; hitung node terdampak, compile ulang, byte patch, stabilitas ID dan p50/p95 latency. | [Derived] Cache rebuilding masuk biaya edit; jangan hanya mengukur steady-state query. |
| Kualitas lintas representasi | Tahap lanjut: error permukaan dua arah, p95/maks jarak pada sampel, volume disagreement, hilangnya fitur tipis, topology dan normal. Sweep resolusi/kuantisasi/toleransi. | [Derived] Sampling Hausdorff hanya estimasi; tak membuktikan galat maksimum global atau manifold. Butuh mesh/reference yang belum dibuat. |
| Editing AI | Set tugas tetap: perbesar sphere 10%, pindahkan satu bagian, ganti operand boolean, tambah lubang. Prompt/schema/model/seed dicatat; ulangi; ukur parse-valid, task-correct, perubahan tak terkait, token dan retry. | [Hypothesized] Bandingkan JSON ber-ID dengan daftar node berindeks pada model dan instruksi sama. Kebenaran geometri memerlukan oracle, bukan penilaian LLM saja. |
| Runtime lanjutan | Jika renderer kelak ada: ms/frame, langkah/ray, miss rate terhadap oracle, silhouette error, energi dan memori. Jika mesh ada: waktu ekstraksi+codec+decode. | [Derived] Belum dapat dinilai dari evaluasi titik; tidak ada klaim FPS, GPU atau mesh dari riset ini. |

[Hypothesized] Kriteria penghentian umum: gugurkan optimasi bila mengubah semantik, menambah ketidakamanan decoder, atau hanya menang pada corpus yang dirancang untuknya. Untuk klaim AI-friendly, minta perbaikan task-correct tanpa kenaikan perubahan tak terkait pada tugas yang sama; laporkan interval ketidakpastian, jangan menyimpulkan dari satu contoh.

## Apa yang sumber mendukung, dan apa yang tidak

[Derived] Hart [1] mendukung sphere tracing menggunakan distance/bound serta kondisi Lipschitz, bukan janji bahwa field sembarang aman. Riset Seyb dkk. [2] menunjukkan deformasi forward memerlukan teknik tambahan berbasis integrasi ODE; ia tidak menjadi alasan menambahkan deformasi sekarang. Translasi primitif tidak memerlukan metode tersebut.

[Derived] Sparse voxel DAG [3] berbagi region spasial identik dari representasi voxel. DAG CSG berbagi ekspresi konstruksi. Keduanya memakai graf asiklik, tetapi kompresi dan workload berbeda. Klaim paper tentang pengurangan node pada scene uji mereka tidak boleh dipindahkan ke model Rust ini, warna/material arbitrer, atau editing dinamis.

[Derived] DeepSDF [4] menunjukkan representasi terpelajar untuk kelas bentuk dan completion. Ia tidak membuktikan bahwa agent lebih mudah menulis bobot neural daripada JSON, bahwa booleannya eksak, atau bahwa inference CPU proyek ini ringan. meshoptimizer [5] mendukung kelayakan codec mesh, bukan bahwa mesh akan lebih kecil daripada program untuk setiap bentuk. Angka performa vendor tidak direproduksi dalam sesi ini.

[Hypothesized] Batas produk yang jujur: inti ini cocok sebagai kernel prosedural terukur, belum pengganti Blender/CAD, belum sculpting umum, belum engine fisika, dan belum workflow scan-to-asset. Material, UV, animasi, constraints parametrik, interoperabilitas, toleransi manufaktur dan editing kolaboratif berada di luar evaluasi saat ini.

## Sumber terverifikasi dan catatan akses

[Tested] Riset memakai **papers MCP dan web**. Verifikasi berarti identitas karya dan isi abstrak/dokumentasi yang berhasil diambil cocok; bukan semua isi PDF dibaca, bukan peer-review ulang, dan bukan reproduksi eksperimen.

1. **John C. Hart (1996).** *Sphere tracing: a geometric method for the antialiased ray tracing of implicit surfaces*. The Visual Computer 12(10), 527–545. [DOI 10.1007/s003710050084](https://doi.org/10.1007/s003710050084). [Abstrak dan ringkasan yang berhasil diambil](https://www.doc.gold.ac.uk/~ffl/Refs/CompGraphics/Render/HartJC.html). [Tested] papers MCP `get_paper_by_doi` mengembalikan metadata Crossref dan Semantic Scholar yang cocok. Salah satu timestamp hasil metadata bergeser ke 1995, tetapi tahun bibliografi kedua sumber adalah 1996; gunakan 1996.
2. **Dario Seyb, Alec Jacobson, Derek Nowrouzezahrai, Wojciech Jarosz (2019).** *Non-linear sphere tracing for rendering deformed signed distance fields*. ACM Transactions on Graphics 38(6), 1–12. [DOI 10.1145/3355089.3356502](https://doi.org/10.1145/3355089.3356502). [Tested] papers MCP mengembalikan judul, penulis, tahun dan abstrak melalui Crossref/Semantic Scholar. Dua fetch halaman penulis Dartmouth, dengan dan tanpa `www`, gagal `TypeError: fetch failed`; isi paper tidak diklaim telah dibaca penuh.
3. **Viktor Kämpe, Erik Sintorn, Ulf Assarsson (2013).** *High resolution sparse voxel DAGs*. ACM Transactions on Graphics 32(4), 1–13. [DOI 10.1145/2461912.2462024](https://doi.org/10.1145/2461912.2462024). [Tested] pencarian Crossref berbasis judul pertama justru memilih tesis *Moxel DAGs*, bukan paper ini. Identitas dan abstrak paper yang benar kemudian diverifikasi lewat `get_paper_by_doi`. Fetch ACM menghasilkan HTTP 403; halaman [arsip SIGGRAPH](https://history.siggraph.org/learning/high-resolution-sparse-voxel-dags-by-kampe-sintorn-and-assarsson/) dapat diakses tetapi hasil ekstraksi terpotong oleh navigasi. Klaim mekanisme berasal dari abstrak Crossref, bukan halaman 403.
4. **Jeong Joon Park, Peter Florence, Julian Straub, Richard Newcombe, Steven Lovegrove (2019).** *DeepSDF: Learning Continuous Signed Distance Functions for Shape Representation*. CVPR, 165–174. [DOI 10.1109/CVPR.2019.00025](https://doi.org/10.1109/CVPR.2019.00025). [Laman resmi CVF dengan abstrak dan tautan paper](https://openaccess.thecvf.com/content_CVPR_2019/html/Park_DeepSDF_Learning_Continuous_Signed_Distance_Functions_for_Shape_Representation_CVPR_2019_paper.html). [Tested] papers MCP `search_crossref` dan web CVF cocok. Kegagalan arXiv query `ti:"DeepSDF"` HTTP 400 dilaporkan oleh agen utama dari pencarian sebelumnya, tidak diuji ulang dalam subriset ini; pencarian sederhana Crossref berhasil sehingga tidak dipakai sebagai penghalang riset.
5. **meshoptimizer**, dokumentasi resmi. [Situs proyek](https://meshoptimizer.org/). [Tested] web fetch HTTP 200 mengonfirmasi codec vertex/index, kuantisasi, optimasi locality, dan rujukan crate Rust. Dokumentasi menyatakan codec vertex lossless terhadap input; kuantisasi atau filter sebelumnya bisa lossy. Tidak ada uji throughput lokal. Halaman panjang terpotong setelah bagian meshlet, sehingga kesimpulan dibatasi pada bagian yang terbaca.
6. **Matthew Keeter, Fidget**, dokumentasi proyek Rust. [README resmi](https://raw.githubusercontent.com/mkeeter/fidget/main/README.md), [repositori](https://github.com/mkeeter/fidget). [Tested] web fetch HTTP 200 mengonfirmasi deduplikasi ekspresi, graph-to-tape, simplifikasi interval, interpreter/JIT CPU dan bytecode. Proyek menyebut dirinya eksperimental dan berlisensi MPL-2.0. Ini preseden dan calon pembanding; tidak diinstal, tidak dibenchmark, dan bukan keputusan adopsi dependensi. URL branch `main` bergerak; pin commit sebelum eksperimen reproduksi.

[Derived] Jumlah sitasi bibliografi tidak digunakan sebagai metrik kebenaran. Hasil papers MCP dari Semantic Scholar/Crossref bisa berbeda dalam hitungan sitasi dan timestamp; judul, penulis, tahun, DOI, serta isi yang mendukung klaim lebih relevan.

## Langkah berikutnya

[Hypothesized] Satu eksperimen paling murah: pada evaluator CPU referensi, jalankan corpus primitif/boolean dengan query titik analitik dan round-trip JSON/biner, termasuk counterexample intersection dan A\\A. Setelah kontrak tanda, zero-set dan decoder lulus, ukur baseline ukuran/load/query sebelum memilih tape, cache atau representasi lain. Tidak diperlukan UI, GPU, atau mesh untuk membantah asumsi awal tersebut.
