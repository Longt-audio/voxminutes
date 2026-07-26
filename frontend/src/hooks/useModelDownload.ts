'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import {
  getDownloadableModels,
  downloadModel,
  cancelModelDownload,
  importModelFile,
  onModelDownloadProgress,
} from '@/services/ipc'
import type { DownloadableModelInfo, ModelDownloadProgress } from '@/types'
import { useMessages } from '@/i18n/useMessages'

/**
 * 正在通过 import_model_file 导入的模型 id（模块级）。
 * 全局 toast hook（useModelDownloadToasts）用它跳过导入的 done/error 事件，
 * 避免与 importModel 的结果 toast 重复。
 */
export const importingModels = new Set<string>()

/**
 * 模型下载/导入共享逻辑：模型列表 + 进度事件订阅 + 下载/取消/导入动作。
 * 设置页 ModelDownloadCard 与首次启动向导 OnboardingDialog 共用，避免两份进度逻辑。
 * done/error 的 toast 由全局 useModelDownloadToasts（下载）与 importModel（导入）分别负责。
 */
export function useModelDownload() {
  const t = useMessages()
  const tRef = useRef(t)
  tRef.current = t
  const [models, setModels] = useState<DownloadableModelInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [progressMap, setProgressMap] = useState<Record<string, ModelDownloadProgress>>({})
  const [importingIds, setImportingIds] = useState<string[]>([])

  const refresh = useCallback(() => {
    getDownloadableModels()
      .then(setModels)
      .catch(() => setModels([]))
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  // 监听下载/导入进度事件（后端两条路径都走 model-download-progress），卸载时取消监听
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined

    onModelDownloadProgress((p) => {
      if (p.stage === 'done' || p.stage === 'error' || p.stage === 'cancelled') {
        setProgressMap((prev) => {
          const next = { ...prev }
          delete next[p.modelId]
          return next
        })
        refresh()
      } else {
        setProgressMap((prev) => ({ ...prev, [p.modelId]: p }))
      }
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [refresh])

  // 下载结果通过 model-download-progress 事件反馈；invoke 直接报错
  // （如同一模型重复下载）时 toast
  const startDownload = useCallback(
    (modelId: string) => {
      downloadModel(modelId)
        .catch((e) => toast.error(String(e)))
        .finally(() => refresh())
    },
    [refresh]
  )

  const cancelDownload = useCallback(
    (modelId: string) => {
      cancelModelDownload(modelId)
        .catch(() => {})
        .finally(() => refresh())
    },
    [refresh]
  )

  // 导入：后端弹原生文件选择框；cancelled 静默，done/error toast；进度复用同一事件
  const importModel = useCallback(
    async (modelId: string) => {
      importingModels.add(modelId)
      setImportingIds((prev) => [...prev, modelId])
      try {
        const res = await importModelFile(modelId)
        if (res.status === 'done') {
          toast.success(tRef.current.setImportDone)
        } else if (res.status === 'error') {
          toast.error(res.message || tRef.current.setImportFailed.replace('{error}', ''))
        }
        // cancelled：用户在选择框取消，静默
      } catch (e) {
        toast.error(tRef.current.setImportFailed.replace('{error}', String(e)))
      } finally {
        importingModels.delete(modelId)
        setImportingIds((prev) => prev.filter((id) => id !== modelId))
        refresh()
      }
    },
    [refresh]
  )

  const isModelBusy = useCallback(
    (m: DownloadableModelInfo) => m.downloading || !!progressMap[m.id] || importingIds.includes(m.id),
    [progressMap, importingIds]
  )

  return {
    models,
    loading,
    progressMap,
    refresh,
    startDownload,
    cancelDownload,
    importModel,
    isModelBusy,
  }
}
