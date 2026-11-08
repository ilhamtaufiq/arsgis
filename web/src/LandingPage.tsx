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

const TOPICS = [
  {
    judul: 'Peta per unit SPAM',
    isi: 'Pipa, valve, reservoir, dan sambungan rumah tampil per unit, dengan batas desa BPS.',
  },
  {
    judul: 'Statistik dan capaian',
    isi: 'Total fitur, panjang pipa, dan jumlah sambungan rumah, dihitung gabungan atau per unit.',
  },
  {
    judul: 'Kelengkapan data',
    isi: 'Setiap desa ditandai bila data batas, pokmas, atau sambungan rumahnya belum lengkap.',
  },
]

/** Halaman depan arsgis. Menjelaskan fungsi aplikasi dan menunjuk ke dashboard dan peta. */
export function LandingPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })

  return (
    <div className="min-h-svh bg-[#f5f5f4] text-[#1f2937]">
      <header className="enter flex items-center justify-between gap-4 border-b border-black/10 bg-white/80 px-5 py-3 backdrop-blur md:px-10">
        <a href="/" className="text-base font-semibold tracking-tight">arsgis</a>
        <nav className="flex items-center gap-5 text-sm">
          <a href="/dashboard" className="hover:underline underline-offset-4">Dashboard</a>
          <a href="/peta" className="hover:underline underline-offset-4">Peta</a>
          <AuthPanel user={me.data ?? null} inline />
        </nav>
      </header>

      <main className="mx-auto grid max-w-6xl gap-12 px-5 py-12 md:grid-cols-[1.1fr_1fr] md:items-center md:px-10 md:py-20">
        <section className="enter enter-2 flex flex-col gap-6">
          <p className="text-xs font-medium uppercase tracking-[0.14em] text-[#DF5E3A]">WebGIS perpipaan SPAM · Cianjur</p>
          <h1 className="text-4xl font-semibold leading-[1.1] tracking-tight md:text-5xl">
            Jaringan air minum, dari peta sampai capaian desa.
          </h1>
          <p className="max-w-prose text-base leading-relaxed text-[#4b5563]">
            Satu tempat untuk memantau pipa, valve, dan reservoir tiap unit SPAM, serta sambungan rumah dan capaian
            Sistem Penyediaan Air Minum di tiap desa.
          </p>
          <div className="flex flex-wrap gap-3">
            <a
              href="/dashboard"
              className="rounded-full bg-[#DF5E3A] px-5 py-2.5 text-sm font-medium text-white hover:bg-[#c94f2d]"
            >
              Buka dashboard
            </a>
            <a
              href="/peta"
              className="rounded-full border border-black/15 bg-white px-5 py-2.5 text-sm font-medium hover:bg-black/5"
            >
              Peta jaringan
            </a>
          </div>
        </section>

        <figure aria-hidden="true" className="enter enter-3 flex flex-col gap-4 rounded-3xl border border-black/10 bg-white p-6">
          <img
            src="/ilustrasi-jaringan.webp"
            alt=""
            width={1280}
            height={720}
            className="enter enter-4 h-auto w-full rounded-2xl"
          />
          <ul className="flex flex-wrap gap-x-5 gap-y-2 text-sm text-[#4b5563]">
            {LEGEND.map((l) => (
              <li key={l.label} className="flex items-center gap-2">
                <span className="inline-block h-2.5 w-2.5 rounded-full" style={{ background: l.color }} />
                {l.label}
              </li>
            ))}
          </ul>
        </figure>
      </main>

      <section className="mx-auto max-w-6xl px-5 pb-20 md:px-10">
        <ul className="enter enter-4 grid gap-px overflow-hidden rounded-2xl border border-black/10 bg-black/10 md:grid-cols-3">
          {TOPICS.map((t) => (
            <li key={t.judul} className="flex flex-col gap-2 bg-white p-6">
              <h2 className="text-base font-semibold">{t.judul}</h2>
              <p className="text-sm leading-relaxed text-[#4b5563]">{t.isi}</p>
            </li>
          ))}
        </ul>
      </section>

      <footer className="border-t border-black/10 px-5 py-6 text-center text-xs text-[#6b7280] md:px-10">
        arsgis · Peta jaringan memerlukan akun.
      </footer>
    </div>
  )
}
