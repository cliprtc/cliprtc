import { LazyStore } from '@tauri-apps/plugin-store'
import { isDev } from '.'

const extname = isDev() ? 'dev.json' : 'json'

const settings_path = `stores/.settings.${extname}`

export const SETTINGS_STORE = new LazyStore(settings_path)

export const SETTINGS_KEYS = {
  KEY: 'key',
  AUTO_START: 'auto_start',
  LANGUAGE: 'language',
}
