import { LAYER_LABELS } from './store'
import type { FeatureType } from './types'

type Props = {
  active: FeatureType[]
  canSeeCustomerData: boolean
  onToggle: (type: FeatureType) => void
}

const ORDER: FeatureType[] = ['pipa', 'valve', 'reservoir', 'bronkaptering', 'sr']

export function LayerSwitcher({ active, canSeeCustomerData, onToggle }: Props) {
  return (
    <fieldset className="layers">
      <legend>Layer</legend>
      {ORDER.filter((type) => canSeeCustomerData || type !== 'sr').map((type) => (
        <label key={type} className="layer-row">
          <input type="checkbox" checked={active.includes(type)} onChange={() => onToggle(type)} />
          <span>{LAYER_LABELS[type]}</span>
        </label>
      ))}
    </fieldset>
  )
}
