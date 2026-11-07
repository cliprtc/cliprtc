import type { WindowsValue } from '@/types/win'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { createTray } from '@/hooks/tray'
import { Windows } from '@/types/win'
import { initWindows } from './window'

const win = getCurrentWindow()

export async function init() {
  await initWindows()

  const LABEL = win.label as WindowsValue
  if (LABEL === Windows.SETTINGS) {
    createTray()
  }
}
