import './preflight.css'
import { useQuery } from '@tanstack/react-query'
import { fetchMe } from './api'
import { AuthPanel } from './Auth'

// Warna jenis fitur, sama dengan peta dan dashboard.
const LEGEND = [
  { label: 'Pipa', color: '#1d4ed8' },
  { label: 'Valve', color: '#111111' },
  { label: 'Reservoir', color: '#0f766e' },
  { label: 'Sambungan rumah', color: '#a16207' },
]

const ALUR = [
  {
    judul: 'Petakan jaringan',
    isi: 'Pipa, valve, reservoir, dan sambungan rumah tampil per unit SPAM, di atas batas desa BPS.',
  },
  {
    judul: 'Catat di lapangan',
    isi: 'Gangguan dicatat dengan lokasi dan foto. Jadwal perawatan dan prioritas rehabilitasi tercatat per aset.',
  },
  {
    judul: 'Laporkan capaian',
    isi: 'Cakupan layanan dihitung per dusun dan per desa, lalu bisa dibaca sebagai statistik dan laporan.',
  },
]

const FITUR = [
  {
    judul: 'Peta per unit SPAM',
    isi: 'Pilih unit, lihat pipa dan titik layanannya. Klik fitur untuk melihat atribut dan riwayatnya.',
  },
  {
    judul: 'Impor data',
    isi: 'Berkas KML dan KMZ masuk ke unit SPAM setelah pratinjau laporan. Riwayat batch bisa dibatalkan.',
  },
  {
    judul: 'Kelengkapan data per desa',
    isi: 'Desa ditandai bila data batas, pokmas, atau sambungan rumahnya belum lengkap.',
  },
  {
    judul: 'Pelanggan per dusun',
    isi: 'Nomor meter, golongan tarif, dan status aktif atau putus dicatat per dusun.',
  },
  {
    judul: 'Cakupan layanan',
    isi: 'Dihitung dari sambungan rumah yang sudah punya pelanggan aktif, per dusun dan per desa.',
  },
  {
    judul: 'Gangguan dan perawatan',
    isi: 'Laporan gangguan dengan foto dan status, jadwal perawatan aset, dan prioritas rehabilitasi.',
  },
]

