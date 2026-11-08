# Komponen halaman peta

Dari `replica/recon.md` (S02 sampai S13) dan screenshot di `replica/screens/`. Hanya pola, tata letak dan perilaku yang ditiru. Semua teks ditulis sendiri, ikon dari Lucide (sudah terpasang di arsgis sebagai `lucide-react`), warna dari `tokens.json`.

Semua permukaan melayang memakai `--map-float` dengan blur `--map-float-blur`, radius pil atau `--map-radius-lg`, dan bayangan `--map-shadow-float`.

Yang dipakai ulang dari arsgis dan tidak dibuat ulang: `AuthPanel`, `Legend`, `InfoPanel`, `DesaInfo`, halaman Dashboard dan Admin.

## Perubahan tata letak yang diminta (sekarang → target)

| bagian | arsgis sekarang | target (mengikuti pola referensi) |
| --- | --- | --- |
| pencarian | kapsul gelap di tengah atas | kapsul putih transparan, lebar `--map-search-width`, tinggi 36, tombol ikon di dalam di kanan |
| layer cepat | tidak ada | baris chip di bawah pencarian, bisa digeser ke samping |
| tombol alat | kolom tombol bulat di kanan atas | deretan tombol bulat melayang di tengah bawah |
| zoom | dua tombol bulat terpisah di kiri bawah | satu kapsul di kanan bawah |
| panel | kartu kecil di kiri | panel menempel tepi dengan header (ikon, judul, minimalkan, tutup), bagian berupa accordion |
| daftar layer | panel sederhana dengan kotak centang | tabel layer dengan pencarian, filter, lencana jenis, dan jumlah |
| label koordinat | tidak ada | label lintang dan bujur kecil di tepi peta (opsional, terakhir) |

## SearchBar

```
SearchBar
  variants  default
  sizes     tinggi 36, lebar clamp(280px, 41vw, 420px), posisi tengah atas, top 12
  states    idle, focus-visible (ring 2px accent), has-value (tombol hapus x), loading, results-open, no-results, error
  tokens    bg float, text text, placeholder text-muted, radius pill, shadow float, font base
  a11y      <input role="combobox" aria-expanded aria-controls>, hasil di <ul role="listbox">, panah atas/bawah memilih, Enter membuka, Esc menutup,
            tombol hapus punya aria-label "Hapus pencarian"
  used on   S02, S04
```

Hasil pencarian: dikelompokkan per tingkat (judul kecil huruf besar `xs` muted), tiap baris dua teks (nama, lalu induk dan kode dalam `xs` muted). Dropdown lebar sama dengan kapsul, radius `md`, tinggi maksimum 60% layar dengan gulir.

## LayerChip

```
LayerChip
  variants  default, dengan tag BARU
  sizes     tinggi 28, gap 8, teks xs/500
  states    off, on (latar accent-soft, teks accent-strong, border accent), hover, focus-visible, loading
  tokens    bg float, radius pill, tag BARU: bg danger-strong teks on-danger, 10px
  a11y      <button aria-pressed>, label teks lengkap (bukan hanya ikon), tag BARU punya teks tersembunyi "baru"
  used on   S02, S12
```

Baris chip: `overflow-x: auto`, tanpa scrollbar visual tetapi bisa digeser dengan sentuhan dan tombol panah keyboard (`tabindex` pada tiap chip). Isinya dari `layer_catalog` yang `chip = true`, urut `chip_urut`.

## RoundToolbar (deretan tombol bawah)

```
RoundToolbar
  variants  tombol bulat berderet, jumlah menyesuaikan arsgis (lihat di bawah)
  sizes     36 px bulat, gap 6, bottom 12, tengah horizontal
  states    idle, hover (border accent), active/pressed (latar accent, ikon on-accent), focus-visible, disabled
  tokens    bg float, ikon text, radius pill, shadow float
  a11y      <button aria-pressed aria-label>, tooltip muncul saat hover dan fokus (bukan hanya hover), tab berurutan kiri ke kanan
  used on   S02
```

Isi tombol untuk arsgis (urutan kiri ke kanan): Layer (daftar), Alat gambar, Cakupan SPM, Impor KML (editor dan admin), Dashboard (tautan), Bantuan. Tombol yang tidak berlaku untuk peran pengguna tidak dirender. Di layar sempit (di bawah 480) deretan bisa digeser horizontal.

## ZoomControl

```
ZoomControl
  variants  satu kapsul dua tombol, pemisah tipis
  sizes     tiap tombol 36, kanan 12, bawah 12
  states    idle, hover, focus-visible, disabled (di zoom min dan maks)
  tokens    bg float, radius pill, shadow float
  a11y      <button aria-label="Perbesar"> dan "Perkecil", disabled sungguhan di batas zoom
  used on   S02
```

## SidePanel

