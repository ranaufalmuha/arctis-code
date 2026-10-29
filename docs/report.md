# Laporan performa ArctisCode dan perbandingan editor

Tanggal pengukuran: 29 Oktober 2026. Laporan ini memakai source ArctisCode pada workspace saat itu (basis Git `e69a673` dengan perubahan lokal yang belum di-commit). Angka di bawah adalah hasil satu sesi pada mesin pengguna, bukan benchmark lintas mesin.

## Hasil yang terukur

Build: `cargo build --release --bin ArctisCode`; Rust 1.94.0; macOS 27.0.1, arsitektur arm64. Binary release hasil build berukuran **5.419.760 byte (5,17 MiB)**. Ini ukuran executable saja, bukan ukuran paket instalasi atau RAM. SHA-256: `6aacd8ff9ef5bee9cd4452a0a06691107acfd2e8fd63d4a7ed64e18bdc9127a6`.

ArctisCode dibuka pada folder sementara yang kosong, tanpa tab dan tanpa interaksi, lalu diambil satu sampel RSS setelah 5 detik. Setiap mode dijalankan tiga kali. Mode default memakai Neovim 0.12.0; total mencakup proses ArctisCode dan child Neovim. Mode native menggunakan `ARCTIS_NO_NVIM=1`. Angka dinyatakan dalam MiB (1 MiB = 1.024 KiB).

| Mode ArctisCode | Run 1 | Run 2 | Run 3 | Median | Cakupan |
|---|---:|---:|---:|---:|---|
| Default, dengan Neovim | 100,77 MiB | 128,27 MiB | 99,52 MiB | **100,77 MiB** | Jumlah RSS proses aplikasi dan Neovim |
| `ARCTIS_NO_NVIM=1` | 101,39 MiB | 96,81 MiB | 118,86 MiB | **101,39 MiB** | RSS proses aplikasi |

Median kedua mode hampir sama dalam sampel kecil ini. Variasi antarrun lebih besar daripada selisih mediannya; angka ini **tidak membuktikan** bahwa Neovim tidak menambah memori. Pengukuran dilakukan pada sistem yang sedang dipakai, tanpa kontrol cache, thermal state, atau aplikasi lain. Penjumlahan RSS antarproses dapat menghitung halaman bersama lebih dari sekali; RSS juga berbeda dari *memory footprint* Activity Monitor, alokasi GPU, dan memori puncak.

| Editor | Observasi RAM | Kondisi dan batas |
|---|---:|---|
| ArctisCode | Median **100,77 MiB RSS** | Tiga run release, folder kosong, setelah 5 detik, termasuk Neovim. |
| VS Code | **697,02 MiB RSS** | Satu snapshot dari 20 proses pada pohon proses VS Code yang sudah berjalan, termasuk child yang teridentifikasi. Workspace, jumlah jendela, ekstensi, dan aktivitas tidak dikendalikan. Ini bukan kondisi yang sebanding dengan ArctisCode. |
| Zed | **Belum diukur** | Tidak ada proses Zed yang berjalan dalam sesi pengukuran ini. |

Karena beban kerja berbeda, angka VS Code tidak dapat dipakai untuk menghitung rasio performa atau menyimpulkan bahwa ArctisCode sekian kali lebih ringan. Tidak ada hasil startup, CPU idle, latensi buka file, frame rate, penggunaan memori proyek besar, atau memory footprint macOS yang terukur dalam sesi ini.

## Perbandingan produk

