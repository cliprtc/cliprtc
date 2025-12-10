export const Windows = {
  MAIN: 'main',
  SETTINGS: 'settings',
} as const
export type WindowsKey = keyof typeof Windows
export type WindowsValue = typeof Windows[keyof typeof Windows]
