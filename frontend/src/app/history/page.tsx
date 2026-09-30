'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { Pencil, RefreshCw } from 'lucide-react'
import {
  apiGetRecordings,
  apiGetRecording,
  apiSaveRecordingTitle,
  apiDeleteRecording,
  startRetranscription,
  sherpaOnnxGetModels,
  cancelRetranscription,
} from '@/services/ipc'
import type { RecordingDetails, RecordingListItem } from '@/types'
import { useAppStore } from '@/state'
import { HistoryList } from '@/components/history/HistoryList'
import { RecordingDetail, type ResultTab } from '@/components/history/RecordingDetail'
import { ImportButton } from '@/components/history/ImportButton'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { AsrModelPicker, useAsrModelOptions, normalizeAsrModelName } from '@/components/models/AsrModelPicker'
import { useMessages } from '@/i18n/useMessages'
import { useHistoryFormat } from '@/components/history/format'
import { useRetranscriptionStore } from '@/stores/retranscriptionStore'

export default function HistoryPage() {
  const t = useMessages()
  const fmt = useHistoryFormat()
  const [recordings, setRecordings] = useState<RecordingListItem[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [details, setDetails] = useState<RecordingDetails | null>(null)
  const [loading, setLoading] = useState(false)
  const [editingTitle, setEditingTitle] = useState(false)
  const [titleDraft, setTitleDraft] = useState('')
  // 详情 tab 提升为受控：再次识别开始时自动切到离线 tab；选中录音若有离线结果也优先离线 tab
  const [detailTab, setDetailTab] = useState<ResultTab>('realtime')

  // 离线（重）识别状态在**全局 store**（见 stores/retranscriptionStore.ts）：
  // 切页面不会丢进度，后台任务照常进行、完成后照常有提示与刷新。
  const activeMeetingId = useRetranscriptionStore((s) => s.meetingId)
  const rProgress = useRetranscriptionStore((s) => s.progress)
  const rMessage = useRetranscriptionStore((s) => s.message)
  const rPartials = useRetranscriptionStore((s) => s.partials)
  const rStopping = useRetranscriptionStore((s) => s.stopping)
  const completionTick = useRetranscriptionStore((s) => s.completionTick)
  const lastResult = useRetranscriptionStore((s) => s.lastResult)
  const startRetrans = useRetranscriptionStore((s) => s.start)
  const setRetransStopping = useRetranscriptionStore((s) => s.setStopping)

  const setModels = useAppStore((s) => s.setModels)
  const [retransModel, setRetransModel] = useState('')
  // 用户是否手动选过重识别模型（手动选择后，详情刷新不再回跳覆盖——2026-09-17 修复：
  // 重识别完成后 loadDetails 触发 details 变化，旧逻辑会把用户选的远程模型重置回录音原引擎）
  const retransUserPickedRef = useRef(false)
  // 重识别 ASR 模型选择弹窗（与录音弹窗 TAB1 共用 AsrModelPicker；离线场景只列非流式模型）
  const [retransPickerOpen, setRetransPickerOpen] = useState(false)
  const [retransDraft, setRetransDraft] = useState('')
  // 离线重识别的识别语言（规范码；'auto' = 交给上游自动检测）。
  // 候选按所选模型过滤（见 AsrModelPicker 的语言行）。
  const [retransLanguage, setRetransLanguage] = useState('auto')
  const asrOptions = useAsrModelOptions('offline')
  // 录音停止跳转到本页时待自动选中的记录 id（从 store 消费一次后清空）
  const [pendingSelectId, setPendingSelectId] = useState<string | null>(null)

  const STATUS_BADGE_MAP: Record<string, { text: string; variant: 'success' | 'warning' | 'destructive' }> = {
    completed: { text: t.histStatusCompleted, variant: 'success' },
    done: { text: t.histStatusCompleted, variant: 'success' },
    pending: { text: t.histStatusPending, variant: 'warning' },
    processing: { text: t.histStatusProcessing, variant: 'warning' },
    failed: { text: t.histStatusFailed, variant: 'destructive' },
    error: { text: t.histStatusFailed, variant: 'destructive' },
  }

  const SOURCE_LABEL_MAP: Record<string, string> = {
    import: t.histSourceImport,
    record: t.histSourceRecord,
    recording: t.histSourceRecord,
  }

  const refresh = useCallback(async () => {
    try {
      setRecordings(await apiGetRecordings())
    } catch {
      toast.error(t.histLoadListFailed)
    }
  }, [t])

  useEffect(() => {
    refresh()
    sherpaOnnxGetModels().then(setModels).catch(() => {})
    // 消费「刚保存的录音」id：录音停止后 useRecorder 会跳转过来并带上该 id
    const latest = useAppStore.getState().latestRecordingId
    if (latest) {
      useAppStore.getState().setLatestRecordingId(null)
      setPendingSelectId(latest)
    }
  }, [refresh, setModels])

  // 列表加载完成后选中新记录
  useEffect(() => {
    if (pendingSelectId && recordings.some((r) => r.id === pendingSelectId)) {
      setSelectedId(pendingSelectId)
      setPendingSelectId(null)
    }
  }, [recordings, pendingSelectId])

  // 再次识别支持：本地已下载模型（sense-voice / x-asr）+ 远程 ASR（见 retranscription.rs）

  const loadDetails = useCallback(async (id: string) => {
    setLoading(true)
    try {
      const d = await apiGetRecording(id)
      setDetails(d)
      // 选中录音时：离线识别已有结果 → 优先展示离线 tab。
      // 目的（用户 2026-09-22 反馈）：引导用户用质量更好的离线结果去做会议总结。
      const hasOffline = (d.segments ?? []).some((s) => s.source === 'offline_asr')
      setDetailTab(hasOffline ? 'offline' : 'realtime')
    } catch {
      setDetails(null)
      toast.error(t.histLoadDetailFailed)
    } finally {
      setLoading(false)
    }
  }, [t])

  // 切换录音时重置编辑态并加载详情
  useEffect(() => {
    setEditingTitle(false)
    // 换了另一条录音：清除手动选择标记，允许按新录音的引擎重新解析默认模型
    retransUserPickedRef.current = false
    if (selectedId) {
      loadDetails(selectedId)
    } else {
      setDetails(null)
    }
  }, [selectedId, loadDetails])

  // 详情加载后：再次识别模型默认跟随该录音的 ASR 引擎或当前配置（校验仍可选）。
  // 用户手动选过模型后不再自动覆盖（避免重识别完成后回跳）。
  useEffect(() => {
    if (!details) return
    if (retransUserPickedRef.current) return
    const preferred = [details.asr_engine, useAppStore.getState().selectedModel]
      .map((n) => normalizeAsrModelName(n))
      .find((n) => n && asrOptions.isValid(n))
    setRetransModel(preferred || asrOptions.defaultChoice())
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [details])

  // 离线识别完成（可能发生在别的页面）：刷新列表；若完成的是当前录音，顺带刷新详情。
  // 用 ref 跳过首次挂载，避免无谓刷新。
  const lastTickRef = useRef(completionTick)
  useEffect(() => {
    if (completionTick === lastTickRef.current) return
    lastTickRef.current = completionTick
    refresh()
    if (lastResult?.meetingId && lastResult.meetingId === selectedId) {
      loadDetails(lastResult.meetingId)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [completionTick])

  const handleSaveTitle = async () => {
    setEditingTitle(false)
    if (!details) return
    const title = titleDraft.trim()
    if (!title || title === details.title) return
    try {
      await apiSaveRecordingTitle(details.id, title)
      setDetails({ ...details, title })
      refresh()
      toast.success(t.histTitleSaved)
    } catch {
      toast.error(t.histSaveTitleFailed)
    }
  }

  const handleDelete = async () => {
    if (!details) return
    if (!window.confirm(t.histDeleteConfirm.replace('{title}', details.title))) return
    try {
      await apiDeleteRecording(details.id)
      toast.success(t.histDeleted)
      setSelectedId(null)
      refresh()
    } catch {
      toast.error(t.histDeleteFailed)
    }
  }

  const handleRetranscribe = async (chosenModel?: string) => {
    if (!details) return
    if (!details.folder_path) {
      toast.error(t.histRetranscribeNoFolder)
      return
    }
    // 弹窗「确认」会把刚选好的模型直接传进来（setState 是异步的，不能依赖 retransModel 已更新）
    const modelChoice = chosenModel ?? retransModel
    if (!modelChoice) {
      toast.error(t.histRetranscribeNoModel)
      return
    }
    const normalized = normalizeAsrModelName(modelChoice)
    const isRemote = normalized === 'remote'
    // ⚠️ 远程必须传**真实模型 id**（2026-09-22 修复 item「选了豆包却显示 deep 模型」）：
    // 旧实现恒发占位名 qwen3-asr-remote，真实模型由后端读「全局离线选择」决定。
    // 而选择器的持久化是**异步**的（persist() 不 await）——用户刚换模型就点识别时，
    // 后端可能仍读到上一个模型，于是「选了豆包、实际跑 deepgram」。
    // 现在直接把当前选择器的真实 id 传下去，后端优先采用（retranscription.rs 已支持）。
    const remoteModelId = asrOptions.remoteAsr.value
    const model = isRemote ? remoteModelId : normalized
    if (isRemote && !model) {
      toast.error(t.histRetranscribeNoModel)
      return
    }
    const provider = isRemote ? 'remote' : normalized.startsWith('x-asr-') ? 'x-asr' : 'sherpaonnx'

    startRetrans(details.id, model, asrOptions.label(modelChoice))
    setDetailTab('offline')
    try {
      await startRetranscription(details.id, details.folder_path, model, provider, retransLanguage)
    } catch (e) {
      useRetranscriptionStore.getState().conclude()
      toast.error(t.histRetranscribeStartFailed, { description: String(e) })
    }
  }

  const retranscribing = activeMeetingId !== null && activeMeetingId === selectedId
  const otherRecordingBusy = activeMeetingId !== null && activeMeetingId !== selectedId

  const statusBadge = details?.status
    ? STATUS_BADGE_MAP[details.status] ?? { text: details.status, variant: 'secondary' as const }
    : null

  return (
    <div className="h-full flex flex-col gap-3 p-5 overflow-hidden">
      {/* 页头：左侧标题；右侧为选中录音的信息与操作 */}
      <header className="flex items-start justify-between gap-4 shrink-0 min-h-[52px]">
        <div className="shrink-0 pt-1">
          <h1 className="text-lg font-semibold tracking-tight">{t.histPageTitle}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t.histPageSubtitle}</p>
        </div>

        {details && !loading && (
          <div className="flex flex-col items-end gap-1.5 min-w-0">
            {/* 标题 + 元信息 */}
            <div className="flex items-center gap-2 flex-wrap justify-end">
              {editingTitle ? (
                <Input
                  autoFocus
                  className="h-8 w-[260px]"
                  value={titleDraft}
                  // 标题会变成导出文件名（Rust 侧 sanitize_filename 截到 80 字符），
                  // 这里先限制长度，避免用户以为能随便写超长标题
                  maxLength={80}
                  onChange={(e) => setTitleDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSaveTitle()
                    if (e.key === 'Escape') setEditingTitle(false)
                  }}
                  onBlur={handleSaveTitle}
                />
              ) : (
                <>
                  <h2 className="text-sm font-semibold truncate max-w-[320px]">{details.title}</h2>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 shrink-0"
                    title={t.histEditTitle}
                    onClick={() => {
                      setTitleDraft(details.title)
                      setEditingTitle(true)
                    }}
                  >
                    <Pencil className="h-3 w-3" />
                  </Button>
                </>
              )}
              <span className="text-xs text-muted-foreground">
                {fmt.formatCreatedAtFull(details.created_at)} · {t.histMetaDuration.replace('{duration}', fmt.formatDurationMs(details.duration_ms))} · {details.source ? SOURCE_LABEL_MAP[details.source] ?? details.source : t.histSourceUnknown}
                {/* 明确标注这是**录音时的实时引擎**，避免与下方离线识别模型混淆
                    （2026-09-22 用户反馈：选了豆包的离线模型，这里却显示 deepgram，
                     误以为「选错模型 / 跑了别的模型」） */}
                {details.asr_engine ? ` · ${t.histRealtimeEngineLabel}：${details.asr_engine}` : ''}
              </span>
              {statusBadge && <Badge variant={statusBadge.variant}>{statusBadge.text}</Badge>}
            </div>

            {/* 操作按钮行（导出在下方各 tab 栏内；打开文件夹在左侧列表每行） */}
            <div className="flex items-center flex-wrap gap-2 justify-end">
              {details.folder_path && (
                <>
                  {/* 离线识别：点击弹出模型选择弹窗（与录音弹窗 TAB1 共用 AsrModelPicker），
                      在弹窗里确认模型后才开始识别 */}
                  <Button
                    size="sm"
                    className="gap-1.5"
                    onClick={() => {
                      setRetransDraft(
                        asrOptions.isValid(retransModel) ? retransModel : asrOptions.defaultChoice()
                      )
                      setRetransPickerOpen(true)
                    }}
                    disabled={retranscribing || otherRecordingBusy}
                    title={otherRecordingBusy ? t.histRetranscribeOtherBusy : t.histRetranscribeModelTitle}
                  >
                    <RefreshCw className={retranscribing ? 'h-3.5 w-3.5 animate-spin' : 'h-3.5 w-3.5'} />
                    {retranscribing
                      ? `${t.histRetranscribing}${rProgress != null ? ` ${Math.round(rProgress)}%` : ''}`
                      : t.histRetranscribe}
                  </Button>
                  {/* 离线识别进行中：停止按钮（后端会立刻中断在途的远程请求） */}
                  {retranscribing && (
                    <Button
                      variant="destructive"
                      size="sm"
                      className="gap-1.5"
                      disabled={rStopping}
                      onClick={() => {
                        // 先进入「正在停止…」过渡态：后端会中断在途请求并发 cancelled 事件，
                        // 由全局事件复位（此前点击后界面毫无变化，用户以为按钮没用）
                        setRetransStopping(true)
                        cancelRetranscription().catch(() => setRetransStopping(false))
                      }}
                    >
                      {rStopping ? t.histRetranscribeStopping : t.histRetranscribeStop}
                    </Button>
                  )}
                </>
              )}
              <Button
                variant="ghost"
                size="sm"
                className="text-destructive hover:text-destructive hover:bg-destructive/10"
                onClick={handleDelete}
              >
                {t.comDelete}
              </Button>
            </div>
            {/* 实时进度消息：远程批量识别期间后端每 400ms 心跳一次（带「已等待 Ns」），
                百分比与秒数都在动——避免用户在云端等待时误以为程序卡死 */}
            {retranscribing && (
              <p className="text-[11px] text-muted-foreground/80 text-right">
                {rMessage ? (
                  <span className="tabular-nums">{rMessage}</span>
                ) : retransModel === 'remote' ? (
                  t.histRetranscribeRemoteSlow
                ) : null}
              </p>
            )}
            {/* 正在识别的是**另一条**录音：给出全局提示（切页面也能看到），
                否则用户回到本页会以为任务停了 */}
            {otherRecordingBusy && (
              <p className="text-[11px] text-primary/80 text-right">
                {t.histRetranscribeOtherBusy}
                {rProgress != null ? ` ${Math.round(rProgress)}%` : ''}
              </p>
            )}
          </div>
        )}
      </header>

      {/* 重新转写进度 */}
      {retranscribing && (
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted shrink-0">
          <div className="h-full bg-primary transition-all" style={{ width: `${rProgress ?? 0}%` }} />
        </div>
      )}

      {/* 左右双栏 */}
      <div className="flex-1 min-h-0 flex gap-4">
        <div className="w-[260px] shrink-0 min-h-0 flex flex-col">
          <HistoryList
            recordings={recordings}
            selectedId={selectedId}
            onSelect={setSelectedId}
            onMerged={(id) => {
              refresh()
              setSelectedId(id)
            }}
            footer={
              <ImportButton
                onImported={(id) => {
                  refresh()
                  setSelectedId(id)
                }}
              />
            }
          />
        </div>
        <div className="flex-1 min-w-0 min-h-0 flex flex-col">
          {selectedId ? (
            <RecordingDetail
              details={details}
              loading={loading}
              onChanged={() => {
                if (selectedId) loadDetails(selectedId)
              }}
              activeTab={detailTab}
              onTabChange={setDetailTab}
              liveSegments={retranscribing ? rPartials : undefined}
              retranscribing={retranscribing}
              retranscribeMessage={rMessage}
            />
          ) : (
            <div className="flex-1 min-h-0 flex flex-col items-center justify-center gap-1 rounded-lg border bg-card shadow-sm">
              <div className="text-sm font-medium text-muted-foreground">{t.histNoSelection}</div>
              <div className="text-xs text-muted-foreground/70">{t.histNoSelectionHint}</div>
            </div>
          )}
        </div>
      </div>

      {/* 重识别 ASR 模型选择弹窗（内容与录音弹窗 TAB1 一致） */}
      <Dialog open={retransPickerOpen} onOpenChange={setRetransPickerOpen}>
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle className="text-base">{t.histOfflineAsrModelLabel}</DialogTitle>
            <DialogDescription className="text-xs">
              {t.histRetranscribeModelTitle}
            </DialogDescription>
          </DialogHeader>
          <AsrModelPicker
            value={retransDraft}
            onChange={setRetransDraft}
            scene="offline"
            language={retransLanguage}
            onLanguageChange={setRetransLanguage}
          />
          <div className="flex items-center justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setRetransPickerOpen(false)}>
              {t.comCancel}
            </Button>
            <Button
              size="sm"
              className="min-w-[96px]"
              disabled={!retransDraft || !asrOptions.isValid(retransDraft)}
              onClick={() => {
                // 手动确认后标记，详情刷新（重识别完成等）不再覆盖该选择
                retransUserPickedRef.current = true
                const chosen = normalizeAsrModelName(retransDraft)
                setRetransModel(chosen)
                setRetransPickerOpen(false)
                // 确认即开始离线识别
                handleRetranscribe(chosen)
              }}
            >
              {t.comConfirm}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
