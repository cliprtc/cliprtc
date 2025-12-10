import type { Window } from '@tauri-apps/api/window'
import type { WindowsKey } from '@/types/win'
import { getAllWindows } from '@tauri-apps/api/window'
import { Windows } from '@/types/win'

export const WINDOW: Partial<Record<WindowsKey, Window>> = {}
export async function initWindows() {
  const wins = await getAllWindows()
  wins.forEach((w) => {
    if (w.label === Windows.MAIN) { WINDOW.MAIN = w }
    if (w.label === Windows.SETTINGS) { WINDOW.SETTINGS = w }
  })
}
