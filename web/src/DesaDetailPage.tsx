import './preflight.css'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState, type ChangeEvent } from 'react'
import { fetchMe, type Role } from './api'
import { AuthPanel } from './Auth'
import {
  fetchDesaDetail,
  fetchDesaPokmas,
  hapusDesaPokmas,
  simpanDesaPokmas,
  type IsianPokmas,
  type PokmasEfektif,
  type SumberPokmas,
} from './desaDetailApi'
import { ConfirmDialog } from '@/components/shared/ConfirmDialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const nf = (n: number) => n.toLocaleString('id-ID')

export function DesaDetailPage({ id }: { id: number }) {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  // Peran ikut di kunci query agar data ulang dimuat setelah login: nama pengurus hanya dikirim untuk editor.
  const role: Role | null = me.data?.role ?? null
  const detail = useQuery({
    queryKey: ['desa-detail', id, role],
    queryFn: () => fetchDesaDetail(id),
    enabled: Number.isFinite(id) && id > 0,
  })

  const d = detail.data
  return (
    <div className="min-h-screen bg-muted/30 p-4 md:p-6">
      <header className="mb-4 flex items-center justify-between gap-3">
        <a href="/peta" className="text-sm text-muted-foreground hover:underline">← Kembali ke peta</a>
        <AuthPanel user={me.data ?? null} />
      </header>

      {detail.isLoading && <p className="text-sm text-muted-foreground">Memuat detail desa…</p>}
      {detail.isError && (
        <Card><CardContent className="p-6 text-sm text-destructive">{(detail.error as Error).message}</CardContent></Card>
      )}

      {d && (
        <div className="grid gap-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-2xl">{d.summary.nama}</CardTitle>
              <CardDescription>
                Kecamatan {d.summary.kecamatan}
                {d.summary.kode_bps ? ` · kode BPS ${d.summary.kode_bps}` : ''}
              </CardDescription>
            </CardHeader>
            <CardContent className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <Stat label="Cakupan SPM" value={d.summary.cakupan_persen === null ? 'Tanpa target' : `${d.summary.cakupan_persen}%`} />
              <Stat label="Target" value={`${nf(d.summary.target_kk)} KK`} />
              <Stat label="JP (perpipaan)" value={`${nf(d.summary.jp_kk)} KK`} />
              <Stat label="BJP" value={`${nf(d.summary.bjp_kk)} KK`} />
              <Stat label="Sambungan rumah" value={nf(d.summary.sr)} />
              <Stat label="Jiwa terlayani" value={nf(d.summary.jiwa)} />
              <Stat label="Unit SPAM" value={nf(d.summary.unit_count)} />
              <Stat label="Batas desa" value={d.summary.punya_batas ? 'Ada' : 'Belum ada'} />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Unit SPAM</CardTitle>
              <CardDescription>{d.units.length} unit aktif di desa ini</CardDescription>
            </CardHeader>
            <CardContent>
              {d.units.length === 0 ? (
                <p className="text-sm text-muted-foreground">Belum ada unit SPAM di desa ini.</p>
              ) : (
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Nama unit</TableHead>
                        <TableHead>Sistem layanan</TableHead>
                        <TableHead>Sumber dana</TableHead>
                        <TableHead className="text-right">SR</TableHead>
                        <TableHead className="text-right">KK</TableHead>
                        <TableHead className="text-right">Jiwa</TableHead>
                        <TableHead className="text-right">BJP KK</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {d.units.map((u) => (
                        <TableRow key={u.id}>
                          <TableCell className="font-medium">
                            {u.nama ?? `Unit #${u.id}`}
                            {u.is_simspam && <Badge variant="secondary" className="ml-2">SIMSPAM</Badge>}
                          </TableCell>
                          <TableCell>{u.sistem_layanan ?? '–'}</TableCell>
                          <TableCell>{u.sumber_dana ?? '–'}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.sr)}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.kk)}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.jiwa)}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.bjp_kk)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>

          <PokmasCard id={id} role={role} />
        </div>
      )}
    </div>
  )
}

