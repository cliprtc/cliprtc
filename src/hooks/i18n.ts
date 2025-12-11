import type EN_US from '@/locales/en-US'
import type { TLanguageKey } from '@/utils/store'
import { I18n } from 'mini-i18n'
import { useSyncExternalStore } from 'react'
import { CrossTabEvent } from '@/utils/cross-tab-events'
import { LANGUAGES, SETTINGS_STORE } from '@/utils/store'
import { WINDOW } from '@/utils/window'
import { updateTrayMenu } from './tray'

export type TI18nLanguage = typeof EN_US
type TModules = Record<string, TI18nLanguage>

const modules: TModules = import.meta.glob('@/locales/*.ts', { eager: true, import: 'default' })
const languagesData = Object.entries(modules).map(([key, value]) => [key.split('/').pop()!.split('.')[0], value])
const languages: Record<TLanguageKey, TI18nLanguage> = Object.fromEntries(languagesData)

let defaultLanguage: TLanguageKey = 'en-US'
export const i18n = new I18n({
  defaultLanguage,
  languages,
})

const crossTabEvent = new CrossTabEvent()
crossTabEvent.on('i18n:language:changed', ({ language }) => {
  i18n.setLanguage(language)
})

SETTINGS_STORE.get<TLanguageKey>('language').then((lang) => {
  if (lang && LANGUAGES.includes(lang)) {
    i18n.setLanguage(lang)
  }
})

function setTitle() {
  const title = i18n.t('window.settings.title')
  WINDOW.SETTINGS?.setTitle(title)
}

async function onChangeLanguage() {
  setTitle()
  updateTrayMenu()
  const lang = i18n.getLanguage()
  await SETTINGS_STORE.set('language', lang)
  SETTINGS_STORE.save()
}

function subscribe(callback: () => void) {
  const off = i18n.on('language:changed', (payload) => {
    callback()
    if (defaultLanguage === payload.language) {
      return
    }
    defaultLanguage = payload.language
    crossTabEvent.emit('i18n:language:changed', payload)
    onChangeLanguage()
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
