import type EN_US from '@/locales/en-US'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { I18n } from 'mini-i18n'
import { useSyncExternalStore } from 'react'
import { SETTINGS_STORE } from '@/utils/store'
import { updateTrayMenu } from './tray'

export const LANGUAGES = ['zh-CN', 'en-US'] as const

type TI18nLanguageKey = typeof LANGUAGES[number]
type TI18nLanguage = typeof EN_US
type TModules = Record<string, TI18nLanguage>

const modules: TModules = import.meta.glob('@/locales/*.ts', { eager: true, import: 'default' })
const languagesData = Object.entries(modules).map(([key, value]) => [key.split('/').pop()!.split('.')[0], value])
const languages: Record<TI18nLanguageKey, TI18nLanguage> = Object.fromEntries(languagesData)

export const i18n = new I18n({
  defaultLanguage: 'en-US',
  languages,
})

SETTINGS_STORE.get<TI18nLanguageKey>('language').then((lang) => {
  if (lang && LANGUAGES.includes(lang)) {
    i18n.setLanguage(lang)
  }
})

const win = getCurrentWindow()
function setTitle() {
  const title = i18n.t('window.settings.title')
  win.setTitle(title)
}

async function onChangeLanguage() {
  setTitle()
  updateTrayMenu()
  const lang = i18n.getLanguage()
  await SETTINGS_STORE.set('language', lang)
  SETTINGS_STORE.save()
}

function subscribe(callback: () => void) {
  const off = i18n.on('language:changed', () => {
    onChangeLanguage()
    callback()
  })
  return () => {
    off()
  }
}

function getSnapshot() {
  return i18n.getLanguage()
}

export function useI18n() {
  const lang = useSyncExternalStore(subscribe, getSnapshot)
  return {
    t: i18n.t.bind(i18n),
    lang,
    setLang: i18n.setLanguage.bind(i18n),
  }
}
