import { getCurrentWindow } from '@tauri-apps/api/window'
import { createTray } from '@/hooks/tray'
import { EWindow } from '../types/win'

const win = getCurrentWindow()

export async function init() {
  if (win.label === EWindow.Settings) {
    createTray()
  }
}