/** Halaman depan arsgis: apa yang dikerjakan aplikasi, bagaimana alurnya, dan ke mana pengunjung bisa pergi. */
export function LandingPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })

  return (
    <div className="min-h-svh bg-[#f5f5f4] text-[#1f2937]">
      <header className="enter flex items-center justify-between gap-4 border-b border-black/10 bg-white px-4 py-3 md:px-10">
        <a href="/" className="text-base font-semibold tracking-tight">
          arsgis
        </a>
        <nav aria-label="Utama" className="flex items-center gap-5 text-sm">
          <a href="/dashboard" className="hover:underline underline-offset-4">Dashboard</a>
          <a href="/peta" className="hover:underline underline-offset-4">Peta</a>
          <AuthPanel user={me.data ?? null} inline />
        </nav>
      </header>

      <main>
        <section className="mx-auto grid max-w-6xl gap-10 px-4 py-12 md:grid-cols-[1.05fr_1fr] md:items-center md:gap-14 md:px-10 md:py-20">
          <div className="enter enter-2 flex min-w-0 flex-col gap-5">
            <p className="text-sm text-[#4b5563]">Cianjur · Sistem penyediaan air minum perdesaan</p>
            <h1 className="text-3xl font-semibold leading-[1.15] tracking-tight md:text-5xl md:leading-[1.1]">
              Pipa, pelanggan, dan gangguan tiap unit SPAM, dalam satu peta.
            </h1>
            <p className="max-w-prose text-base leading-relaxed text-[#4b5563]">
              Dipakai tim dinas dan pengelola desa untuk memantau jaringan, mencatat pelanggan, dan menindaklanjuti
              gangguan di lapangan.
            </p>
            <div className="flex flex-wrap gap-3">
              <a
                href="/dashboard"
                className="rounded-md bg-[#b8431f] px-5 py-2.5 text-sm font-medium text-white hover:bg-[#9c3a1b]"
              >
                Buka dashboard
              </a>
              <a
                href="/peta"
                className="rounded-md border border-black/20 px-5 py-2.5 text-sm font-medium text-[#1f2937] hover:bg-black/5"
              >
                Lihat peta
              </a>
            </div>
          </div>

          <figure className="enter enter-3 flex min-w-0 flex-col gap-3">
            <img
              src="/ilustrasi-jaringan.webp"
              alt="Contoh jaringan pipa dengan valve, reservoir, dan sambungan rumah"
              width={1280}
              height={720}
              className="enter enter-4 h-auto w-full rounded-md border border-black/10 bg-white"
            />
            <figcaption>
              <ul className="flex flex-wrap gap-x-5 gap-y-2 text-sm text-[#4b5563]">
                {LEGEND.map((l) => (
                  <li key={l.label} className="flex items-center gap-2">
                    <span className="inline-block h-2.5 w-2.5 rounded-full" style={{ background: l.color }} />
                    {l.label}
                  </li>
                ))}
              </ul>
            </figcaption>
          </figure>
        </section>

        <section aria-labelledby="alur" className="mx-auto max-w-6xl px-4 pb-14 md:px-10 md:pb-20">
          <h2 id="alur" className="text-xl font-semibold tracking-tight md:text-2xl">
            Cara kerjanya
          </h2>
          <ol className="mt-6 divide-y divide-black/10 border-y border-black/10">
            {ALUR.map((langkah) => (
              <li key={langkah.judul} className="grid gap-2 py-5 md:grid-cols-[14rem_1fr] md:gap-10">
                <h3 className="font-semibold">{langkah.judul}</h3>
                <p className="text-sm leading-relaxed text-[#4b5563]">{langkah.isi}</p>
              </li>
            ))}
          </ol>
        </section>

        <section aria-labelledby="fitur" className="mx-auto max-w-6xl px-4 pb-16 md:px-10 md:pb-24">
          <h2 id="fitur" className="text-xl font-semibold tracking-tight md:text-2xl">
            Isi aplikasi
          </h2>
          <dl className="mt-6 grid gap-x-12 md:grid-cols-2">
            {FITUR.map((f) => (
              <div key={f.judul} className="grid gap-1 border-t border-black/10 py-5">
                <dt className="font-semibold">{f.judul}</dt>
                <dd className="text-sm leading-relaxed text-[#4b5563]">{f.isi}</dd>
              </div>
            ))}
          </dl>
        </section>
      </main>

      <footer className="border-t border-black/10 bg-white">
        <div className="mx-auto grid max-w-6xl gap-8 px-4 py-10 md:grid-cols-3 md:px-10">
          <div className="flex flex-col gap-2">
            <p className="font-semibold">arsgis</p>
            <p className="text-sm text-[#4b5563]">Aplikasi peta dan data SPAM perdesaan untuk Kabupaten Cianjur.</p>
          </div>
          <nav aria-label="Tautan" className="flex flex-col gap-2 text-sm">
            <a href="/peta" className="w-fit hover:underline underline-offset-4">Peta jaringan</a>
            <a href="/dashboard" className="w-fit hover:underline underline-offset-4">Dashboard</a>
            <a href="/masuk" className="w-fit hover:underline underline-offset-4">Masuk</a>
          </nav>
          <address className="flex flex-col gap-2 text-sm not-italic text-[#4b5563]">
            <p className="font-medium text-[#1f2937]">Dinas Perumahan dan Kawasan Permukiman</p>
            <p>Jalan Adi Sucipta No. 7, Cianjur 43211</p>
            <p>Telepon (0263) 261210</p>
          </address>
        </div>
        <div className="border-t border-black/10 px-4 py-4 text-xs text-[#4b5563] md:px-10">
          Peta jaringan dan data pelanggan hanya bisa dibuka dengan akun.
        </div>
      </footer>
    </div>
  )
}
