import { LAYER_LABELS, PIPE_LABELS } from './store'
import type { FeatureType, PipeFunction } from './types'

type Props = {
  active: FeatureType[]
  pipeFunctions: PipeFunction[]
  canSeeCustomerData: boolean
  onToggle: (type: FeatureType) => void
  onTogglePipe: (fungsi: PipeFunction) => void
}

// Warna titik kecil di chip, sama dengan warna di peta.
const COLOR: Record<PipeFunction | Exclude<FeatureType, 'pipa'>, string> = {
  transmisi: '#1e3a8a',
  distribusi: '#1d4ed8',
  valve: '#111111',
  reservoir: '#0f766e',
  bronkaptering: '#0891b2',
  sr: '#a16207',
}

const PIPE_ORDER: PipeFunction[] = ['transmisi', 'distribusi']
const ORDER: Exclude<FeatureType, 'pipa'>[] = ['valve', 'reservoir', 'bronkaptering', 'sr']

function Chip({ label, color, pressed, onClick }: { label: string; color: string; pressed: boolean; onClick: () => void }) {
  return (
    <button type="button" className="chip" aria-pressed={pressed} onClick={onClick} style={{ ['--c' as string]: color }}>
      <span className="dot" />
      {label}
    </button>
  )
}

export function LayerSwitcher({ active, pipeFunctions, canSeeCustomerData, onToggle, onTogglePipe }: Props) {
  return (
    <div className="chips" role="group" aria-label="Layer">
      {PIPE_ORDER.map((fungsi) => (
        <Chip
          key={fungsi}
          label={PIPE_LABELS[fungsi]}
          color={COLOR[fungsi]}
          pressed={pipeFunctions.includes(fungsi)}
          onClick={() => onTogglePipe(fungsi)}
        />
      ))}
      {ORDER.filter((type) => canSeeCustomerData || type !== 'sr').map((type) => (
        <Chip
          key={type}
          label={LAYER_LABELS[type]}
          color={COLOR[type]}
          pressed={active.includes(type)}
          onClick={() => onToggle(type)}
        />
      ))}
    </div>
  )
}
