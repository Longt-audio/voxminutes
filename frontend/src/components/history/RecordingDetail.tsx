'use client'

import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Sparkles } from 'lucide-react'
import { apiUpdateSegmentText, summaryLoad } from '@/services/ipc'
import type { RecordingDetails, RetranscriptionPartial } from '@/types'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { cn } from '@/lib/utils'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { loadPrompts, type SummaryPromptPreset } from '@/lib/summaryPrompts'
import type { SummaryGenerateParams } from '@/hooks/useSummaryGeneration'
import { ExportMenu } from './ExportMenu'
import { SummaryDialog } from './SummaryDialog'
import { SummaryResultDialog } from './SummaryResultDialog'
import { formatSegmentTime } from './format'

export type ResultTab = 'realtime' | 'offline'

interface RecordingDetailProps {
  details: RecordingDetails | null
  loading: boolean
  /** 片段保存后刷新详情 */
  onChanged: () => void
  /** 受控 tab（再次识别开始时父组件切到离线 tab）；不传则内部维护 */
  activeTab?: ResultTab
  onTabChange?: (tab: ResultTab) => void
  /** 再次识别过程中各块的增量结果（离线 tab 实时展示） */
  liveSegments?: RetranscriptionPartial[]
}

/** 右栏录音详情：实时识别/离线识别双 tab 转录片段列表（双击行内编辑）。标题/元信息/操作在页头。 */
export function RecordingDetail({
  details,
  loading,
  onChanged,
  activeTab: controlledTab,
  onTabChange,
  liveSegments,
}: RecordingDetailProps) {
  const t = useMessages()
  const language = useLanguageStore((s) => s.language)
  const [innerTab, setInnerTab] = useState<ResultTab>('realtime')
  const activeTab = controlledTab ?? innerTab
  const [editingSegmentId, setEditingSegmentId] = useState<string | null>(null)
  const [segmentDraft, setSegmentDraft] = useState('')
  const [summaryOpen, setSummaryOpen] = useState(false)
  const [resultOpen, setResultOpen] = useState(false)
  const [resultGenerate, setResultGenerate] = useState<SummaryGenerateParams | null>(null)
  const [hasSavedSummary, setHasSavedSummary] = useState(false)
  const [prompts, setPrompts] = useState<SummaryPromptPreset[]>([])
  const [summaryPromptId, setSummaryPromptId] = useState('default')

  // 实时结果：录音时保存的段落（source 为 'Audio'/空）；离线结果：再次优化识别的段落
  const realtimeSegments = details?.segments.filter((s) => s.source !== 'offline_asr') ?? []
  const offlineSegments = details?.segments.filter((s) => s.source === 'offline_asr') ?? []
  const segments = activeTab === 'realtime' ? realtimeSegments : offlineSegments

  // 总结模板列表（行内紧凑选择器与对话框共享同一个选中项）
  useEffect(() => {
    setPrompts(loadPrompts(language))
  }, [language])

  // 当前 tab 是否已有保存过的会议纪要
  const refreshSavedSummary = useCallback(() => {
    if (!details) return
    summaryLoad(details.id, activeTab)
      .then((saved) => setHasSavedSummary(!!saved))
      .catch(() => setHasSavedSummary(false))
  }, [details, activeTab])

  useEffect(() => {
    refreshSavedSummary()
  }, [refreshSavedSummary])

  const summaryTranscript = segments
    .map((s) => '[' + formatSegmentTime(s.start_ms) + '] ' + s.text)
    .join('\n')

  const handleSaveSegment = async () => {
    if (!editingSegmentId) return
    const text = segmentDraft.trim()
    try {
      await apiUpdateSegmentText(editingSegmentId, text)
      setEditingSegmentId(null)
      toast.success(t.histSegmentSaved)
      onChanged()
    } catch {
      toast.error(t.histSegmentSaveFailed)
    }
  }

  const switchTab = (tab: ResultTab) => {
    if (onTabChange) onTabChange(tab)
    else setInnerTab(tab)
    setEditingSegmentId(null)
  }

  const tabCls = (tab: ResultTab) =>
    cn(
      'px-1 pb-2 text-xs font-medium border-b-2 -mb-px transition-colors',
      activeTab === tab
        ? 'border-primary text-primary'
        : 'border-transparent text-muted-foreground hover:text-foreground'
    )

  return (
    <div className="flex-1 min-h-0 flex flex-col rounded-lg border bg-card shadow-sm p-3">
      {loading ? (
        <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
          {t.comLoading}
        </div>
      ) : !details ? (
        <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
          {t.histLoadDetailFailed}
        </div>
      ) : (
        <>
          {/* tab 栏 + 当前 tab 的导出按钮（导出内容与 tab 对应） */}
          <div className="shrink-0 flex items-end justify-between gap-2 border-b border-border/60 mb-1">
            <div className="flex gap-4">
              <button className={tabCls('realtime')} onClick={() => switchTab('realtime')}>
                {t.histTabRealtime}
              </button>
              <button className={tabCls('offline')} onClick={() => switchTab('offline')}>
                {t.histTabOffline}
              </button>
            </div>
            <div className="pb-1.5">
              <ExportMenu
                key={activeTab}
                recordingId={details.id}
                source={activeTab === 'realtime' ? 'realtime' : 'offline_asr'}
                disabled={segments.length === 0}
              />
            </div>
          </div>

          {/* 会议总结行：模板选择 + 生成入口 + 查看已保存 */}
          <div className="shrink-0 flex items-center gap-2 py-2">
            <Select value={summaryPromptId} onValueChange={setSummaryPromptId}>
              <SelectTrigger className="h-8 w-[180px] text-xs" title={t.sumPromptPreset}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {prompts.map((p) => (
                  <SelectItem key={p.id} value={p.id} className="text-xs">
                    {p.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              size="sm"
              className="gap-1.5"
              disabled={segments.length === 0}
              title={segments.length === 0 ? t.sumNoTranscript : undefined}
              onClick={() => setSummaryOpen(true)}
            >
              <Sparkles className="h-3.5 w-3.5" />
              {t.sumButton}
            </Button>
            {hasSavedSummary && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setResultGenerate(null)
                  setResultOpen(true)
                }}
              >
                {t.sumViewSaved}
              </Button>
            )}
          </div>

          <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar">
            {segments.length === 0 ? (
              activeTab === 'offline' && liveSegments && liveSegments.length > 0 ? (
                // 再次识别进行中：按块增量展示部分结果（不可编辑，颜色区别于正式结果）
                <div>
                  <div className="px-2 py-1.5 text-xs text-muted-foreground">
                    {t.histRetranscribing} {liveSegments.length}/
                    {liveSegments[liveSegments.length - 1].chunks_total}
                  </div>
                  {liveSegments.map((seg) => (
                    <div key={seg.chunk_index} className="flex items-start gap-3 px-2 py-1.5 rounded-md">
                      <span className="min-w-[50px] shrink-0 mt-0.5 text-right text-xs tabular-nums text-muted-foreground/60">
                        {formatSegmentTime(seg.start_ms)}
                      </span>
                      <p className="flex-1 min-w-0 text-sm leading-relaxed text-muted-foreground/70">{seg.text}</p>
                    </div>
                  ))}
                </div>
              ) : activeTab === 'offline' ? (
                <div className="mt-1 rounded-md border border-primary/20 bg-primary/5 px-4 py-3">
                  <div className="text-sm font-medium">{t.histOfflineEmptyTitle}</div>
                  <div className="mt-0.5 text-sm text-muted-foreground">
                    {t.histOfflineEmptyHint}
                  </div>
                </div>
              ) : details.status === 'pending' ? (
                <div className="mt-1 rounded-md border border-primary/20 bg-primary/5 px-4 py-3">
                  <div className="text-sm font-medium">{t.histPendingTitle}</div>
                  <div className="mt-0.5 text-sm text-muted-foreground">
                    {t.histPendingHint}
                  </div>
                </div>
              ) : (
                <div className="py-8 text-center text-sm text-muted-foreground">{t.histTranscriptEmpty}</div>
              )
            ) : (
              segments.map((seg) =>
                editingSegmentId === seg.id ? (
                  <div key={seg.id} className="px-2 py-1.5">
                    <textarea
                      autoFocus
                      className="w-full min-h-[64px] rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm focus:outline-none focus:ring-1 focus:ring-ring"
                      value={segmentDraft}
                      onChange={(e) => setSegmentDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Escape') setEditingSegmentId(null)
                      }}
                    />
                    <div className="mt-2 flex justify-end gap-2">
                      <Button variant="ghost" size="sm" onClick={() => setEditingSegmentId(null)}>
                        {t.comCancel}
                      </Button>
                      <Button size="sm" onClick={handleSaveSegment}>
                        {t.comSave}
                      </Button>
                    </div>
                  </div>
                ) : (
                  <div
                    key={seg.id}
                    className="flex items-start gap-3 px-2 py-1.5 rounded-md hover:bg-muted/60"
                    title={t.histDoubleClickEdit}
                    onDoubleClick={() => {
                      setEditingSegmentId(seg.id)
                      setSegmentDraft(seg.text)
                    }}
                  >
                    <span className="min-w-[50px] shrink-0 mt-0.5 text-right text-xs tabular-nums text-muted-foreground/60">
                      {formatSegmentTime(seg.start_ms)}
                    </span>
                    <p className="flex-1 min-w-0 text-sm leading-relaxed text-foreground/80">{seg.text}</p>
                  </div>
                )
              )
            )}
          </div>

          <SummaryDialog
            open={summaryOpen}
            onOpenChange={setSummaryOpen}
            transcript={summaryTranscript}
            onGenerate={(params) => {
              setResultGenerate(params)
              setResultOpen(true)
            }}
            promptId={summaryPromptId}
            onPromptChange={setSummaryPromptId}
          />
          <SummaryResultDialog
            open={resultOpen}
            onOpenChange={setResultOpen}
            recordingId={details.id}
            source={activeTab}
            generate={resultGenerate}
            onSaved={refreshSavedSummary}
          />
        </>
      )}
    </div>
  )
}
