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

const COLOR: Record<string, string> = {
  pipa: '#1d4ed8',
  valve: '#111111',
  reservoir: '#0f766e',
  sr: '#a16207',
  bronkaptering: '#0891b2',
}

type Props = {
  features: PipeFeature[]
}

/** Dynamic island: kompak saat idle, melebar saat fokus, dan menampilkan hasil pencarian nama fitur. */
export function SearchBox({ features }: Props) {
  const [query, setQuery] = useState('')
  const [focused, setFocused] = useState(false)
  const [cursor, setCursor] = useState(0)
  const select = useMapStore((s) => s.select)
  const setFocus = useMapStore((s) => s.setFocus)

  const term = query.trim().toLowerCase()
  const matches = term
    ? features.filter((f) => (f.properties.nama ?? '').toLowerCase().includes(term)).slice(0, 8)
    : []
  const open = focused || term !== ''
  const showResults = term !== ''
  const highlighted = matches.length ? Math.min(cursor, matches.length - 1) : 0

  function pick(feature: PipeFeature) {
    setFocus(feature)
    select(feature)
    setQuery('')
    setCursor(0)
  }

  function handleKey(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setCursor((c) => (matches.length ? (c + 1) % matches.length : 0))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setCursor((c) => (matches.length ? (c - 1 + matches.length) % matches.length : 0))
    } else if (e.key === 'Enter' && matches[highlighted]) {
      pick(matches[highlighted])
    } else if (e.key === 'Escape') {
      if (query) setQuery('')
      else e.currentTarget.blur()
    }
  }

  return (
    <div className={`island${open ? ' is-open' : ''}${showResults ? ' has-results' : ''}`} role="search">
      <div className="island-bar" onClick={(e) => e.currentTarget.querySelector('input')?.focus()}>
        <svg className="island-icon" viewBox="0 0 24 24" aria-hidden="true" width="18" height="18">
          <circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" strokeWidth="2" />
          <path d="M20 20l-4-4" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <input
          aria-label="Cari nama fitur"
          aria-expanded={showResults && matches.length > 0}
          placeholder="Cari nama fitur"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
            setCursor(0)
          }}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          onKeyDown={handleKey}
        />
        {query ? (
          <button type="button" className="island-clear" aria-label="Hapus pencarian" onMouseDown={(e) => e.preventDefault()} onClick={() => setQuery('')}>
            ×
          </button>
        ) : (
          !focused && <span className="island-count">{features.length} fitur</span>
        )}
      </div>

      {showResults && (
        <ul className="island-results" role="listbox">
          {matches.map((f, i) => (
            <li key={f.id} role="option" aria-selected={i === highlighted}>
              <button
                type="button"
                className={i === highlighted ? 'is-active' : ''}
                onMouseDown={(e) => e.preventDefault()}
                onMouseEnter={() => setCursor(i)}
                onClick={() => pick(f)}
              >
                <span className="island-dot" style={{ background: COLOR[f.properties.feature_type] }} />
                <span className="island-name">{f.properties.nama}</span>
                <small>{LABEL[f.properties.feature_type] ?? f.properties.feature_type}</small>
              </button>
            </li>
          ))}
          {matches.length === 0 && <li className="island-empty">Tidak ada fitur yang cocok di area ini.</li>}
        </ul>
      )}
    </div>
  )
}
