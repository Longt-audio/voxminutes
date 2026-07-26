import { useEffect, useRef } from 'react'
import { toast } from 'sonner'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onModelLoading } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'

/**
 * 全局监听后端 model-loading 事件，用 toast 提示模型加载进度。
 * start 用固定 id 的 loading toast（不自动消失，同模型重复 start 只会更新原 toast）；
 * done / error 复用同一 id 替换它，分别在 3s / 6s 后自动消失。
 * 在 AppShell 中挂载一次即可全局生效。
 */
export function useModelLoadingToasts() {
  const t = useMessages()
  const tRef = useRef(t)
  tRef.current = t

  useEffect(() => {
    let unlisten: UnlistenFn | undefined
    onModelLoading((e) => {
      const id = `model-loading:${e.model}`
      const name = modelDisplayName(e.model)
      const m = tRef.current
      if (e.phase === 'start') {
        toast.loading(m.modelLoadingStart.replace('{model}', name), { id, duration: Infinity })
      } else if (e.phase === 'done') {
        const seconds = ((e.elapsed_ms ?? 0) / 1000).toFixed(1)
        toast.success(m.modelLoadingDone.replace('{model}', name).replace('{seconds}', seconds), {
          id,
          duration: 3000,
        })
      } else {
        toast.error(
          m.modelLoadingError.replace('{model}', name).replace('{message}', e.message ?? ''),
          { id, duration: 6000 }
        )
      }
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(() => {})
    return () => {
      unlisten?.()
    }
  }, [])
}

/** model-loading 事件中的模型标识 → 界面显示名；查不到映射就原样显示。 */
export function modelDisplayName(model: string): string {
  const key = model.toLowerCase()
  if (key.startsWith('x-asr')) return 'X-ASR'
  if (key.startsWith('sense-voice')) return 'SenseVoice'
  if (key.startsWith('opus-mt')) return 'OPUS-MT'
  if (key.startsWith('hy-mt2')) return 'Hy-MT2'
  if (key.includes('qwen2.5-3b')) return 'Qwen2.5-3B'
  if (key.includes('qwen3-4b')) return 'Qwen3-4B-2507'
  if (key.includes('gemma-3-4b')) return 'Gemma-3-4B'
  return model
}
