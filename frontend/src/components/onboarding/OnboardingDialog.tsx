'use client'

import { useCallback, useEffect, useState } from 'react'
import {
  apiGetSettings,
  apiSaveSetting,
  getDownloadableModels,
  onFirstLaunchDetected,
  sherpaOnnxGetModelsDirectory,
  setModelsDirectoryCustom,
  getRemoteConfig,
  setRemoteConfig,
  setRemoteEnabled,
  checkRemoteAsrHealth,
} from '@/services/ipc'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import { Input } from '@/components/ui/input'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { useMessages } from '@/i18n/useMessages'
import { useModelDownload } from '@/hooks/useModelDownload'
import { LanguageSwitcher } from '@/components/LanguageSwitcher'
import { SourceLinksPanel } from '@/components/models/SourceLinks'
import { formatSize, stageText, modelGroup, modelDesc, modelDisplayName } from '@/lib/modelDisplay'

/** 设置页"重新打开新手指引"通过该窗口事件通知 AppShell 里的向导弹出 */
export const OPEN_ONBOARDING_EVENT = 'vox:open-onboarding'

/** 向导步骤：0 欢迎+远程配置(可跳过) / 1 本地模型 / 2 完成 */
const TOTAL_STEPS = 3

/** 各步骤的模型选项（多 id 表示一张卡对应多个模型，如 OPUS-MT 中英双向） */
const ASR_OPTIONS: string[][] = [['x-asr-480ms'], ['sense-voice']]
const TRANSLATE_OPTIONS: string[][] = [['opus-mt-zh-en', 'opus-mt-en-zh'], ['hy-mt2-1.8b-q4_k_m']]
const SUMMARY_OPTIONS: string[][] = [
  ['qwen2.5-3b-instruct-q4_k_m'],
  ['qwen3-4b-instruct-2507-q4_k_m'],
  ['gemma-3-4b-it-q4_k_m'],
]

