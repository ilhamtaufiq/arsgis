import '../preflight.css'
import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { canReadPelanggan, canSeeCustomerData, fetchMe, ROLE_LABEL, type User } from '../api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import {
  cakupanDesa,
  daftarDesa,
  daftarDusun,
  daftarPelanggan,
  daftarSr,
  GOLONGAN,
  pasangSrKeDusun,
  riwayatPelanggan,
  simpanDusun,
  simpanPelanggan,
  type BarisCakupan,
  type Dusun,
  type Pelanggan,
  type StatusPelanggan,
} from './pelangganApi'
import { fieldBerubah, kePayload, LABEL_FIELD, validasiFormPelanggan, type FormPelanggan } from './pelangganLogika'

const nf = new Intl.NumberFormat('id-ID')

function persen(x: number | null): string {
  return x === null ? '–' : `${x.toLocaleString('id-ID', { maximumFractionDigits: 1 })}%`
}

function labelGolongan(v: string): string {
  return GOLONGAN.find((g) => g.value === v)?.label ?? v
}

/**
 * Halaman pelanggan dan cakupan. Daftar pelanggan hanya untuk peran yang punya BacaDataPelanggan (operator, admin,
 * bendahara). Cakupan dusun bisa dibaca semua peran yang login. Mengubah data hanya untuk operator dan admin.
 */
