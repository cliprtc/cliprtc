import type { IRustDeviceInfo } from '@/utils/cmds'
import type { IDeviceInfo } from '@/utils/store'
import { listen } from '@tauri-apps/api/event'
import { InfoIcon, LinkIcon, MonitorSmartphoneIcon, UnlinkIcon } from 'lucide-react'
import React, { useEffect, useMemo, useState } from 'react'
import { Card, CardContent } from '@/components/ui/card'
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@/components/ui/empty'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useI18n } from '@/hooks/i18n'
import { CMDS } from '@/utils/cmds'
import { EVENT_NAMES } from '@/utils/event'
import { INFO_KEYS, INFO_STORE } from '@/utils/store'

function mergeDevices(prev: IDeviceInfo[], next: IRustDeviceInfo[]): IDeviceInfo[] {
  const map = new Map<string, IDeviceInfo>()

  for (const d of prev) {
    map.set(d.uuid, { ...d, ip: [...d.ip] })
  }

  for (const device of next) {
    const uuid = device.meta_info.uuid
    const old = map.get(uuid)

    const v4 = device.addresses.v4 || []
    const v6 = device.addresses.v6 || []

    const mergedIp = old
      ? [...old.ip, ...v4, ...v6]
      : [...v4, ...v6]

    const ip = Array.from(new Set(mergedIp)).sort((a, b) => {
      const aIsV4 = a.includes('.')
      const bIsV4 = b.includes('.')
      return aIsV4 === bIsV4 ? 0 : aIsV4 ? -1 : 1
    })

    map.set(uuid, {
      uuid,
      hostname: device.host.split('.')[0],
      online: false,
      version: device.meta_info.version,
      ip,
    })
  }

  return Array.from(map.values())
}

type TTabs = 'my' | 'other'

export default function Main() {
  const { t } = useI18n()
  const [initialized, setInitialized] = useState(false)
  const [selected, setSelected] = useState<TTabs>('my')
  const [devices, setDevices] = useState<IDeviceInfo[]>([])
  const [linkDeviceInfo, setLinkDeviceInfo] = useState<IDeviceInfo[]>([])

  async function getDevices() {
    const devicesRaw = await CMDS.get_devices()
    setDevices(prev => mergeDevices(prev, devicesRaw))
  }

  useEffect(() => {
    if (!initialized) {
      return
    }
    INFO_STORE.set(INFO_KEYS.LINK_DEVICE_INFO, linkDeviceInfo)
    linkDeviceInfo.map(d => CMDS.allow_device_add(d.uuid))
  }, [linkDeviceInfo, initialized])

  function handleLink(device: IDeviceInfo) {
    CMDS.allow_device_add(device.uuid)
    setLinkDeviceInfo(prev => Array.from(new Map([...prev, device].map(item => [item.uuid, item])).values()))
  }
  function handleUnlink(device: IDeviceInfo) {
    CMDS.allow_device_remove(device.uuid)
    setLinkDeviceInfo(prev => prev.filter(d => d.uuid !== device.uuid))
  }

  const { myDevices, otherDevices } = useMemo(() => {
    const my = new Map(linkDeviceInfo.map(ld => [ld.uuid, ld]))
    const other = new Map()

    for (const device of devices) {
      const d = my.get(device.uuid)
      if (d) {
        my.set(device.uuid, { ...d, online: true })
        continue
      }
      other.set(device.uuid, { ...device, online: true })
    }

    const sortByOnline = (a: IDeviceInfo, b: IDeviceInfo) => {
      if (a.online === b.online) {
        return 0
      }
      return a.online ? -1 : 1
    }

    const myDevices = [...my.values()].sort(sortByOnline)
    const otherDevices = [...other.values()].sort(sortByOnline)

    return { myDevices, otherDevices }
  }, [devices, linkDeviceInfo])

  useEffect(() => {
    (async () => {
      getDevices()
    })()

    const unlistenPromise = listen(EVENT_NAMES.DEVICE_FOUND, () => {
      getDevices()
    })

    INFO_STORE.get<IDeviceInfo[]>(INFO_KEYS.LINK_DEVICE_INFO).then((value) => {
      if (value) {
        const devices = value.map(i => ({ ...i, online: false }))
        setLinkDeviceInfo(devices)
      }
      setInitialized(true)
    })

    return () => {
      unlistenPromise.then(f => f())
    }
  }, [])
  return (
    <div className="w-full h-full p-3">
      <Tabs value={selected} onValueChange={(value: any) => setSelected(value)}>
        <TabsList>
          <TabsTrigger value="my" className="cursor-pointer">{t('window.main.tabs.my.title')}</TabsTrigger>
          <TabsTrigger value="other" className="cursor-pointer">{t('window.main.tabs.other.title')}</TabsTrigger>
        </TabsList>
        <TabsContent value="my">
          <DeviceCard
            device={{ devices: myDevices, onClick: handleUnlink }}
            empty={{ title: t('window.main.tabs.my.empty.title'), desc: t('window.main.tabs.my.empty.desc') }}
          />
        </TabsContent>
        <TabsContent value="other">
          <DeviceCard
            device={{ devices: otherDevices, onClick: handleLink, isLinked: false }}
            empty={{ title: t('window.main.tabs.other.empty.title'), desc: t('window.main.tabs.other.empty.desc') }}
          />
        </TabsContent>
      </Tabs>
    </div>
  )
}

