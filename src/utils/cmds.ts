import { invoke } from '@tauri-apps/api/core'

export interface IRustDeviceInfo {
  host: string
  fullname: string
  addresses: IRustAddressInfo
  meta_info: IRustMetaInfo
}

export interface IRustAddressInfo {
  v4: string[]
  v6: string[]
}

export interface IRustMetaInfo {
  version: string
  port: number
  uuid: string
  fingerprint: string
}

const CMDS_NAME = {
  EXIT_APP: 'exit_app',
  RESTART_APP: 'restart_app',
  START_SERVER: 'start_server',
  ALLOW_DEVICE_ADD: 'allow_device_add',
  ALLOW_DEVICE_REMOVE: 'allow_device_remove',
  GET_DEVICES: 'get_devices',
}

export const CMDS = {
  exit_app: () => invoke<void>(CMDS_NAME.EXIT_APP),
  restart_app: () => invoke<void>(CMDS_NAME.RESTART_APP),
  start_server: () => invoke<void>(CMDS_NAME.START_SERVER),
  allow_device_add: (id: string) => invoke<boolean>(CMDS_NAME.ALLOW_DEVICE_ADD, { id }),
  allow_device_remove: (id: string) => invoke<boolean>(CMDS_NAME.ALLOW_DEVICE_REMOVE, { id }),
  get_devices: () => invoke<IRustDeviceInfo[]>(CMDS_NAME.GET_DEVICES),
}
