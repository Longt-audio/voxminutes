'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import {
  sherpaOnnxGetModelsDirectory,
  deleteModel,
} from '@/services/ipc'
import type { DownloadableModelInfo } from '@/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'
import { modelDisplayName, formatSize, stageText, modelGroup, modelDesc, type ModelGroup } from '@/lib/modelDisplay'
import { useModelDownload } from '@/hooks/useModelDownload'

/** 设置页 Section 1：ASR 模型目录展示 + 模型下载 / 导入 / 取消 / 删除管理 */
export function ModelDownloadCard() {
  const t = useMessages()
  const [modelsDir, setModelsDir] = useState('')
  const {
    models,
    loading,
    progressMap,
    refresh,
    startDownload,
    cancelDownload,
    importModel,
    isModelBusy,
  } = useModelDownload()

  useEffect(() => {
    sherpaOnnxGetModelsDirectory()
      .then(setModelsDir)
      .catch(() => {})
  }, [])

  const handleDelete = async (m: DownloadableModelInfo) => {
    if (!window.confirm(t.setDeleteConfirm.replace('{name}', modelDisplayName(m.id, t, m.display_name)))) return
    try {
      await deleteModel(m.id)
      toast.success(t.setModelDeleted)
    } catch (e) {
      toast.error(t.setDeleteFailed.replace('{error}', String(e)))
    } finally {
      refresh()
    }
  }

  // 单个模型行（含一句话描述与下载进度条）
  const renderModel = (m: DownloadableModelInfo) => {
    const progress = progressMap[m.id]
    const isDownloading = isModelBusy(m)
    const desc = modelDesc(m.id, t)
    return (
      <div key={m.id} className="py-3 first:pt-0 last:pb-0">
        <div className="flex items-center gap-3">
          <div className="flex-1 min-w-0">
            <div className="truncate text-sm font-medium">{modelDisplayName(m.id, t, m.display_name)}</div>
            {formatSize(m.size_bytes) && (
              <div className="mt-0.5 text-xs tabular-nums text-muted-foreground">
                {formatSize(m.size_bytes)}
              </div>
            )}
            {desc && <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div>}
          </div>
          {m.installed ? (
            <Badge variant="success">{t.setInstalled}</Badge>
          ) : isDownloading ? (
            <Badge variant="warning">{t.setDownloading}</Badge>
          ) : (
            <Badge variant="secondary">{t.setNotInstalled}</Badge>
          )}
          {m.installed ? (
            <Button
              variant="ghost"
              size="sm"
              className="text-destructive hover:text-destructive hover:bg-destructive/10"
              onClick={() => handleDelete(m)}
            >
              {t.comDelete}
            </Button>
          ) : isDownloading ? (
            <Button variant="outline" size="sm" onClick={() => cancelDownload(m.id)}>
              {t.comCancel}
            </Button>
          ) : (
            <>
              {/* 不同模型可并行下载/导入；仅同一模型互斥（后端校验） */}
              <Button variant="outline" size="sm" onClick={() => importModel(m.id)}>
                {t.setImport}
              </Button>
              <Button size="sm" onClick={() => startDownload(m.id)}>
                {t.comDownload}
              </Button>
            </>
          )}
        </div>

        {/* 下载/导入进度条 */}
        {isDownloading && (
          <div className="mt-2">
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full bg-primary transition-all"
                style={{ width: `${Math.min(100, Math.max(0, progress?.percent ?? 0))}%` }}
              />
            </div>
            <div className="mt-1 text-xs tabular-nums text-muted-foreground">
              {progress ? stageText(progress, t) : t.setDownloadingPending}
            </div>
          </div>
        )}
      </div>
    )
  }

  // 按类别分组：ASR / 翻译 / 总结
  const groups: { key: ModelGroup; title: string; items: DownloadableModelInfo[] }[] = (
    [
      { key: 'asr', title: t.setGroupAsr },
      { key: 'translate', title: t.setGroupTranslate },
      { key: 'summary', title: t.setGroupSummary },
    ] as const
  )
    .map((g) => ({ ...g, items: models.filter((m) => modelGroup(m.id) === g.key) }))
    .filter((g) => g.items.length > 0)

  return (
    <SettingsSection title={t.setAsrModels}>
      {/* 模型目录 */}
      <div className="flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.setModelDir}</span>
        <Input readOnly value={modelsDir} placeholder={t.comLoading} />
        <span className="text-xs text-muted-foreground/70">{t.setModelDirHint}</span>
      </div>

      {/* 可下载模型列表（按类别分组） */}
      <div className="mt-4">
        {loading ? (
          <div className="py-3 text-xs text-muted-foreground">{t.comLoading}</div>
        ) : models.length === 0 ? (
          <div className="py-3 text-sm text-muted-foreground">{t.setNoModels}</div>
        ) : (
          groups.map((g) => (
            <div key={g.key} className="mb-4 last:mb-0">
              <h3 className="text-xs font-semibold text-muted-foreground mb-1">{g.title}</h3>
              <div className="divide-y divide-border/60">{g.items.map(renderModel)}</div>
            </div>
          ))
        )}
      </div>

      <div className="mt-4 rounded-md border border-primary/20 bg-primary/5 px-4 py-3 text-sm text-muted-foreground">
        {t.setModelNote1}
        <br />
        {t.setModelNote2}
      </div>
    </SettingsSection>
  )
}
