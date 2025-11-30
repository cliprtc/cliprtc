import { getName, getVersion } from '@tauri-apps/api/app'
import { isDev } from '.'

export const TRAY_ID = 'app-tray'

export const CMDS = {
  EXIT_APP: 'exit_app',
  RESTART_APP: 'restart_app',
}

export const ENV = {
  isDev: isDev(),
  appName: '',
  appVersion: '',
}

getName().then(name => ENV.appName = name)
getVersion().then(v => ENV.appVersion = v)