interface DeviceCardProps {
  device: DeviceListProps
  empty: EmptyStateProps
}
function DeviceCard({ device, empty }: DeviceCardProps) {
  return (
    <Card className="">
      {
        device.devices.length
          ? (
              <ScrollArea className="">
                <CardContent className="max-h-[calc(100vh-90px)] grid gap-4">
                  <DeviceList devices={device.devices} onClick={device.onClick} isLinked={device.isLinked} />
                </CardContent>
              </ScrollArea>
            )
          : (<EmptyState title={empty.title} desc={empty.desc} icon={empty.icon} />)
      }
    </Card>
  )
}

interface DeviceListProps {
  devices: IDeviceInfo[]
  onClick: (device: IDeviceInfo) => void
  isLinked?: boolean
}
function DeviceList({ devices, onClick, isLinked = true }: DeviceListProps) {
  return (
    <>
      {devices.map(device => (
        <React.Fragment key={device.uuid}>
          <div className="gap-1.5 flex justify-between">
            <div className="">
              <div className="">
                {device.hostname}
              </div>
              <div className="text-xs text-gray-500">
                <div
                  className={
                    `mr-1 relative inline-block w-2 h-2 rounded-full
                    ${device.online ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`
                  }
                />
                IP:
                {device.ip[0]}
                <Popover>
                  <PopoverTrigger>
                    <InfoIcon className="ml-1 mb-0.5 inline w-4 cursor-pointer" />
                  </PopoverTrigger>
                  <PopoverContent className="p-2 w-fit" sideOffset={-2}>
                    <ul className="text-gray-500 text-xs">
                      {device.ip.map(ip => (
                        <li key={ip} className="mt-1">{ip}</li>
                      ))}
                    </ul>
                  </PopoverContent>
                </Popover>
              </div>
            </div>
            <div className="flex items-center">
              <div className="cursor-pointer" onClick={() => onClick(device)}>
                {isLinked ? <UnlinkIcon className="w-5" /> : <LinkIcon className="w-5" /> }
              </div>
            </div>
          </div>
          <Separator />
        </React.Fragment>
      ))}
    </>
  )
}

interface EmptyStateProps {
  title: string
  desc: string
  icon?: React.ReactNode
}
const EmptyDefaultIcon = <MonitorSmartphoneIcon />
function EmptyState({ title, desc, icon = EmptyDefaultIcon }: EmptyStateProps) {
  return (
    <CardContent className="h-[calc(100vh-90px)] flex">
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            {icon}
          </EmptyMedia>
          <EmptyTitle>{title}</EmptyTitle>
          <EmptyDescription>{desc}</EmptyDescription>
        </EmptyHeader>
      </Empty>
    </CardContent>
  )
}
