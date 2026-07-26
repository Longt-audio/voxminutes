'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Pencil, RefreshCw } from 'lucide-react'
import {
  apiGetRecordings,
  apiGetRecording,
  apiSaveRecordingTitle,
  apiDeleteRecording,
  startRetranscription,
  onRetranscriptionProgress,
  onRetranscriptionComplete,
  onRetranscriptionError,
  onRetranscriptionPartial,
  sherpaOnnxGetModels,
} from '@/services/ipc'
import type { RecordingDetails, RecordingListItem, RetranscriptionPartial } from '@/types'
import { useAppStore } from '@/state'
import { HistoryList } from '@/components/history/HistoryList'
import { RecordingDetail, type ResultTab } from '@/components/history/RecordingDetail'
import { ImportButton } from '@/components/history/ImportButton'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useMessages } from '@/i18n/useMessages'
import { useHistoryFormat } from '@/components/history/format'

const modelSelectCls =
  'h-8 w-[150px] rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50'

export default function HistoryPage() {
  const t = useMessages()
  const fmt = useHistoryFormat()
  const [recordings, setRecordings] = useState<RecordingListItem[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [details, setDetails] = useState<RecordingDetails | null>(null)
  const [loading, setLoading] = useState(false)
  const [editingTitle, setEditingTitle] = useState(false)
  const [titleDraft, setTitleDraft] = useState('')
  const [retranscribing, setRetranscribing] = useState(false)
  const [retransProgress, setRetransProgress] = useState<number | null>(null)
  // 详情 tab 提升为受控：再次识别开始时自动切到离线 tab
  const [detailTab, setDetailTab] = useState<ResultTab>('realtime')
  // 再次识别过程中按 chunk_index 排序的增量部分结果（仅当前选中录音）
  const [livePartials, setLivePartials] = useState<RetranscriptionPartial[]>([])

  const models = useAppStore((s) => s.models)
  const setModels = useAppStore((s) => s.setModels)
  const [retransModel, setRetransModel] = useState('')
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

  const installedModels = models.filter((m) => !m.hidden && !m.is_remote && m.status !== 'Missing')
  // 再次识别仅支持离线模型（SenseVoice 等），X-ASR 流式模型不支持离线文件识别
  const retransModels = installedModels.filter((m) => !m.name.startsWith('x-asr-'))

  const loadDetails = useCallback(async (id: string) => {
    setLoading(true)
    try {
      setDetails(await apiGetRecording(id))
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
    setRetranscribing(false)
    setRetransProgress(null)
    setLivePartials([])
    if (selectedId) {
      loadDetails(selectedId)
    } else {
      setDetails(null)
    }
  }, [selectedId, loadDetails])

  // 详情加载后：再次识别模型默认跟随该录音的 ASR 引擎或当前配置
  useEffect(() => {
    if (!details) return
    const preferred = [details.asr_engine, useAppStore.getState().selectedModel].find(
      (n): n is string => !!n && retransModels.some((m) => m.name === n)
    )
    setRetransModel(preferred || retransModels[0]?.name || '')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [details])

  // 重新转写事件监听（卸载时取消）
  const selectedIdRef = useRef(selectedId)
  selectedIdRef.current = selectedId
  const loadDetailsRef = useRef(loadDetails)
  loadDetailsRef.current = loadDetails
  useEffect(() => {
    let disposed = false
    let unlistens: UnlistenFn[] = []
    Promise.all([
      onRetranscriptionProgress((p) => {
        if (p.meeting_id === selectedIdRef.current) setRetransProgress(p.progress_percentage)
      }),
      onRetranscriptionPartial((p) => {
        if (p.meeting_id !== selectedIdRef.current) return
        // 同一 chunk 的最新部分结果覆盖旧的，保持按 chunk_index 升序
        setLivePartials((prev) => {
          const next = prev.filter((x) => x.chunk_index !== p.chunk_index)
          next.push(p)
          next.sort((a, b) => a.chunk_index - b.chunk_index)
          return next
        })
      }),
      onRetranscriptionComplete((r) => {
        toast.success(t.histRetranscribeDone.replace('{count}', String(r.segments_count)))
        refresh()
        if (r.meeting_id === selectedIdRef.current) {
          setRetranscribing(false)
          setRetransProgress(null)
          setLivePartials([])
          loadDetailsRef.current(r.meeting_id)
        } else {
          setRetranscribing(false)
          setRetransProgress(null)
        }
      }),
      onRetranscriptionError((e) => {
        if (e.meeting_id === selectedIdRef.current) {
          setRetranscribing(false)
          setRetransProgress(null)
          setLivePartials([])
        }
        toast.error(t.histRetranscribeFailed.replace('{error}', e.error))
      }),
    ]).then((fns) => {
      if (disposed) fns.forEach((f) => f())
      else unlistens = fns
    })
    return () => {
      disposed = true
      unlistens.forEach((f) => f())
    }
  }, [refresh, t])

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

  const handleRetranscribe = async () => {
    if (!details) return
    if (!details.folder_path) {
      toast.error(t.histRetranscribeNoFolder)
      return
    }
    if (!retransModel) {
      toast.error(t.histRetranscribeNoModel)
      return
    }
    setRetranscribing(true)
    setRetransProgress(0)
    setLivePartials([])
    setDetailTab('offline')
    try {
      await startRetranscription(
        details.id,
        details.folder_path,
        retransModel,
        retransModel.startsWith('x-asr-') ? 'x-asr' : 'sherpaonnx'
      )
    } catch {
      setRetranscribing(false)
      setRetransProgress(null)
      toast.error(t.histRetranscribeStartFailed)
    }
  }

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
                {details.asr_engine ? ` · ${details.asr_engine}` : ''}
              </span>
              {statusBadge && <Badge variant={statusBadge.variant}>{statusBadge.text}</Badge>}
            </div>

            {/* 操作按钮行（导出在下方各 tab 栏内；打开文件夹在左侧列表每行） */}
            <div className="flex items-center flex-wrap gap-2 justify-end">
              {details.folder_path && (
                <>
                  <select
                    className={modelSelectCls}
                    value={retransModel}
                    onChange={(e) => setRetransModel(e.target.value)}
                    disabled={retranscribing}
                    title={t.histRetranscribeModelTitle}
                  >
                    {retransModels.map((m) => (
                      <option key={m.name} value={m.name}>
                        {m.name === 'sense-voice' ? t.histModelSenseVoice : m.name}
                      </option>
                    ))}
                  </select>
                  <Button size="sm" className="gap-1.5" onClick={handleRetranscribe} disabled={retranscribing || !retransModel}>
                    <RefreshCw className={retranscribing ? 'h-3.5 w-3.5 animate-spin' : 'h-3.5 w-3.5'} />
                    {retranscribing
                      ? `${t.histRetranscribing}${retransProgress != null ? ` ${Math.round(retransProgress)}%` : ''}`
                      : t.histRetranscribe}
                  </Button>
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
          </div>
        )}
      </header>

      {/* 重新转写进度 */}
      {retranscribing && (
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted shrink-0">
          <div className="h-full bg-primary transition-all" style={{ width: `${retransProgress ?? 0}%` }} />
        </div>
      )}

      {/* 左右双栏 */}
      <div className="flex-1 min-h-0 flex gap-4">
        <div className="w-[260px] shrink-0 min-h-0 flex flex-col">
          <HistoryList
            recordings={recordings}
            selectedId={selectedId}
            onSelect={setSelectedId}
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
              liveSegments={livePartials}
            />
          ) : (
            <div className="flex-1 min-h-0 flex flex-col items-center justify-center gap-1 rounded-lg border bg-card shadow-sm">
              <div className="text-sm font-medium text-muted-foreground">{t.histNoSelection}</div>
              <div className="text-xs text-muted-foreground/70">{t.histNoSelectionHint}</div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
