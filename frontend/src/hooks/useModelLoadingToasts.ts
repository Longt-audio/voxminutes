import { useEffect, useRef } from 'react'
import { toast } from 'sonner'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onModelLoading } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'

/**
 * 全局监听后端 model-loading 事件，用 toast 提示模型加载/卸载进度。
 * start 用固定 id 的 loading toast（不自动消失，同模型重复 start 只会更新原 toast）；
 * done / error 复用同一 id 替换它，分别在 3s / 6s 后自动消失；
 * 每个模型本会话首次 done 用「首次加载」文案（含耗时 + 「清空模型后台」按钮提示）；
 * unloaded（后台卸载：手动清空 / sidecar 换载 / 闲置超时）用 info toast 提示。
 * 在 AppShell 中挂载一次即可全局生效。
 */
export function useModelLoadingToasts() {
  const t = useMessages()
  const tRef = useRef(t)
  tRef.current = t
  // 本会话已完成首次加载的模型集合（卸载后重新加载不再算首次）
  const firstLoadSeen = useRef<Set<string>>(new Set())

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
        if (firstLoadSeen.current.has(e.model)) {
          toast.success(m.modelLoadingDone.replace('{model}', name).replace('{seconds}', seconds), {
            id,
            duration: 3000,
          })
        } else {
          firstLoadSeen.current.add(e.model)
          toast.success(
            m.modelLoadingFirstDone.replace('{model}', name).replace('{seconds}', seconds),
            { id, duration: 8000 }
          )
        }
      } else if (e.phase === 'unloaded') {
        const template =
          e.message === 'idle'
            ? m.modelUnloadedIdle
            : e.message === 'swap'
              ? m.modelUnloadedSwap
              : m.modelUnloaded
        toast.info(template.replace('{model}', name), { id, duration: 4000 })
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

/** model-loading 事件中的模型标识 → 界面显示名；查不到映射就原样显示。
 *  本地模型统一拼「（本地）」后缀（此处拿不到 t，用中文常量，与 mdLocalSuffix 的 zh 值一致）。 */
export function modelDisplayName(model: string): string {
  const key = model.toLowerCase()
  let name = model
  if (key.startsWith('x-asr')) name = 'X-ASR'
  else if (key.startsWith('sense-voice')) name = 'SenseVoice'
  else if (key.startsWith('opus-mt')) name = 'OPUS-MT'
  else if (key.startsWith('hy-mt2')) name = 'Hy-MT2'
  else if (key.includes('qwen2.5-3b')) name = 'Qwen2.5-3B'
  else if (key.includes('qwen3-4b')) name = 'Qwen3-4B-2507'
  else if (key.includes('gemma-3-4b')) name = 'Gemma-3-4B'
  return `${name}（本地）`
}
