// Logika murni untuk halaman pelanggan: validasi isian di sisi klien (mencerminkan aturan di pipa-core)
// dan selisih riwayat. Dipisah dari komponen supaya bisa diuji tanpa DOM.

import type { PelangganInput, RiwayatItem, StatusPelanggan } from './pelangganApi'
import { GOLONGAN } from './pelangganApi'

export type FormPelanggan = {
  nama: string
  alamat: string
  nomorMeter: string
  golongan: string
  status: StatusPelanggan
  srId: number | null
}

/** Pesan galat pertama, atau null bila isian boleh dikirim. Validasi final tetap di server. */
export function validasiFormPelanggan(f: FormPelanggan): string | null {
  const nama = f.nama.trim()
  if (nama.length < 2) return 'Nama pelanggan minimal 2 karakter.'
  if (nama.length > 120) return 'Nama pelanggan maksimal 120 karakter.'
  if (f.alamat.trim().length > 255) return 'Alamat maksimal 255 karakter.'
  const meter = f.nomorMeter.trim()
  if (meter.length > 32) return 'Nomor meter maksimal 32 karakter.'
  if (meter && !/^[A-Za-z0-9\-/.]+$/.test(meter)) return "Nomor meter hanya boleh huruf, angka, '-', '/', atau '.'."
  if (!GOLONGAN.some((g) => g.value === f.golongan)) return 'Pilih golongan tarif.'
  if (f.srId === null) return 'Pilih titik SR yang dilayani.'
  return null
}

export function kePayload(f: FormPelanggan): PelangganInput {
  return {
    nama: f.nama.trim(),
    alamat: f.alamat.trim() || null,
    nomor_meter: f.nomorMeter.trim() || null,
    golongan_tarif: f.golongan,
    status: f.status,
    sr_feature_id: f.srId as number,
  }
}

/** Daftar nama field yang berbeda antara dua keadaan riwayat. Dipakai untuk menampilkan perubahan. */
export function fieldBerubah(item: RiwayatItem): string[] {
  if (item.aksi === 'tambah' || !item.sebelum || !item.sesudah) return []
  const kunci = new Set([...Object.keys(item.sebelum), ...Object.keys(item.sesudah)])
  return [...kunci].filter((k) => JSON.stringify(item.sebelum?.[k]) !== JSON.stringify(item.sesudah?.[k]))
}

export const LABEL_FIELD: Record<string, string> = {
  nama: 'Nama',
  alamat: 'Alamat',
  nomor_meter: 'Nomor meter',
  golongan_tarif: 'Golongan tarif',
  status: 'Status',
  sr_feature_id: 'Titik SR',
}