```
SidePanel
  variants  kiri, kanan (satu per sisi)
  sizes     lebar clamp(280px, 30vw, 360px), tinggi penuh, header 40
  states    open, minimized (hanya header), closed, loading, error, empty
  tokens    bg bg, border sisi dalam border, shadow panel, radius lg di sudut dalam
  a11y      <aside aria-label>, header berisi <h2>, tombol Minimalkan dan Tutup punya aria-label, Esc menutup panel yang sedang fokus,
            fokus pindah ke judul saat dibuka dan kembali ke tombol pemicu saat ditutup
  used on   S05, S06, S07, S09, S10
```

Di bawah 480: panel menjadi lembar bawah (bottom sheet) selebar layar, tinggi maksimum 70%, tombol Tutup selalu ada.

Dua panel bisa terbuka bersamaan (satu kiri, satu kanan), seperti referensi. Di layar sempit hanya satu.

## AccordionSection

```
AccordionSection
  variants  dengan ikon, tanpa ikon
  sizes     tinggi tertutup 44, ikon dalam kotak 28, padding 10, gap antarbagian 8
  states    collapsed, expanded, hover, focus-visible, disabled
  tokens    bg bg, border border, radius md, judul sm/600 text, subjudul xs text-muted, kotak ikon bg accent-soft ikon accent-strong
  a11y      pola disclosure: <button aria-expanded aria-controls> di dalam <h3>, panel punya id, chevron dekoratif aria-hidden
  used on   S07, S09
```

Isi bagian Alat untuk arsgis: Gambar (Persegi panjang dulu), Ukur, Ekspor. Kisi tombol alat: dua kolom, tombol `ToolButton` tinggi 40 dengan ikon dan label.

## ToolButton

```
ToolButton
  variants  default, primary (satu per kelompok), pressed (alat gambar aktif)
  sizes     tinggi 40, radius md, teks sm
  states    default, hover, pressed, focus-visible, disabled, loading (label tetap dibaca pembaca layar)
  tokens    default: bg bg + border border; primary dan pressed: bg accent-strong + teks on-accent
  a11y      <button>, aria-pressed untuk alat gambar, disabled saat tidak ada yang bisa dilakukan, teks bantuan di bawah saat mode aktif
  used on   S09, S09c, S11
```

## LayerRow dan lencana jenis

```
LayerRow
  variants  tabel (daftar panjang), kartu (panel aktif)
  sizes     tinggi min 44, nama sm/500, sumber xs muted, lencana di kanan
  states    off, on, hover, focus-visible, error (layer gagal dimuat), long-content (nama 60 karakter membungkus)
  a11y      <label> membungkus <input type="checkbox">, nama dan sumber dalam teks yang sama, lencana punya teks (bukan hanya warna)
  used on   S05, S06

TypeBadge
  variants  feature (ungu), layanan atau arcgis (biru), vektor (teal)
  tokens    badge-*-bg dan badge-*-text, radius pill, xs/600, padding 2x8
```

Bagian tabel: kotak pencarian layer, tiga pilihan filter (kategori, jenis, urutan) dalam satu baris yang membungkus, jumlah hasil (`xs` muted, `aria-live="polite"`), daftar bergulir. Daftar lebih dari 100 baris memakai windowing.

## FilterSelect

```
FilterSelect
  variants  default
  sizes     tinggi 32, radius md, teks sm
  states    default, hover, focus-visible, disabled
  a11y      <select> asli dengan <label> terlihat atau aria-label; nilai terpilih tidak terpotong
  used on   S06
```

## GridLabel (label koordinat tepi)

```
GridLabel
  sizes     teks 10/xs, padding 1x4, radius sm, bg float
  states    mengikuti geser dan zoom
  a11y      aria-hidden (dekoratif, koordinat tepat tersedia di Cek Lokasi)
  used on   S02
```

Dikerjakan terakhir dan bisa dimatikan. Perlu garis kisi tipis dan label yang menyesuaikan zoom.

## Toast dan status

```
Toast
  variants  info, sukses, galat
  states    muncul, hilang otomatis 4s, ditutup manual
  tokens    bg float, teks text, bilah kiri 3px warna status (accent, success, danger-strong)
  a11y      role="status" (info dan sukses) atau role="alert" (galat), tidak mencuri fokus
  used on   S09c (ekspor selesai), S02 (galat muat)
```

## Tidak dibuat

- Kartu donasi: arsgis tidak punya model donasi.
- Pita "Citra satelit", AI dan modul Geo*: di luar irisan (lihat recon).
- Logo, ikon dan teks milik referensi: tidak dipakai. Semua ikon Lucide, semua teks ditulis ulang.

## Primitif

arsgis tidak memakai Storybook. Primitif (SearchBar, LayerChip, RoundToolbar, ZoomControl, SidePanel, AccordionSection, ToolButton, TypeBadge) dibangun di `/replica-build` bersama layarnya, dengan CSS dari `tokens.css`, lalu diperiksa lewat tangkapan layar halaman `/peta`.
