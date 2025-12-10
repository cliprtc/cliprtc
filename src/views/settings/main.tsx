import { getName, getVersion } from '@tauri-apps/api/app'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { useEffect, useState } from 'react'
import { bugs, repository } from '@/../package.json'
import logo from '@/assets/img/logo/cliprtc-512x512.png'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { Avatar, AvatarImage } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { useI18n } from '@/hooks/i18n'
import { CMDS } from '@/utils/cmds'
import { LANGUAGES, SETTINGS_KEYS, SETTINGS_STORE } from '@/utils/store'

const components = {
  General,
  About,
} as const

export type SettingItem = keyof typeof components

export interface IProps {
  selected: SettingItem
}

export const Main: React.FC<IProps> = ({ selected }) => {
  const Component = components[selected]

  return (
    <main className="bg-[#eee] flex-1">
      <ScrollArea>
        <div className="p-5">
          <Component />
        </div>
      </ScrollArea>
    </main>
  )
}

function General() {
  const [key, setKey] = useState('')
  const [autoStart, setAutoStart] = useState(false)
  const { t, lang, setLang } = useI18n()

  useEffect(() => {
    (async () => {
      const value = await SETTINGS_STORE.get<string>('key')
      setKey(value!)

      const isEnable = await isEnabled()
      setAutoStart(isEnable)
    })()
  }, [])

  const onSaveKey = async () => {
    await SETTINGS_STORE.set(SETTINGS_KEYS.KEY, key)
    await SETTINGS_STORE.save()
    await CMDS.restart_app()
  }

  const onChangeAutoStart = async (checked: boolean) => {
    setAutoStart(checked)
    await SETTINGS_STORE.set(SETTINGS_KEYS.AUTO_START, checked)
    await SETTINGS_STORE.save()
    await (checked ? enable() : disable())
  }

  return (
    <div className="mb-6">
      <h2 className="text-base font-medium text-gray-800 mb-3">{t('window.settings.general.subtitle')}</h2>

      {/* Key */}
      <div className="flex items-center justify-between p-4 bg-white rounded shadow-sm mb-2">
        <div className="flex-1">
          <p className="text-gray-900">{t('window.settings.general.key.name')}</p>
          <p className="text-xs text-gray-500">{t('window.settings.general.key.tips')}</p>

          <div className="mt-2.5 flex gap-2">
            <Input
              type="text"
              placeholder="Key"
              value={key}
              onChange={e => setKey(e.target.value)}
            />

            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="default" disabled={!key}>{t('public.button.save')}</Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>{t('window.settings.general.key.alert.title')}</AlertDialogTitle>
                  <AlertDialogDescription>{t('window.settings.general.key.alert.tips')}</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>{t('public.button.cancel')}</AlertDialogCancel>
                  <AlertDialogAction onClick={onSaveKey}>{t('public.button.confirm')}</AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </div>
      </div>

      {/* Auto Start */}
      <div className="flex items-center justify-between p-4 bg-white rounded shadow-sm mb-2">
        <div>
          <p className="text-gray-900">{t('window.settings.general.autoStart.name')}</p>
          <p className="text-xs text-gray-500">{t('window.settings.general.autoStart.tips')}</p>
        </div>

        <Switch checked={autoStart} onCheckedChange={onChangeAutoStart} />
      </div>

      {/* Switch Language */}
      <div className="flex items-center justify-between p-4 bg-white rounded shadow-sm mb-2">
        <div>
          <p className="text-gray-900">{t('window.settings.general.switchLanguage.name')}</p>
          <p className="text-xs text-gray-500">{t('window.settings.general.switchLanguage.tips')}</p>
        </div>

        <Select value={lang} onValueChange={value => setLang(value as typeof lang)}>
          <SelectTrigger className="w-[100px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {LANGUAGES.map(lang => <SelectItem value={lang} key={lang}>{lang}</SelectItem>)}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}

function About() {
  const { t } = useI18n()
  const [info, setInfo] = useState({ name: '', version: '' })
  useEffect(() => {
    (async () => {
      const [name, version] = await Promise.all([getName(), getVersion()])
      setInfo({ name, version })
    })()
  })

  return (
    <div className="mb-6">
      <h2 className="text-base font-medium text-gray-800 mb-3">{t('window.settings.about.subtitle')}</h2>

      <div className="flex flex-col p-4 bg-white rounded shadow-sm mb-2">
        {/* Logo */}
        <div className="flex">
          <div className="flex">
            <Avatar className="w-16 h-16 ">
              <AvatarImage src={logo}></AvatarImage>
            </Avatar>

            <div className="ml-4 flex flex-col justify-evenly">
              <p>{info.name}</p>
              <p className="text-sm text-gray-500">{`${t('window.settings.about.version')}: ${info.version}`}</p>
            </div>
          </div>
          <div className="flex">

          </div>
        </div>

        {/* GitHub */}
        <div className="flex-1 mt-4 flex items-center justify-between">
          <div>
            <p className="text-gray-900">{t('window.settings.about.source.label')}</p>
            <Button className="text-xs text-gray-500 p-0 h-0" variant="link">
              <a href={repository.url} target="_blank">{repository.url}</a>
            </Button>
          </div>
          <Button variant="destructive">
            <a href={bugs.url} target="_blank">{t('window.settings.about.source.issue')}</a>
          </Button>
        </div>
      </div>
    </div>
  )
}