export function PelangganPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const user: User | null = me.data ?? null
  const bolehBaca = canReadPelanggan(user)
  const bolehKelola = canSeeCustomerData(user)
  const desa = useQuery({ queryKey: ['pelanggan-desa'], queryFn: daftarDesa })
  const [pilihDesa, setPilihDesa] = useState<number | null>(null)
  const desaId = pilihDesa ?? desa.data?.[0]?.id ?? null

  if (me.isPending) return null

  return (
    <div className="min-h-screen bg-background p-4 text-foreground md:p-6">
      <div className="mx-auto flex max-w-6xl flex-col gap-4">
        <header className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <a href="/peta" className="text-sm underline">Kembali ke peta</a>
            <h1 className="text-xl font-semibold">Pelanggan dan cakupan layanan</h1>
            {user && <p className="text-sm text-muted-foreground">Masuk sebagai {user.username} ({ROLE_LABEL[user.role]})</p>}
          </div>
          <div className="flex items-center gap-2">
            <Label htmlFor="pilih-desa">Desa</Label>
            <select
              id="pilih-desa"
              className="rounded-md border bg-background px-2 py-1 text-sm"
              value={desaId ?? ''}
              onChange={(e) => setPilihDesa(Number(e.target.value))}
            >
              {(desa.data ?? []).map((d) => (
                <option key={d.id} value={d.id}>
                  {d.nama} ({d.kecamatan})
                </option>
              ))}
            </select>
          </div>
        </header>

        {desaId === null ? (
          <p className="text-sm text-muted-foreground">{desa.isPending ? 'Memuat desa…' : 'Belum ada desa.'}</p>
        ) : (
          <Tabs defaultValue={bolehBaca ? 'pelanggan' : 'cakupan'}>
            <TabsList>
              {bolehBaca && <TabsTrigger value="pelanggan">Pelanggan</TabsTrigger>}
              <TabsTrigger value="cakupan">Cakupan per dusun</TabsTrigger>
            </TabsList>
            {bolehBaca && (
              <TabsContent value="pelanggan">
                <DaftarPelanggan desaId={desaId} bolehKelola={bolehKelola} />
              </TabsContent>
            )}
            <TabsContent value="cakupan">
              <CakupanPanel desaId={desaId} bolehKelola={bolehKelola} />
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  )
}

/* ---------- Daftar pelanggan ---------- */

function DaftarPelanggan({ desaId, bolehKelola }: { desaId: number; bolehKelola: boolean }) {
  const [cari, setCari] = useState('')
  const [dusunId, setDusunId] = useState<number | null>(null)
  const [status, setStatus] = useState<StatusPelanggan | ''>('')
  const [editing, setEditing] = useState<Pelanggan | 'baru' | null>(null)
  const dusun = useQuery({ queryKey: ['dusun', desaId], queryFn: () => daftarDusun(desaId) })
  const daftar = useQuery({
    queryKey: ['pelanggan', desaId, cari, dusunId, status],
    queryFn: () => daftarPelanggan({ q: cari.trim(), desaId, dusunId, status }),
  })

  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-wrap items-end gap-2">
        <div className="flex flex-col gap-1">
          <Label htmlFor="cari-pelanggan">Cari nama atau nomor meter</Label>
          <Input id="cari-pelanggan" value={cari} onChange={(e) => setCari(e.target.value)} className="w-64" />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="filter-dusun">Dusun</Label>
          <select
            id="filter-dusun"
            className="rounded-md border bg-background px-2 py-2 text-sm"
            value={dusunId ?? ''}
            onChange={(e) => setDusunId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">Semua dusun</option>
            {(dusun.data ?? []).map((d) => (
              <option key={d.id} value={d.id}>
                {d.nama}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="filter-status">Status</Label>
          <select
            id="filter-status"
            className="rounded-md border bg-background px-2 py-2 text-sm"
            value={status}
            onChange={(e) => setStatus(e.target.value as StatusPelanggan | '')}
          >
            <option value="">Semua</option>
            <option value="aktif">Aktif</option>
            <option value="putus">Putus</option>
          </select>
        </div>
        {bolehKelola && (
          <Button type="button" onClick={() => setEditing('baru')}>
            Tambah pelanggan
          </Button>
        )}
      </div>

      {editing !== null && (
        <FormPelanggan
          key={editing === 'baru' ? 'baru' : editing.id}
          desaId={desaId}
          awal={editing === 'baru' ? null : editing}
          onSelesai={() => setEditing(null)}
        />
      )}

      {daftar.isError && (
        <p role="alert" className="text-sm text-red-700">
          {(daftar.error as Error).message}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Nama</TableHead>
            <TableHead>Nomor meter</TableHead>
            <TableHead>Golongan</TableHead>
            <TableHead>Titik SR</TableHead>
            <TableHead>Dusun</TableHead>
            <TableHead>Status</TableHead>
            {bolehKelola && <TableHead />}
          </TableRow>
        </TableHeader>
        <TableBody>
          {(daftar.data ?? []).map((p) => (
            <TableRow key={p.id}>
              <TableCell>{p.nama}</TableCell>
              <TableCell>{p.nomor_meter ?? '–'}</TableCell>
              <TableCell>{labelGolongan(p.golongan_tarif)}</TableCell>
              <TableCell className="font-mono text-xs">{p.kode_aset_sr}</TableCell>
              <TableCell>{p.dusun_nama ?? 'Belum di dusun'}</TableCell>
              <TableCell>{p.status === 'aktif' ? 'Aktif' : 'Putus'}</TableCell>
              {bolehKelola && (
                <TableCell>
                  <Button type="button" variant="outline" size="sm" onClick={() => setEditing(p)}>
                    Ubah
                  </Button>
                </TableCell>
              )}
            </TableRow>
          ))}
          {daftar.data?.length === 0 && (
            <TableRow>
              <TableCell colSpan={bolehKelola ? 7 : 6} className="text-center text-muted-foreground">
                Tidak ada pelanggan yang cocok.
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </section>
  )
}

function FormPelanggan({
  desaId,
  awal,
  onSelesai,
}: {
  desaId: number
  awal: Pelanggan | null
  onSelesai: () => void
}) {
  const qc = useQueryClient()
  const [form, setForm] = useState<FormPelanggan>({
    nama: awal?.nama ?? '',
    alamat: awal?.alamat ?? '',
    nomorMeter: awal?.nomor_meter ?? '',
    golongan: awal?.golongan_tarif ?? 'rumah_tangga',
    status: awal?.status ?? 'aktif',
    srId: awal?.sr_feature_id ?? null,
  })
  const [srCari, setSrCari] = useState('')
  const [galat, setGalat] = useState<string | null>(null)
  const sr = useQuery({ queryKey: ['sr-pilihan', desaId, srCari], queryFn: () => daftarSr({ desaId, q: srCari.trim() }) })
  const riwayat = useQuery({
    queryKey: ['riwayat-pelanggan', awal?.id],
    queryFn: () => riwayatPelanggan(awal!.id),
    enabled: awal !== null,
  })
  const simpan = useMutation({
    mutationFn: () => simpanPelanggan(awal?.id ?? null, kePayload(form)),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['pelanggan'] })
      qc.invalidateQueries({ queryKey: ['cakupan'] })
      onSelesai()
    },
    onError: (err) => setGalat((err as Error).message),
  })

  function kirim(e: FormEvent) {
    e.preventDefault()
    const salah = validasiFormPelanggan(form)
    if (salah) {
      setGalat(salah)
      return
    }
    setGalat(null)
    simpan.mutate()
  }

  return (
    <div className="grid gap-4 rounded-lg border p-4 md:grid-cols-2">
      <form onSubmit={kirim} className="flex flex-col gap-3" aria-label={awal ? 'Ubah pelanggan' : 'Tambah pelanggan'}>
        <h2 className="font-semibold">{awal ? `Ubah ${awal.nama}` : 'Pelanggan baru'}</h2>
        <div className="flex flex-col gap-1">
          <Label htmlFor="pl-nama">Nama</Label>
          <Input id="pl-nama" value={form.nama} onChange={(e) => setForm({ ...form, nama: e.target.value })} />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="pl-alamat">Alamat</Label>
          <Input id="pl-alamat" value={form.alamat} onChange={(e) => setForm({ ...form, alamat: e.target.value })} />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="pl-meter">Nomor meter (opsional)</Label>
          <Input id="pl-meter" value={form.nomorMeter} onChange={(e) => setForm({ ...form, nomorMeter: e.target.value })} />
        </div>
        <div className="flex gap-3">
          <div className="flex flex-col gap-1">
            <Label htmlFor="pl-golongan">Golongan tarif</Label>
            <select
              id="pl-golongan"
              className="rounded-md border bg-background px-2 py-2 text-sm"
              value={form.golongan}
              onChange={(e) => setForm({ ...form, golongan: e.target.value })}
            >
              {GOLONGAN.map((g) => (
                <option key={g.value} value={g.value}>
                  {g.label}
                </option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <Label htmlFor="pl-status">Status</Label>
            <select
              id="pl-status"
              className="rounded-md border bg-background px-2 py-2 text-sm"
              value={form.status}
              onChange={(e) => setForm({ ...form, status: e.target.value as StatusPelanggan })}
            >
              <option value="aktif">Aktif</option>
              <option value="putus">Putus</option>
            </select>
          </div>
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="pl-sr-cari">Cari titik SR (kode atau nama)</Label>
          <Input id="pl-sr-cari" value={srCari} onChange={(e) => setSrCari(e.target.value)} />
          <select
            id="pl-sr"
            aria-label="Titik SR"
            className="rounded-md border bg-background px-2 py-2 text-sm"
            value={form.srId ?? ''}
            onChange={(e) => setForm({ ...form, srId: e.target.value ? Number(e.target.value) : null })}
          >
            <option value="">Pilih titik SR…</option>
            {(sr.data ?? []).map((s) => (
              <option key={s.id} value={s.id}>
                {s.kode_aset} {s.dusun_nama ? `· ${s.dusun_nama}` : '· belum di dusun'}
              </option>
            ))}
          </select>
          {sr.data?.length === 200 && <p className="text-xs text-muted-foreground">Menampilkan 200 pertama. Persempit pencarian.</p>}
        </div>
        {galat && (
          <p role="alert" className="text-sm text-red-700">
            {galat}
          </p>
        )}
        <div className="flex gap-2">
          <Button type="submit" disabled={simpan.isPending}>
            {simpan.isPending ? 'Menyimpan…' : 'Simpan'}
          </Button>
          <Button type="button" variant="outline" onClick={onSelesai}>
            Batal
          </Button>
        </div>
      </form>

      <div className="flex flex-col gap-2">
        <h3 className="font-semibold">Riwayat perubahan</h3>
        {awal === null && <p className="text-sm text-muted-foreground">Riwayat muncul setelah pelanggan disimpan.</p>}
        {riwayat.isPending && awal !== null && <p className="text-sm text-muted-foreground">Memuat riwayat…</p>}
        <ol className="flex flex-col gap-2 text-sm">
          {(riwayat.data ?? []).map((r, i) => (
            <li key={i} className="rounded border p-2">
              <div className="font-medium">
                {r.aksi === 'tambah' ? 'Ditambahkan' : 'Diubah'} oleh {r.diubah_oleh}
              </div>
              <div className="text-xs text-muted-foreground">{r.diubah_pada.replace('T', ' ').slice(0, 16)}</div>
              {fieldBerubah(r).map((k) => (
                <div key={k} className="text-xs">
                  {LABEL_FIELD[k] ?? k}: {String(r.sebelum?.[k] ?? '–')} → {String(r.sesudah?.[k] ?? '–')}
                </div>
              ))}
            </li>
          ))}
        </ol>
      </div>
    </div>
  )
}

/* ---------- Cakupan ---------- */

function CakupanPanel({ desaId, bolehKelola }: { desaId: number; bolehKelola: boolean }) {
  const cakupan = useQuery({ queryKey: ['cakupan', desaId], queryFn: () => cakupanDesa(desaId) })
  const dusun = useQuery({ queryKey: ['dusun', desaId], queryFn: () => daftarDusun(desaId) })

  return (
    <section className="flex flex-col gap-4">
      <p className="text-sm text-muted-foreground">
        Persen cakupan = SR terlayani dibagi jumlah rumah dusun. SR terlayani adalah titik SR yang punya pelanggan aktif.
        Jiwa terlayani adalah perkiraan proporsional, bukan hitungan langsung. Di peta, titik SR hijau berarti terlayani
        dan kuning berarti belum.
      </p>
      {cakupan.isError && (
        <p role="alert" className="text-sm text-red-700">
          {(cakupan.error as Error).message}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Dusun</TableHead>
            <TableHead className="text-right">Rumah</TableHead>
            <TableHead className="text-right">Jiwa</TableHead>
            <TableHead className="text-right">SR</TableHead>
            <TableHead className="text-right">Terlayani</TableHead>
            <TableHead className="text-right">Belum</TableHead>
            <TableHead className="text-right">Cakupan</TableHead>
            <TableHead className="text-right">SR terlayani</TableHead>
            <TableHead className="text-right">Jiwa terlayani (perkiraan)</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {[...(cakupan.data?.per_dusun ?? []), ...(cakupan.data ? [cakupan.data.tanpa_dusun, cakupan.data.total] : [])].map(
            (b: BarisCakupan, i) => (
              <TableRow key={`${b.nama}-${i}`} className={b.nama === 'Total desa' ? 'font-semibold' : undefined}>
                <TableCell>{b.nama}</TableCell>
                <TableCell className="text-right tabular-nums">{nf.format(b.jumlah_rumah)}</TableCell>
                <TableCell className="text-right tabular-nums">{nf.format(b.jumlah_jiwa)}</TableCell>
                <TableCell className="text-right tabular-nums">{nf.format(b.cakupan.jumlah_sr)}</TableCell>
                <TableCell className="text-right tabular-nums">{nf.format(b.cakupan.sr_terlayani)}</TableCell>
                <TableCell className="text-right tabular-nums">{nf.format(b.cakupan.sr_belum)}</TableCell>
                <TableCell className="text-right tabular-nums">{persen(b.cakupan.persen_cakupan)}</TableCell>
                <TableCell className="text-right tabular-nums">{persen(b.cakupan.persen_sr_terlayani)}</TableCell>
                <TableCell className="text-right tabular-nums">
                  {b.cakupan.jiwa_terlayani_perkiraan === null ? '–' : nf.format(b.cakupan.jiwa_terlayani_perkiraan)}
                </TableCell>
              </TableRow>
            ),
          )}
          {cakupan.data && cakupan.data.per_dusun.length === 0 && (
            <TableRow>
              <TableCell colSpan={9} className="text-center text-muted-foreground">
                Belum ada dusun di desa ini.
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>

      {bolehKelola && <DusunEditor desaId={desaId} daftar={dusun.data ?? []} />}
      {bolehKelola && <PasangSr desaId={desaId} daftarDusun={dusun.data ?? []} />}
    </section>
  )
}

function DusunEditor({ desaId, daftar }: { desaId: number; daftar: Dusun[] }) {
  const qc = useQueryClient()
  const [pilih, setPilih] = useState<number | 'baru'>('baru')
  const terpilih = daftar.find((d) => d.id === pilih) ?? null
  const [nama, setNama] = useState('')
  const [rumah, setRumah] = useState('0')
  const [jiwa, setJiwa] = useState('0')
  const [galat, setGalat] = useState<string | null>(null)
  const simpan = useMutation({
    mutationFn: () =>
      simpanDusun(desaId, terpilih ? terpilih.id : null, {
        nama,
        jumlah_rumah: Number(rumah),
        jumlah_jiwa: Number(jiwa),
      }),
    onSuccess: () => {
      setGalat(null)
      qc.invalidateQueries({ queryKey: ['dusun', desaId] })
      qc.invalidateQueries({ queryKey: ['cakupan', desaId] })
      setPilih('baru')
      setNama('')
      setRumah('0')
      setJiwa('0')
    },
    onError: (err) => setGalat((err as Error).message),
  })

  function pilihDusun(v: string) {
    const id = v === 'baru' ? 'baru' : Number(v)
    setPilih(id)
    const d = daftar.find((x) => x.id === id)
    setNama(d?.nama ?? '')
    setRumah(String(d?.jumlah_rumah ?? 0))
    setJiwa(String(d?.jumlah_jiwa ?? 0))
  }

  function kirim(e: FormEvent) {
    e.preventDefault()
    if (nama.trim() === '') {
      setGalat('Nama dusun wajib diisi.')
      return
    }
    if (!Number.isInteger(Number(rumah)) || Number(rumah) < 0 || !Number.isInteger(Number(jiwa)) || Number(jiwa) < 0) {
      setGalat('Jumlah rumah dan jiwa harus bilangan bulat nol atau lebih.')
      return
    }
    setGalat(null)
    simpan.mutate()
  }

  return (
    <form onSubmit={kirim} className="flex flex-wrap items-end gap-3 rounded-lg border p-4" aria-label="Data dusun">
      <div className="flex flex-col gap-1">
        <Label htmlFor="dusun-pilih">Dusun</Label>
        <select
          id="dusun-pilih"
          className="rounded-md border bg-background px-2 py-2 text-sm"
          value={pilih === 'baru' ? 'baru' : String(pilih)}
          onChange={(e) => pilihDusun(e.target.value)}
        >
          <option value="baru">Dusun baru</option>
          {daftar.map((d) => (
            <option key={d.id} value={d.id}>
              {d.nama}
            </option>
          ))}
        </select>
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="dusun-nama">Nama dusun</Label>
        <Input id="dusun-nama" value={nama} onChange={(e) => setNama(e.target.value)} className="w-48" />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="dusun-rumah">Jumlah rumah</Label>
        <Input id="dusun-rumah" type="number" min={0} value={rumah} onChange={(e) => setRumah(e.target.value)} className="w-32" />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="dusun-jiwa">Jumlah jiwa</Label>
        <Input id="dusun-jiwa" type="number" min={0} value={jiwa} onChange={(e) => setJiwa(e.target.value)} className="w-32" />
      </div>
      <Button type="submit" disabled={simpan.isPending}>
        {terpilih ? 'Simpan perubahan' : 'Tambah dusun'}
      </Button>
      {galat && (
        <p role="alert" className="w-full text-sm text-red-700">
          {galat}
        </p>
      )}
    </form>
  )
}

function PasangSr({ desaId, daftarDusun }: { desaId: number; daftarDusun: Dusun[] }) {
  const qc = useQueryClient()
  const [cari, setCari] = useState('')
  const [galat, setGalat] = useState<string | null>(null)
  const sr = useQuery({ queryKey: ['sr-pilihan', desaId, cari], queryFn: () => daftarSr({ desaId, q: cari.trim() }) })
  const pasang = useMutation({
    mutationFn: (args: { featureId: number; dusunId: number | null }) => pasangSrKeDusun(args.featureId, args.dusunId),
    onSuccess: () => {
      setGalat(null)
      qc.invalidateQueries({ queryKey: ['cakupan', desaId] })
      qc.invalidateQueries({ queryKey: ['sr-pilihan', desaId] })
    },
    onError: (err) => setGalat((err as Error).message),
  })

  return (
    <section className="flex flex-col gap-2">
      <h2 className="font-semibold">Pasang titik SR ke dusun</h2>
      <Input placeholder="Cari kode atau nama SR" value={cari} onChange={(e) => setCari(e.target.value)} className="w-64" />
      {galat && (
        <p role="alert" className="text-sm text-red-700">
          {galat}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Kode SR</TableHead>
            <TableHead>Dusun</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {(sr.data ?? []).map((s) => (
            <TableRow key={s.id}>
              <TableCell className="font-mono text-xs">{s.kode_aset}</TableCell>
              <TableCell>
                <select
                  aria-label={`Dusun untuk ${s.kode_aset}`}
                  className="rounded-md border bg-background px-2 py-1 text-sm"
                  value={s.dusun_id ?? ''}
                  onChange={(e) =>
                    pasang.mutate({ featureId: s.id, dusunId: e.target.value ? Number(e.target.value) : null })
                  }
                >
                  <option value="">Belum di dusun</option>
                  {daftarDusun.map((d) => (
                    <option key={d.id} value={d.id}>
                      {d.nama}
                    </option>
                  ))}
                </select>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </section>
  )
}