| Aspek | ArctisCode saat ini | VS Code | Zed |
|---|---|---|---|
| Arsitektur | Rust, `egui`/`eframe`, backend Neovim opsional; UI tidak memakai WebView. | Desktop berbasis Electron; model proses dan ekstensi dapat menambah proses. | Rust dan framework UI GPUI dengan rendering GPU. [Sumber](https://zed.dev/blog/zed-1-0) |
| Cakupan | Editor, Explorer, beberapa window, preview gambar, Settings, dan task board lokal. | Editor, terminal, debugger, Git, dan ekosistem ekstensi luas. [Sumber](https://code.visualstudio.com/docs/getstarted/overview) | Editor native, bahasa dan language server, ekstensi, serta kolaborasi. [Bahasa](https://zed.dev/docs/languages), [kolaborasi](https://zed.dev/docs/collaboration/overview) |
| Memori | Terukur pada workspace kosong seperti tabel di atas; belum diukur untuk proyek dan file besar. | Sangat bergantung pada workspace, jendela, ekstensi, language server, dan workload; snapshot di atas bukan baseline bersih. | Belum diukur pada mesin dan workload ini. |
| Kebutuhan sistem | Belum ada persyaratan resmi yang tervalidasi; Neovim dibutuhkan untuk backend tersebut, dengan fallback native. | Microsoft merekomendasikan 1 GB RAM untuk sistem; ini **bukan** angka pemakaian aplikasi. [Sumber](https://code.visualstudio.com/docs/supporting/requirements) | Persyaratan berbeda menurut platform dan GPU; dokumentasi instalasi tidak memberikan angka RSS untuk workload ini. [Sumber](https://zed.dev/docs/installation) |

## Kelebihan dan kekurangan

| Editor | Kelebihan | Kekurangan / trade-off |
|---|---|---|
| ArctisCode | Binary release kecil pada build ini; UI native tanpa runtime web; Explorer memuat isi folder secara lazy; task board disimpan lokal per workspace; backend Neovim dapat dinonaktifkan. | Masih tahap pengembangan: fitur editor, integrasi bahasa, debugging, terminal internal, Git UI, dan ekosistem ekstensi belum setara editor matang. Belum ada benchmark yang membuktikan keunggulan terhadap VS Code atau Zed pada workload yang sama. |
| VS Code | Fitur development lengkap, banyak ekstensi, dokumentasi dan integrasi Git, debugging, serta terminal. | Cakupan fitur dan ekstensi bisa menambah proses dan penggunaan resource; hasil aktual bergantung konfigurasi dan proyek. |
| Zed | Rust native, GPU rendering, language server, ekstensi, dan kolaborasi real-time. | Memerlukan dukungan GPU/platform sesuai persyaratannya; konsumsi resource dan performa relatif terhadap ArctisCode belum diukur di sini. |

## Temuan teknis dan risiko performa ArctisCode

| Area | Temuan dari kode | Dampak yang perlu diukur |
|---|---|---|
| Explorer | Membaca child folder saat diperlukan dan menyimpan cache; pencarian descendant dibatasi dan melewati direktori berat tertentu. | Berpotensi mengurangi kerja awal; belum ada angka waktu buka proyek 100 atau 5.000 file. |
| Editor dan gutter | Editor menghitung jumlah baris dari seluruh teks; gutter membangun dan memproses nomor baris. Saat edit, ada penyalinan konten dan sinkronisasi Neovim. | File besar dan banyak baris dapat menaikkan CPU, alokasi, dan latensi input; perlu profiling, bukan asumsi. |
| Syntax highlighting | Highlight dibatasi hingga 512 KiB; file lebih besar memakai tampilan tanpa highlight. | Membatasi biaya analisis dengan mengorbankan fitur pada file besar. |
| Task board | Daftar kartu disaring dan diurutkan untuk kolom; perubahan board disimpan ke JSON workspace. | Perlu diuji pada ratusan hingga ribuan kartu dan saat drag. |
| Preview | Decode gambar memakai worker dan gambar besar diperkecil. | Beban decode, tekstur GPU, dan cache tetap perlu diukur dengan gambar nyata. |

## Target dan langkah benchmark berikutnya

Target di [PERF.md](PERF.md) adalah target project, **bukan hasil**: first frame < 800 ms, proyek 100 file interaktif < 300 ms, idle RSS < 150 MB, dan proyek 5.000 file RSS < 300 MB. Laporan ini tidak memverifikasi target tersebut; sampel ArctisCode di sini hanya satu titik setelah 5 detik, bukan pengukuran idle berkelanjutan. Catatan `PERF.md` yang merujuk `tests/perf.rs` juga belum dapat diverifikasi karena file tersebut tidak ada pada workspace ini.

Untuk perbandingan yang adil, gunakan mesin, resolusi, versi OS, workspace, file, jumlah tab, dan skenario yang sama untuk ketiga editor. Catat versi masing-masing, konfigurasi ekstensi dan language server, jumlah window, serta proses tambahan. Jalankan minimal lima kali setelah kondisi cache ditentukan, lalu laporkan median dan rentang untuk:

1. RSS seluruh pohon proses dan *memory footprint* macOS saat workspace kosong dan saat proyek 5.000 file sudah tenang selama 30 detik.
2. Waktu start hingga frame pertama dan hingga Explorer interaktif, memakai penanda waktu yang sama untuk tiap aplikasi.
3. Waktu buka file 50 KiB serta latensi input dan memori pada file besar; catat apakah fitur highlighting dan language server aktif.
4. CPU idle, penggunaan GPU, serta frame time saat scroll dan drag dalam proyek yang sama.

Jalankan skenario baseline tanpa ekstensi pihak ketiga, lalu skenario kerja nyata dengan daftar ekstensi yang dicatat. Gunakan Activity Monitor atau alat yang mengukur footprint, dan catat RSS setiap proses serta child-nya secara terpisah. Jangan menjumlahkan RSS lalu menyebutnya memori fisik unik tanpa memperhitungkan halaman bersama.