export function OnboardingDialog() {
  const t = useMessages()
  const [open, setOpen] = useState(false)
  const [step, setStep] = useState(0)
  // 每个模型卡的"链接"面板展开状态
  const [linksOpen, setLinksOpen] = useState<Record<string, boolean>>({})
  const [modelsDir, setModelsDir] = useState('')
  // 远程服务设置（步骤 1）
  const [remoteUrl, setRemoteUrl] = useState('')
  const [remoteKey, setRemoteKey] = useState('')
  const [remoteEnable, setRemoteEnable] = useState(false)
  const [remoteHealth, setRemoteHealth] = useState<boolean | null>(null)
  const [remoteChecking, setRemoteChecking] = useState(false)
  const [remoteSaving, setRemoteSaving] = useState(false)
  const {
    models,
    progressMap,
    startDownload,
    cancelDownload,
    importModel,
    isModelBusy,
  } = useModelDownload()

  // 触发条件（满足其一，且 onboarding.completed 不存在）：
  // 1) 收到后端 first-launch-detected 事件；
  // 2) 挂载时检查：无 onboarding.completed 且没有任何已安装的 ASR 模型。
  // 另外监听设置页"重新打开新手指引"派发的窗口事件。
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined

    const maybeOpen = async () => {
      try {
        const settings = await apiGetSettings()
        if (settings['onboarding.completed']) return
        const list = await getDownloadableModels()
        const hasAsr = list.some((m) => modelGroup(m.id) === 'asr' && m.installed)
        if (!hasAsr) setOpen(true)
      } catch {
        // 后端不可用时静默，不打扰用户
      }
    }

    void maybeOpen()
    onFirstLaunchDetected(() => void maybeOpen()).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })

    const onReopen = () => {
      setStep(0)
      setOpen(true)
    }
    window.addEventListener(OPEN_ONBOARDING_EVENT, onReopen)

    return () => {
      disposed = true
      unlisten?.()
      window.removeEventListener(OPEN_ONBOARDING_EVENT, onReopen)
    }
  }, [])

  // 任何完成/跳过（含点 X 关闭）都写入 onboarding.completed，避免下次再弹
  const finish = useCallback(() => {
    apiSaveSetting('onboarding.completed', 'true').catch(() => {})
    setOpen(false)
    setStep(0)
  }, [])

  // 多模型卡（OPUS-MT 双向）同时开始下载（后端按模型互斥，不同模型可并行）
  const downloadAll = (ids: string[]) => {
    ids.forEach((id) => startDownload(id))
  }

  const asrInstalled = models.some((m) => modelGroup(m.id) === 'asr' && m.installed)
  const groupInstalled = (group: 'translate' | 'summary') =>
    models.some((m) => modelGroup(m.id) === group && m.installed)

  // 首次使用确认模型存放目录（可修改）
  useEffect(() => {
    sherpaOnnxGetModelsDirectory()
      .then(setModelsDir)
      .catch(() => {})
    getRemoteConfig()
      .then((cfg) => {
        setRemoteUrl(cfg.serverUrl || '')
        setRemoteKey(cfg.license || '')
      })
      .catch(() => {})
  }, [])

  const handleRemoteTest = async () => {
    const url = remoteUrl.trim()
    if (!url) {
      toast.error(t.onbRemoteNeedUrl)
      return
    }
    if (!remoteKey.trim()) {
      toast.error(t.onbRemoteNeedKey)
      return
    }
    setRemoteChecking(true)
    setRemoteHealth(null)
    try {
      setRemoteHealth(await checkRemoteAsrHealth(url))
    } catch {
      setRemoteHealth(false)
    } finally {
      setRemoteChecking(false)
    }
  }

  const handleRemoteNext = async () => {
    const url = remoteUrl.trim()
    const key = remoteKey.trim()
    // 填了地址+授权码才保存；否则视为跳过远程（直接下一步）
    if (url && key) {
      setRemoteSaving(true)
      try {
        await setRemoteConfig(url, key)
        if (remoteEnable) await setRemoteEnabled(true)
        toast.success(t.onbRemoteSaved)
      } catch (e) {
        toast.error(t.onbRemoteSaveFailed.replace('{error}', String(e)))
        setRemoteSaving(false)
        return
      }
      setRemoteSaving(false)
    }
    setStep(2)
  }

  const handleRemoteSkip = () => {
    setStep(2)
  }

  const handleChangeModelsDir = async () => {
    try {
      const selected = await openDialog({ directory: true, title: t.setModelDir })
      if (!selected) return
      const dir = typeof selected === 'string' ? selected : selected?.[0]
      if (!dir) return
      const effective = await setModelsDirectoryCustom(dir)
      setModelsDir(effective)
    } catch {
      // 静默
    }
  }

  // 模型选项卡：标题 + 描述 + 体积 + 下载/导入/取消 + 进度条
  const renderOption = (ids: string[], customTitle?: string) => {
    const infos = ids.map((id) => models.find((m) => m.id === id))
    if (infos.some((m) => !m)) return null // 后端未注册该模型时不展示
    const list = infos as NonNullable<(typeof infos)[number]>[]
    const installed = list.every((m) => m.installed)
    const busyInfo = list.find((m) => isModelBusy(m))
    const progress = busyInfo ? progressMap[busyInfo.id] : undefined
    const totalSize = list.reduce((sum, m) => sum + (m.size_bytes || 0), 0)
    const title = customTitle ?? modelDisplayName(ids[0], t, list[0].display_name)
    const desc = modelDesc(ids[0], t)

    return (
      <div key={ids.join('+')} className="rounded-md border border-border/60 px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="flex-1 min-w-0">
            <div className="truncate text-sm font-medium">{title}</div>
            {formatSize(totalSize) && (
              <div className="mt-0.5 text-xs tabular-nums text-muted-foreground">{formatSize(totalSize)}</div>
            )}
            {desc && <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div>}
          </div>
          {installed ? (
            <Badge variant="success">{t.setInstalled}</Badge>
          ) : busyInfo ? (
            <Button variant="outline" size="sm" onClick={() => cancelDownload(busyInfo.id)}>
              {t.comCancel}
            </Button>
          ) : (
            <>
              {/* 单模型卡支持从本地文件导入；不同模型可并行下载/导入 */}
              {ids.length === 1 && (
                <Button variant="outline" size="sm" onClick={() => importModel(ids[0])}>
                  {t.setImport}
                </Button>
              )}
              <Button size="sm" onClick={() => downloadAll(ids)}>
                {t.comDownload}
              </Button>
              {/* 展开/收起下载源直链（可复制到外部下载器） */}
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground"
                onClick={() => setLinksOpen((prev) => ({ ...prev, [ids[0]]: !prev[ids[0]] }))}
              >
                {t.setLinks}
              </Button>
            </>
          )}
        </div>
        {linksOpen[ids[0]] && !installed && (
          <SourceLinksPanel
            model={list[0]}
            disabled={!!busyInfo}
            onUseSource={(i) => startDownload(ids[0], i)}
          />
        )}
        {busyInfo && (
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

  const stepIndicator = t.onbStepIndicator
    .replace('{n}', String(step + 1))
    .replace('{total}', String(TOTAL_STEPS))

  return (
    <Dialog open={open} onOpenChange={(o) => (o ? setOpen(true) : finish())}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto custom-scrollbar">
        {/* 步骤 0：欢迎 + 远程配置（合并：第一页即可配置远程服务，可跳过） */}
        {step === 0 && (
          <>
            <div className="absolute right-12 top-4">
              <LanguageSwitcher />
            </div>
            <DialogHeader>
              <div className="mb-2 flex items-center gap-2.5">
                <div className="w-9 h-9 rounded-full bg-primary flex items-center justify-center shrink-0 overflow-hidden">
                  <svg width="18" height="13" viewBox="0 0 20 14" fill="none">
                    <rect x="0" y="6" width="3" height="8" rx="1" fill="white" />
                    <rect x="4.25" y="0" width="3" height="14" rx="1" fill="white" />
                    <rect x="8.5" y="4" width="3" height="10" rx="1" fill="white" />
                    <rect x="12.75" y="0" width="3" height="14" rx="1" fill="white" />
                    <rect x="17" y="5" width="3" height="9" rx="1" fill="white" />
                  </svg>
                </div>
                <DialogTitle>{t.onbWelcomeTitle}</DialogTitle>
              </div>
              <DialogDescription>{t.onbWelcomeSubtitle}</DialogDescription>
            </DialogHeader>
            <ul className="list-disc space-y-1.5 pl-5 text-sm text-muted-foreground">
              <li>{t.onbWelcomePoint1}</li>
              <li>{t.onbWelcomePoint2}</li>
              <li>{t.onbWelcomePoint3}</li>
            </ul>

            {/* 远程服务配置（可跳过；填了地址+授权码才保存） */}
            <div className="mt-3 rounded-md border border-border/60 px-4 py-3">
              <div className="text-sm font-medium">{t.onbRemoteTitle}</div>
              <p className="mt-0.5 text-xs text-muted-foreground">{t.onbRemoteDesc}</p>
              <div className="mt-3 flex flex-col gap-2.5">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={remoteEnable}
                    onChange={(e) => setRemoteEnable(e.target.checked)}
                    className="h-4 w-4"
                  />
                  <span>{t.onbRemoteEnable}</span>
                </label>
                <div className="flex flex-col gap-1.5">
                  <span className="text-xs text-muted-foreground">{t.onbRemoteUrl}</span>
                  <Input
                    value={remoteUrl}
                    onChange={(e) => {
                      setRemoteUrl(e.target.value)
                      setRemoteHealth(null)
                    }}
                    placeholder="http://127.0.0.1:8788"
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <span className="text-xs text-muted-foreground">{t.onbRemoteKey}</span>
                  <div className="flex items-center gap-2">
                    <Input
                      className="flex-1 min-w-0"
                      value={remoteKey}
                      onChange={(e) => setRemoteKey(e.target.value)}
                      placeholder="sk-…"
                      type="password"
                    />
                    <Button variant="outline" className="shrink-0" onClick={handleRemoteTest} disabled={remoteChecking}>
                      {remoteChecking ? t.onbRemoteTesting : t.onbRemoteTest}
                    </Button>
                    {remoteHealth !== null && (
                      <Badge variant={remoteHealth ? 'success' : 'destructive'}>
                        {remoteHealth ? t.setOnline : t.setOffline}
                      </Badge>
                    )}
                  </div>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-between pt-2">
              <span className="text-xs text-muted-foreground/70">{stepIndicator}</span>
              <div className="flex items-center gap-2">
                <Button variant="outline" onClick={handleRemoteSkip}>{t.onbRemoteSkipStep}</Button>
                <Button onClick={handleRemoteNext} disabled={remoteSaving}>{t.onbNext}</Button>
              </div>
            </div>
          </>
        )}

        {/* 步骤 1：本地模型（合并远程后的第二步） */}
        {step === 1 && (
          <>
            <DialogHeader>
              <DialogTitle>{t.onbStepLocalModelsTitle}</DialogTitle>
              <DialogDescription>{t.onbStepLocalModelsDesc}</DialogDescription>
            </DialogHeader>
            {/* 模型存放目录：首次使用可确认/修改 */}
            <div className="flex items-center gap-2 rounded-md border border-border/60 px-3 py-2">
              <span className="text-xs text-muted-foreground shrink-0">{t.setModelDir}</span>
              <span className="flex-1 min-w-0 truncate text-xs text-muted-foreground">{modelsDir || t.comLoading}</span>
              <Button variant="outline" size="sm" className="shrink-0" onClick={handleChangeModelsDir}>
                {t.setModelDirChange}
              </Button>
            </div>
            <div className="flex max-h-[48vh] flex-col gap-4 overflow-y-auto pr-1">
              <div>
                <h4 className="mb-2 text-sm font-medium">{t.setGroupAsr}</h4>
                <div className="flex flex-col gap-3">{ASR_OPTIONS.map((ids) => renderOption(ids))}</div>
              </div>
              <div>
                <h4 className="mb-2 text-sm font-medium">{t.setGroupTranslate}</h4>
                <div className="flex flex-col gap-3">
                  {renderOption(TRANSLATE_OPTIONS[0], t.onbOpusPairTitle)}
                  {renderOption(TRANSLATE_OPTIONS[1])}
                </div>
              </div>
              <div>
                <h4 className="mb-2 text-sm font-medium">{t.setGroupSummary}</h4>
                <div className="flex flex-col gap-3">{SUMMARY_OPTIONS.map((ids) => renderOption(ids))}</div>
              </div>
            </div>
            <div className="flex items-center justify-between pt-2">
              <Button variant="ghost" onClick={() => setStep(0)}>
                {t.onbBack}
              </Button>
              <div className="flex items-center gap-3">
                <Button onClick={() => setStep(2)}>
                  {t.onbNext}
                </Button>
              </div>
            </div>
          </>
        )}

        {/* 步骤 2：完成页 */}
        {step === 2 && (
          <>
            <DialogHeader>
              <DialogTitle>{t.onbStepDoneTitle}</DialogTitle>
              <DialogDescription>{t.onbDoneDesc}</DialogDescription>
            </DialogHeader>
            <div className="flex items-center justify-between pt-2">
              <span className="text-xs text-muted-foreground/70">{stepIndicator}</span>
              <Button onClick={finish}>{t.onbFinish}</Button>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