function PokmasCard({ id, role }: { id: number; role: Role | null }) {
  const queryClient = useQueryClient()
  const bisaEdit = role === 'editor' || role === 'admin'
  const [mengedit, setMengedit] = useState(false)
  const [pesan, setPesan] = useState<string | null>(null)
  const [konfirmasiKembali, setKonfirmasiKembali] = useState(false)
  const pokmas = useQuery({
    queryKey: ['desa-pokmas', id, role],
    queryFn: () => fetchDesaPokmas(id),
    enabled: Number.isFinite(id) && id > 0,
  })

  const selesai = () => {
    setPesan(null)
    setMengedit(false)
    void queryClient.invalidateQueries({ queryKey: ['desa-pokmas', id] })
    void queryClient.invalidateQueries({ queryKey: ['desa-detail', id] })
  }
  const simpan = useMutation({
    mutationFn: (isian: IsianPokmas) => simpanDesaPokmas(id, isian),
    onSuccess: selesai,
    onError: (e) => setPesan((e as Error).message),
  })
  const kembalikan = useMutation({
    mutationFn: () => hapusDesaPokmas(id),
    onSuccess: selesai,
    onError: (e) => setPesan((e as Error).message),
  })
  const sedangKirim = simpan.isPending || kembalikan.isPending
  const data = pokmas.data

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <CardTitle className="text-base">Pokmas</CardTitle>
            <CardDescription>Kelompok masyarakat pengelola SPAM di desa ini</CardDescription>
          </div>
          {data && <SumberBadge sumber={data.sumber} />}
        </div>
      </CardHeader>
      <CardContent className="grid gap-4">
        {pokmas.isLoading && <p className="text-sm text-muted-foreground">Memuat data Pokmas…</p>}
        {pokmas.isError && <p className="text-sm text-destructive">{(pokmas.error as Error).message}</p>}

        {data && mengedit && (
          <FormPokmas data={data} sedangKirim={sedangKirim} onSimpan={(isian) => simpan.mutate(isian)} onBatal={() => { setPesan(null); setMengedit(false) }} />
        )}

        {data && !mengedit && (
          <>
            {!data.pokmas ? (
              <p className="text-sm text-muted-foreground">Profil Pokmas belum terisi.</p>
            ) : (
              <div className="grid gap-3 md:grid-cols-2">
                <Field label="Nama Pokmas" value={data.pokmas.nama} />
                <Field label="Perdes" value={data.pokmas.perdes} />
                {data.pokmas.konflik && (
                  <div className="md:col-span-2">
                    <Badge variant="destructive">Ada perbedaan nama Pokmas antar unit</Badge>
                  </div>
                )}
                {data.pokmas.kepala !== undefined ? (
                  <>
                    <Field label="Kepala" value={data.pokmas.kepala ?? null} />
                    <Field label="Bendahara" value={data.pokmas.bendahara ?? null} />
                    <Field label="Sekretaris" value={data.pokmas.sekretaris ?? null} />
                  </>
                ) : (
                  <p className="text-xs text-muted-foreground md:col-span-2">
                    Nama pengurus hanya terlihat untuk editor dan admin.
                  </p>
                )}
              </div>
            )}

            <p className="text-xs text-muted-foreground">
              {data.sumber === 'override'
                ? `Dikoreksi di arsgis${data.koreksi ? ` oleh ${data.koreksi.diperbarui_oleh} pada ${data.koreksi.diperbarui_pada}` : ''}. Data hasil sinkron Arumanis tidak ikut berubah.`
                : 'Data hasil sinkron dari Arumanis.'}
              {data.koreksi?.catatan ? ` Catatan: ${data.koreksi.catatan}` : ''}
            </p>

            {bisaEdit && (
              <div className="flex flex-wrap gap-2">
                <Button size="sm" disabled={sedangKirim} onClick={() => { setPesan(null); setMengedit(true) }}>
                  {data.pokmas ? 'Ubah Pokmas' : 'Isi Pokmas'}
                </Button>
                {data.sumber === 'override' && (
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={sedangKirim}
                    onClick={() => setKonfirmasiKembali(true)}
                  >
                    Kembalikan ke data Arumanis
                  </Button>
                )}
              </div>
            )}
          </>
        )}

        <ConfirmDialog
          open={konfirmasiKembali}
          judul="Kembalikan ke data Arumanis?"
          pesan="Koreksi Pokmas yang tersimpan di arsgis akan dihapus. Data hasil sinkron Arumanis dipakai lagi."
          labelAksi="Ya, kembalikan"
          nada="bahaya"
          sibuk={kembalikan.isPending}
          onBatal={() => setKonfirmasiKembali(false)}
          onKonfirmasi={() => {
            setKonfirmasiKembali(false)
            kembalikan.mutate()
          }}
        />

        {pesan && <p className="text-sm text-destructive">{pesan}</p>}
      </CardContent>
    </Card>
  )
}

