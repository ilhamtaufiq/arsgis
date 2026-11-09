import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { BATAS_TAMPIL_DEFAULT, filterOpsi, type OpsiCombo } from './comboboxLogic'
import './combobox.css'

type Props = {
  id: string
  label: string
  options: OpsiCombo[]
  /** Nilai terpilih; string kosong berarti belum memilih. */
  value: string
  onChange: (value: string) => void
  placeholder?: string
  /** Teks untuk pilihan kosong, misalnya "Tanpa paket". Kosong berarti tidak ada pilihan kosong. */
  kosongLabel?: string
  disabled?: boolean
  /** Pesan saat daftar belum bisa dimuat atau kosong sama sekali. */
  emptyText?: string
  limit?: number
}

/** Dropdown yang bisa dicari. Default menampilkan 10 hasil pertama; ketik untuk mempersempit. */
export function Combobox({
  id,
  label,
  options,
  value,
  onChange,
  placeholder = 'Ketik untuk mencari…',
  kosongLabel,
  disabled = false,
  emptyText = 'Tidak ada pilihan.',
  limit = BATAS_TAMPIL_DEFAULT,
}: Props) {
  const listId = useId()
  const wrap = useRef<HTMLDivElement>(null)
  const terpilih = options.find((o) => o.value === value)
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [aktif, setAktif] = useState(0)

  const { items, total } = useMemo(() => filterOpsi(options, query, limit), [options, query, limit])
  const daftar: OpsiCombo[] = kosongLabel !== undefined && query.trim() === ''
    ? [{ value: '', label: kosongLabel }, ...items]
    : items

  useEffect(() => {
    if (!open) return
    const tutup = (e: MouseEvent) => {
      if (!wrap.current?.contains(e.target as Node)) {
        setOpen(false)
        setQuery('')
      }
    }
    document.addEventListener('mousedown', tutup)
    return () => document.removeEventListener('mousedown', tutup)
  }, [open])

  const pilih = (v: string) => {
    onChange(v)
    setOpen(false)
    setQuery('')
  }

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setOpen(true)
      setAktif((i) => Math.min(i + 1, Math.max(daftar.length - 1, 0)))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setAktif((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter' && open && daftar[aktif]) {
      e.preventDefault()
      pilih(daftar[aktif].value)
    } else if (e.key === 'Escape') {
      setOpen(false)
      setQuery('')
    }
  }

  const teks = open ? query : (terpilih?.label ?? (kosongLabel && value === '' ? kosongLabel : ''))
  const opsiId = (i: number) => `${id}-opsi-${i}`

  return (
    <div className="combo" ref={wrap}>
      <label htmlFor={id}>{label}</label>
      <div className="combo-box">
        <input
          id={id}
          role="combobox"
          aria-expanded={open}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={open && daftar[aktif] ? opsiId(aktif) : undefined}
          autoComplete="off"
          disabled={disabled}
          placeholder={placeholder}
          value={teks}
          onFocus={() => { setOpen(true); setAktif(0) }}
          onChange={(e) => { setQuery(e.target.value); setOpen(true); setAktif(0) }}
          onKeyDown={onKey}
        />
        {value !== '' && !disabled && (
          <button type="button" className="combo-hapus" aria-label={`Hapus pilihan ${label}`} onClick={() => pilih('')}>
            ×
          </button>
        )}
      </div>
      {open && (
        <ul id={listId} role="listbox" className="combo-list" aria-label={label}>
          {daftar.length === 0 && <li className="combo-kosong">{emptyText}</li>}
          {daftar.map((o, i) => (
            <li
              key={`${o.value}-${i}`}
              id={opsiId(i)}
              role="option"
              aria-selected={o.value === value}
              className={i === aktif ? 'is-aktif' : undefined}
              onMouseDown={(e) => e.preventDefault()}
              onMouseEnter={() => setAktif(i)}
              onClick={() => pilih(o.value)}
            >
              <span>{o.label}</span>
              {o.hint && <small>{o.hint}</small>}
            </li>
          ))}
          {total > items.length && (
            <li className="combo-info" aria-live="polite">
              Menampilkan {items.length} dari {total}. Ketik lebih spesifik untuk mempersempit.
            </li>
          )}
        </ul>
      )}
    </div>
  )
}
