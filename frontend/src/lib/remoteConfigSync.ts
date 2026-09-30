// remoteConfigSync.ts
//
// 远程服务配置（服务器地址/授权码/总开关）的跨组件实时同步。
// 用户中心（RemoteServiceSection）和欢迎弹窗（WelcomeDialog）都能编辑这份配置，
// 任何一处保存成功后 dispatch 本事件，另一处监听并刷新自己的表单状态，
// 避免「在 A 处填了授权码，打开 B 处还要再填一次」。

export const REMOTE_CONFIG_CHANGED_EVENT = 'vox:remote-config-changed'

export interface RemoteConfigChangedDetail {
  serverUrl?: string
  license?: string
  enabled?: boolean
  /** serverUrl 变化时带上：当前是否内置默认地址（设置页高级卡片改地址后派发） */
  isDefault?: boolean
}

export function dispatchRemoteConfigChanged(detail: RemoteConfigChangedDetail) {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent(REMOTE_CONFIG_CHANGED_EVENT, { detail }))
}

export function onRemoteConfigChanged(handler: (detail: RemoteConfigChangedDetail) => void): () => void {
  const fn = (e: Event) => handler((e as CustomEvent<RemoteConfigChangedDetail>).detail || {})
  window.addEventListener(REMOTE_CONFIG_CHANGED_EVENT, fn)
  return () => window.removeEventListener(REMOTE_CONFIG_CHANGED_EVENT, fn)
}
