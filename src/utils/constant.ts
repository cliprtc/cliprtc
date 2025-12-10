import { getName, getVersion } from '@tauri-apps/api/app'
import { isDev } from '.'

export const TRAY_ID = 'app-tray'

export const ENV = {
  isDev: isDev(),
  appName: '',
  appVersion: '',
}

getName().then(name => ENV.appName = name)
getVersion().then(v => ENV.appVersion = v)
