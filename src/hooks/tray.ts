import type { TrayIconOptions } from '@tauri-apps/api/tray'
import { defaultWindowIcon } from '@tauri-apps/api/app'
import { Menu } from '@tauri-apps/api/menu'
import { TrayIcon } from '@tauri-apps/api/tray'
import { CMDS } from '@/utils/cmds'
import { ENV, TRAY_ID } from '@/utils/constant'
import { WINDOW } from '@/utils/window'
import { i18n } from './i18n'

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
        await WINDOW.MAIN?.show()
        await WINDOW.MAIN?.setFocus()
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
        id: 'open_main',
        text: i18n.t('tray.menu.open_main'),
        async action() {
          await WINDOW.MAIN?.show()
          await WINDOW.MAIN?.setFocus()
        },
      },
      {
        id: 'open_settings',
        text: i18n.t('tray.menu.open_settings'),
        async action() {
          await WINDOW.SETTINGS?.show()
          await WINDOW.SETTINGS?.setFocus()
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
          CMDS.restart_app()
        },
      },
      {
        id: 'quit',
        text: i18n.t('tray.menu.quit'),
        action() {
          CMDS.exit_app()
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
