import { LazyStore } from '@tauri-apps/plugin-store'
import { isDev } from '.'

export const LANGUAGES = ['zh-CN', 'en-US'] as const
export type TLanguageKey = typeof LANGUAGES[number]

const extname = isDev() ? 'dev.json' : 'json'

export interface ISettings {
  key: string
  auto_start: boolean
  language: TLanguageKey
}
type TSettingsKeys = keyof ISettings
const settings_path = `stores/.settings.${extname}`
export const SETTINGS_STORE = new LazyStore(settings_path)
export const SETTINGS_KEYS: Record<Uppercase<TSettingsKeys>, TSettingsKeys> = {
  KEY: 'key',
  AUTO_START: 'auto_start',
  LANGUAGE: 'language',
}

export interface IInfo {
  uuid: string
  link_device_info: IDeviceInfo[]
}
type TInfoKeys = keyof IInfo
export interface IDeviceInfo {
  uuid: string
  hostname: string
  online: boolean
  version: string
  ip: string[]
}
const info_path = `stores/.info.${extname}`
export const INFO_STORE = new LazyStore(info_path)
export const INFO_KEYS: Record<Uppercase<TInfoKeys>, TInfoKeys> = {
  UUID: 'uuid',
  LINK_DEVICE_INFO: 'link_device_info',
}
