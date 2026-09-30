import { create } from 'zustand'
import { getRemoteEnabled, listRemoteCatalog, type RemoteModelItem } from '@/services/ipc'

// 网关随 /v1/models 给每个远程模型下发 tags（后台可配，最多 4 个，「推荐」也在其中）。
// ipc.ts 的 RemoteModelItem 由其他模块维护，这里用 TS 模块增强补该可选字段：
// 旧网关不下发 → undefined，UI 回退到内置徽标逻辑（在线远程/流式/推荐等）。
declare module '@/services/ipc' {
  interface RemoteModelItem {
    /** 网关后台配置的模型标签（最多 4 个，原样渲染，客户端不做翻译） */
    tags?: string[]
  }
}

/** 远程模型目录全局 store：AppShell 挂载时启动 60s 轮询，
 *  后台（网关）改了模型上架/价格/推荐，前端最多 60s 内自动生效。
 *  updatedAt 不变时不 set，避免无意义重渲染。 */
interface RemoteCatalogState {
  models: RemoteModelItem[]
  updatedAt: string
  loading: boolean
  refresh: (force?: boolean) => Promise<void>
  startPolling: () => void
  stopPolling: () => void
}

const POLL_INTERVAL_MS = 60_000
let pollTimer: ReturnType<typeof setInterval> | null = null

export const useRemoteCatalogStore = create<RemoteCatalogState>((set, get) => ({
  models: [],
  updatedAt: '',
  loading: false,

  refresh: async (force = false) => {
    try {
      const enabled = await getRemoteEnabled().catch(() => false)
      if (!enabled) {
        // 远程服务关闭：清空目录
        if (get().models.length > 0 || get().updatedAt) {
          set({ models: [], updatedAt: '', loading: false })
        }
        return
      }
      const { models, updatedAt } = await listRemoteCatalog()
      if (force || updatedAt !== get().updatedAt) {
        set({ models, updatedAt, loading: false })
      }
    } catch {
      set({ loading: false })
    }
  },

  startPolling: () => {
    if (pollTimer) return // 防重复启动
    pollTimer = setInterval(() => {
      void get().refresh()
    }, POLL_INTERVAL_MS)
  },

  stopPolling: () => {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  },
}))

/** 按 kind 过滤 + 排序：recommended 在前，其次 display_name/id 字典序 */
export function catalogByKind(models: RemoteModelItem[], kind: 'asr' | 'translate' | 'tts'): RemoteModelItem[] {
  return models
    .filter((m) => m.kind === kind)
    .sort((a, b) => {
      if (!!a.recommended !== !!b.recommended) return a.recommended ? -1 : 1
      const an = a.display_name?.trim() || a.id
      const bn = b.display_name?.trim() || b.id
      return an.localeCompare(bn)
    })
}

/** 按 id 查找模型（用于按模型消耗页等只有 model id 的显示名映射） */
export function findRemoteModel(models: RemoteModelItem[], id: string): RemoteModelItem | undefined {
  return models.find((m) => m.id === id)
}
