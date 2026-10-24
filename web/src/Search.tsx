import { useState, type KeyboardEvent } from 'react'
import { useMapStore } from './store'
import type { PipeFeature } from './types'

const LABEL: Record<string, string> = {
  pipa: 'Pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
}

type Props = {
  features: PipeFeature[]
}

/** Dynamic island: pencarian nama fitur di data yang dimuat. Melebar saat fokus atau ada hasil. */
export function SearchBox({ features }: Props) {
  const [query, setQuery] = useState('')
  const [focused, setFocused] = useState(false)
  const select = useMapStore((s) => s.select)
  const setFocus = useMapStore((s) => s.setFocus)

  const term = query.trim().toLowerCase()
  const matches = term
    ? features.filter((f) => (f.properties.nama ?? '').toLowerCase().includes(term)).slice(0, 8)
    : []
  const expanded = focused || term !== ''

  function pick(feature: PipeFeature) {
    setFocus(feature)
    select(feature)
    setQuery('')
  }

  function handleKey(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter' && matches[0]) pick(matches[0])
    if (e.key === 'Escape') {
      setQuery('')
      e.currentTarget.blur()
    }
  }

  return (
    <div className={`island${expanded ? ' is-expanded' : ''}`}>
      <div className="island-bar">
        <svg viewBox="0 0 24 24" aria-hidden="true" width="18" height="18">
          <circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" strokeWidth="2" />
          <path d="M20 20l-4-4" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <input
          aria-label="Cari nama fitur"
          placeholder="Cari nama fitur…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          onKeyDown={handleKey}
        />
      </div>
      {matches.length > 0 && (
        <ul className="island-results">
          {matches.map((f) => (
            <li key={f.id}>
              <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => pick(f)}>
                <span>{f.properties.nama}</span>
                <small>{LABEL[f.properties.feature_type] ?? f.properties.feature_type}</small>
              </button>
            </li>
          ))}
        </ul>
      )}
      {term !== '' && matches.length === 0 && <p className="island-empty">Tidak ada fitur yang cocok di area ini.</p>}
    </div>
  )
}
