import type { TLanguageKey } from './store'
import { EventEmitter } from 'mini-emit'

export interface CrossTabEvents {
  'i18n:language:changed': { language: TLanguageKey }
}
export type EventNames = keyof CrossTabEvents

export class CrossTabEvent extends EventEmitter<CrossTabEvents> {
  #channel: BroadcastChannel
  constructor(channelName = 'global-cross-tab-channel') {
    super()

    this.#channel = new BroadcastChannel(channelName)
    this.#channel.onmessage = (e) => {
      const data = e.data as { event: EventNames, payload: CrossTabEvents[EventNames] }
      super.emit(data.event, data.payload)
    }
  }

  emit<K extends EventNames>(event: K, payload: CrossTabEvents[K]) {
    this.#channel.postMessage({ event, payload })
  }

  close() {
    super.clear()
    this.#channel.close()
  }
}
