import { LAYER_LABELS, PIPE_LABELS } from './store'
import type { FeatureType, PipeFunction } from './types'

type Props = {
  active: FeatureType[]
  pipeFunctions: PipeFunction[]
  canSeeCustomerData: boolean
  onToggle: (type: FeatureType) => void
  onTogglePipe: (fungsi: PipeFunction) => void
}

const PIPE_ORDER: PipeFunction[] = ['transmisi', 'distribusi']
const ORDER: Exclude<FeatureType, 'pipa'>[] = ['valve', 'reservoir', 'bronkaptering', 'sr']

export function LayerSwitcher({ active, pipeFunctions, canSeeCustomerData, onToggle, onTogglePipe }: Props) {
  return (
    <fieldset className="layers">
      <legend>Layer</legend>
      {PIPE_ORDER.map((fungsi) => (
        <label key={fungsi} className="layer-row">
          <input type="checkbox" checked={pipeFunctions.includes(fungsi)} onChange={() => onTogglePipe(fungsi)} />
          <span>{PIPE_LABELS[fungsi]}</span>
        </label>
      ))}
      {ORDER.filter((type) => canSeeCustomerData || type !== 'sr').map((type) => (
        <label key={type} className="layer-row">
          <input type="checkbox" checked={active.includes(type)} onChange={() => onToggle(type)} />
          <span>{LAYER_LABELS[type]}</span>
        </label>
      ))}
    </fieldset>
  )
}
