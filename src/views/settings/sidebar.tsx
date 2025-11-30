import type { SettingItem } from './main'
import React from 'react'

export interface ISetting {
  name: SettingItem
  label: string
  icon: React.JSX.Element
}

export interface ISidebarProps {
  settings: ISetting[]
  selected: ISetting['name']
  onSelected: (setting: ISetting) => void
}

export const Sidebar: React.FC<ISidebarProps> = ({ settings, selected, onSelected }) => {
  return (
    <aside className="p-5 py-6 w-56 h-inherit overflow-y-auto select-none">
      {settings.map(setting => (
        <span
          key={setting.label}
          className={`group relative flex items-center px-3 py-2 rounded-sm text-[#878593] mb-2 hover:bg-[#f5f5f5] 
            ${setting.name === selected ? 'font-bold text-white bg-[#211f2d]!' : ''}`}
          onClick={() => onSelected(setting)}
        >
          {setting.icon}
          <span>{setting.label}</span>
        </span>
      ))}
    </aside>
  )
}