function SumberBadge({ sumber }: { sumber: SumberPokmas }) {
  return sumber === 'override' ? (
    <Badge>Koreksi arsgis</Badge>
  ) : (
    <Badge variant="secondary">Data Arumanis</Badge>
  )
}

function FormPokmas({
  data,
  sedangKirim,
  onSimpan,
  onBatal,
}: {
  data: PokmasEfektif
  sedangKirim: boolean
  onSimpan: (isian: IsianPokmas) => void
  onBatal: () => void
}) {
  const [isian, setIsian] = useState<IsianPokmas>(() => ({
    nama: data.pokmas?.nama ?? '',
    perdes: data.pokmas?.perdes ?? '',
    kepala: data.pokmas?.kepala ?? '',
    bendahara: data.pokmas?.bendahara ?? '',
    sekretaris: data.pokmas?.sekretaris ?? '',
    catatan: data.koreksi?.catatan ?? '',
  }))
  const ubah = (kunci: keyof IsianPokmas) => (e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
    setIsian((s) => ({ ...s, [kunci]: e.target.value }))

  return (
    <form
      className="grid gap-3 md:grid-cols-2"
      onSubmit={(e) => {
        e.preventDefault()
        onSimpan(isian)
      }}
    >
      <CampoIsian id="pokmas-nama" label="Nama Pokmas" wajib value={isian.nama} onChange={ubah('nama')} maks={200} />
      <CampoIsian id="pokmas-perdes" label="Perdes" value={isian.perdes} onChange={ubah('perdes')} maks={200} />
      <CampoIsian id="pokmas-kepala" label="Kepala" value={isian.kepala} onChange={ubah('kepala')} maks={120} />
      <CampoIsian id="pokmas-bendahara" label="Bendahara" value={isian.bendahara} onChange={ubah('bendahara')} maks={120} />
      <CampoIsian id="pokmas-sekretaris" label="Sekretaris" value={isian.sekretaris} onChange={ubah('sekretaris')} maks={120} />
      <div className="grid gap-1.5 md:col-span-2">
        <Label htmlFor="pokmas-catatan">Catatan</Label>
        <textarea
          id="pokmas-catatan"
          rows={3}
          maxLength={1000}
          value={isian.catatan}
          onChange={ubah('catatan')}
          className="w-full min-w-0 rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 dark:bg-input/30"
        />
      </div>
      <div className="flex flex-wrap gap-2 md:col-span-2">
        <Button type="submit" size="sm" disabled={sedangKirim || isian.nama.trim() === ''}>
          {sedangKirim ? 'Menyimpan…' : 'Simpan'}
        </Button>
        <Button type="button" size="sm" variant="outline" disabled={sedangKirim} onClick={onBatal}>
          Batal
        </Button>
      </div>
    </form>
  )
}

function CampoIsian({
  id,
  label,
  value,
  onChange,
  maks,
  wajib = false,
}: {
  id: string
  label: string
  value: string
  onChange: (e: ChangeEvent<HTMLInputElement>) => void
  maks: number
  wajib?: boolean
}) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>
        {label}
        {wajib ? ' *' : ''}
      </Label>
      <Input id={id} value={value} onChange={onChange} maxLength={maks} required={wajib} />
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border bg-background p-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-lg font-semibold tabular-nums">{value}</p>
    </div>
  )
}

function Field({ label, value }: { label: string; value: string | null }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-sm">{value ?? '–'}</p>
    </div>
  )
}
