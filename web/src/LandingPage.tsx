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

// Alur kerja dari lapangan sampai laporan. Setiap langkah menunjuk ke bagian yang sudah ada di aplikasi.
const ALUR = [
  {
    judul: 'Petakan jaringan',
    isi: 'Pipa, valve, reservoir, dan sambungan rumah tampil per unit SPAM di atas peta, lengkap dengan batas desa BPS.',
  },
  {
    judul: 'Catat di lapangan',
    isi: 'Gangguan dicatat dengan lokasi dan foto. Jadwal perawatan dan prioritas rehabilitasi ikut tercatat per aset.',
  },
  {
    judul: 'Laporkan capaian',
    isi: 'Cakupan layanan dihitung per dusun dan per desa, lalu tersedia sebagai statistik dan laporan.',
  },
]

// Fitur dikelompokkan menurut pekerjaan yang dilakukan, bukan menurut halaman.
const GRUP = [
  {
    judul: 'Peta dan aset',
    item: [
      {
        judul: 'Peta per unit SPAM',
        isi: 'Pilih unit, lalu lihat pipa dan titik layanannya. Klik fitur untuk melihat atribut dan riwayatnya.',
      },
      {
        judul: 'Impor data',
        isi: 'Impor berkas KML dan KMZ ke unit SPAM, dengan pratinjau laporan dan riwayat batch yang bisa dibatalkan.',
      },
      {
        judul: 'Kelengkapan data per desa',
        isi: 'Desa ditandai bila data batas, pokmas, atau sambungan rumahnya belum lengkap.',
      },
    ],
  },
  {
    judul: 'Layanan dan pelanggan',
    item: [
      {
        judul: 'Pelanggan per dusun',
        isi: 'Pelanggan dicatat per dusun dengan nomor meter, golongan tarif, dan status aktif atau putus.',
      },
      {
        judul: 'Cakupan layanan',
        isi: 'Cakupan dihitung dari sambungan rumah yang sudah punya pelanggan aktif, per dusun dan per desa.',
      },
    ],
  },
  {
    judul: 'Operasi dan laporan',
    item: [
      {
        judul: 'Gangguan dan perawatan',
        isi: 'Laporan gangguan dengan foto dan status, jadwal perawatan aset, dan daftar prioritas rehabilitasi.',
      },
      {
        judul: 'Statistik dan capaian',
        isi: 'Total fitur, panjang pipa, dan jumlah sambungan rumah, dihitung gabungan atau per unit.',
      },
    ],
  },
]

/** Halaman depan arsgis. Menjelaskan alur kerja dan fitur, lalu menunjuk ke dashboard dan peta. */
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
        <section className="mx-auto grid max-w-6xl gap-10 px-4 py-10 md:grid-cols-[1.05fr_1fr] md:items-center md:gap-14 md:px-10 md:py-20">
          <div className="enter enter-2 flex min-w-0 flex-col gap-5">
            <p className="text-xs font-medium uppercase tracking-[0.14em] text-[#b8431f]">
              Sistem informasi SPAM perdesaan · Cianjur
            </p>
            <h1 className="text-3xl font-semibold leading-[1.15] tracking-tight md:text-5xl md:leading-[1.1]">
              Jaringan air minum Cianjur, dari peta sampai laporan gangguan.
            </h1>
            <p className="max-w-prose text-base leading-relaxed text-[#4b5563]">
              Tempat tim memantau pipa tiap unit SPAM, mencatat pelanggan per dusun, dan menindaklanjuti gangguan serta
              perawatan aset.
            </p>
            <div className="flex flex-wrap gap-3">
              <a
                href="/dashboard"
                className="rounded-lg bg-[#b8431f] px-5 py-2.5 text-sm font-medium text-white hover:bg-[#9c3a1b]"
              >
                Buka dashboard
              </a>
              <a
                href="/peta"
                className="rounded-lg border border-black/15 bg-white px-5 py-2.5 text-sm font-medium text-[#1f2937] hover:bg-black/5"
              >
                Lihat peta jaringan
              </a>
            </div>
            <p className="text-sm text-[#4b5563]">Peta jaringan dan data pelanggan memerlukan akun.</p>
          </div>

          <figure aria-hidden="true" className="enter enter-3 flex min-w-0 flex-col gap-4 rounded-2xl border border-black/10 bg-white p-4 md:p-6">
            <img
              src="/ilustrasi-jaringan.webp"
              alt=""
              width={1280}
              height={720}
              className="enter enter-4 h-auto w-full rounded-xl"
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

        <section aria-labelledby="alur" className="border-y border-black/10 bg-white">
          <div className="mx-auto max-w-6xl px-4 py-14 md:px-10 md:py-16">
            <h2 id="alur" className="text-sm font-semibold uppercase tracking-[0.12em] text-[#4b5563]">
              Alur kerja
            </h2>
            <ol className="mt-6 grid gap-8 md:grid-cols-3 md:gap-10">
              {ALUR.map((langkah, i) => (
                <li key={langkah.judul} className="flex min-w-0 flex-col gap-2">
                  <span className="text-sm font-semibold tabular-nums text-[#b8431f]">0{i + 1}</span>
                  <h3 className="text-lg font-semibold">{langkah.judul}</h3>
                  <p className="text-sm leading-relaxed text-[#4b5563]">{langkah.isi}</p>
                </li>
              ))}
            </ol>
          </div>
        </section>

        <section aria-labelledby="fitur" className="mx-auto max-w-6xl px-4 py-14 md:px-10 md:py-20">
          <h2 id="fitur" className="text-2xl font-semibold tracking-tight md:text-3xl">
            Yang ada di dalam aplikasi
          </h2>
          <div className="mt-8 grid gap-10">
            {GRUP.map((grup) => (
              <div key={grup.judul} className="grid gap-5 md:grid-cols-[13rem_1fr] md:gap-10">
                <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[#4b5563]">{grup.judul}</h3>
                <ul className="grid gap-px overflow-hidden rounded-2xl border border-black/10 bg-black/10 md:grid-cols-2">
                  {grup.item.map((t) => (
                    <li key={t.judul} className="flex min-w-0 flex-col gap-2 bg-white p-5 md:p-6">
                      <h4 className="text-base font-semibold">{t.judul}</h4>
                      <p className="text-sm leading-relaxed text-[#4b5563]">{t.isi}</p>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </section>
      </main>

      <footer className="border-t border-black/10 px-4 py-6 text-center text-xs text-[#4b5563] md:px-10">
        arsgis · Peta jaringan dan data pelanggan memerlukan akun.
      </footer>
    </div>
  )
}
