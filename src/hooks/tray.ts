import type { TrayIconOptions } from '@tauri-apps/api/tray'
import { defaultWindowIcon } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { Menu } from '@tauri-apps/api/menu'
import { TrayIcon } from '@tauri-apps/api/tray'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { CMDS, ENV, TRAY_ID } from '../utils/constant'
import { i18n } from './i18n'

const win = getCurrentWindow()

async function getTray(id = TRAY_ID) {
  return TrayIcon.getById(id)
}

export async function createTray() {
  const tray = await getTray()
  if (tray) {
    return
  }

  const menu = await getTrayMenu()
  const trayIconOptions: TrayIconOptions = {
    id: TRAY_ID,
    menu,
    tooltip: `${ENV.appName} v${ENV.appVersion}`,
    menuOnLeftClick: false,
    async action(event) {
      if (event.type === 'Click' && event.button === 'Left' && event.buttonState === 'Up') {
        await win.show()
        await win.setFocus()
      }
    },
  }

  const icon = await defaultWindowIcon()
  if (icon) {
    trayIconOptions.icon = icon
  }

  return TrayIcon.new(trayIconOptions)
}

async function getTrayMenu() {
  const menu = await Menu.new({
    items: [
      {
        id: 'open_settings',
        text: i18n.t('tray.menu.open_settings'),
        async action() {
          await win.show()
          await win.setFocus()
        },
      },
      { item: 'Separator' },
      {
        id: 'version',
        text: `${i18n.t('tray.menu.version')} v${ENV.appVersion}`,
        enabled: false,
      },
      {
        id: 'restart_app',
        text: i18n.t('tray.menu.restart'),
        action() {
          invoke(CMDS.RESTART_APP)
        },
      },
      {
        id: 'quit',
        text: i18n.t('tray.menu.quit'),
        action() {
          invoke(CMDS.EXIT_APP)
        },
      },
    ],
  })

  return menu
}

export async function updateTrayMenu() {
  const tray = await getTray()

  if (!tray) {
    return
  }

  const menu = await getTrayMenu()

  tray.setMenu(menu)
}
