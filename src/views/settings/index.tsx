import type { ISetting } from './sidebar'
import { CircleAlert as CircleAlertIcon, Settings as SettingsIcon } from 'lucide-react'
import { useState } from 'react'
import { useI18n } from '@/hooks/i18n'
import { Main } from './main'
import { Sidebar } from './sidebar'

export default function Settings() {
  const { t } = useI18n()
  const settings: ISetting[] = [
    { name: 'General', label: t('window.settings.general.label'), icon: <SettingsIcon size="1.2rem" className="group-hover:animate-touch-stir mr-2.5" /> },
    { name: 'About', label: t('window.settings.about.label'), icon: <CircleAlertIcon size="1.2rem" className="group-hover:animate-touch-stir mr-2.5" /> },
  ]
  const [selected, setSelected] = useState(settings[0].name)

  return (
    <div className="w-full h-full flex">
      <Sidebar
        settings={settings}
        selected={selected}
        onSelected={setting => setSelected(setting.name)}
      />

      <Main selected={selected} />
    </div>
  )
}
