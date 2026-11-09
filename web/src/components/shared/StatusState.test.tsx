import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { StatusState } from './StatusState'

describe('StatusState', () => {
  it('keadaan kosong menampilkan judul tanpa role khusus', () => {
    const html = renderToStaticMarkup(<StatusState tipe="kosong" judul="Belum ada pelanggan" />)
    expect(html).toContain('status-state-kosong')
    expect(html).toContain('Belum ada pelanggan')
    expect(html).not.toContain('role=')
  })

  it('keadaan memuat memakai teks dan role status', () => {
    const html = renderToStaticMarkup(
      <StatusState tipe="memuat" judul="Memuat data desa" deskripsi="Mohon tunggu sebentar." />,
    )
    expect(html).toContain('status-state-memuat')
    expect(html).toContain('role="status"')
    expect(html).toContain('Memuat data desa')
    expect(html).toContain('Mohon tunggu sebentar.')
  })

  it('keadaan gagal memakai role alert dan menampilkan aksi', () => {
    const html = renderToStaticMarkup(
      <StatusState
        tipe="gagal"
        judul="Gagal memuat tabel"
        deskripsi="Server tidak merespons."
        aksi={<button type="button">Coba lagi</button>}
      />,
    )
    expect(html).toContain('status-state-gagal')
    expect(html).toContain('role="alert"')
    expect(html).toContain('Server tidak merespons.')
    expect(html).toContain('Coba lagi')
  })

  it('deskripsi dan aksi bersifat opsional', () => {
    const html = renderToStaticMarkup(<StatusState tipe="gagal" judul="Gagal" />)
    expect(html).not.toContain('status-state-deskripsi')
    expect(html).not.toContain('status-state-aksi')
  })
})
