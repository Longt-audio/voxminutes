'use client'

import { useEffect, useRef } from 'react'
import { toast } from 'sonner'
import { onModelDownloadProgress } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { importingModels } from './useModelDownload'

/**
 * 全局监听 model-download-progress 的 done/error 并统一 toast，在 AppShell 挂载一次。
 * 导入（import_model_file）的 done/error 事件跳过——其结果由发起方 importModel toast，避免重复。
 */
export function useModelDownloadToasts() {
  const t = useMessages()
  const tRef = useRef(t)
  tRef.current = t

  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined

    onModelDownloadProgress((p) => {
      if (importingModels.has(p.modelId)) return
      if (p.stage === 'done') {
        toast.success(tRef.current.setDownloadDone)
      } else if (p.stage === 'error') {
        toast.error(p.message || tRef.current.setDownloadFailed)
      }
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [])
}
