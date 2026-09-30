'use client'

import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Check, Copy, Sparkles } from 'lucide-react'
import { apiUpdateSegmentText, getSpeakerNames, setSpeakerName, summaryLoad, getOfflineRecognitionInfo } from '@/services/ipc'
import type { RecordingDetails, RetranscriptionPartial } from '@/types'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useMessages } from '@/i18n/useMessages'
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
  /** 再次识别进行中（离线 tab 显示等待态，而不是「暂无结果」） */
  retranscribing?: boolean
  /** 后端进度消息（远程批量识别期间带「已等待 Ns」） */
  retranscribeMessage?: string
}

/** 右栏录音详情：实时识别/离线识别双 tab 转录片段列表（双击行内编辑）。标题/元信息/操作在页头。 */
export function RecordingDetail({
  details,
  loading,
  onChanged,
  activeTab: controlledTab,
  onTabChange,
  liveSegments,
  retranscribing,
  retranscribeMessage,
}: RecordingDetailProps) {
  const t = useMessages()
  const [innerTab, setInnerTab] = useState<ResultTab>('realtime')
  const activeTab = controlledTab ?? innerTab
  const [editingSegmentId, setEditingSegmentId] = useState<string | null>(null)
  const [segmentDraft, setSegmentDraft] = useState('')
  // 说话人命名表（离线识别说话人分离）：{speakerId: 自定义名}
  const [speakerNames, setSpeakerNames] = useState<Record<string, string>>({})
  const [renamingSpeakerId, setRenamingSpeakerId] = useState<string | null>(null)
  const [speakerDraft, setSpeakerDraft] = useState('')
  // 离线识别实际使用的模型（metadata.json；显示在离线 tab 顶部，避免与所选模型混淆）
  const [retranscribedModel, setRetranscribedModel] = useState('')
  // 离线识别摘要：音频时长 + 识别耗时（metadata.json，由 retranscription 写入）
  const [offlineDurationSec, setOfflineDurationSec] = useState(0)
  const [offlineElapsedSec, setOfflineElapsedSec] = useState(0)
  // 上游告警（内容风控部分拦截 / 档位降级 / 部分分片失败）：持久展示，避免用户
  // 把「结果不完整」当成软件故障（toast 一闪而过，看不到第二次）
  const [offlineWarnings, setOfflineWarnings] = useState<string[]>([])
  // 「复制文本」按钮的已复制态（1.5s 后复位）
  const [copied, setCopied] = useState(false)
  const [summaryOpen, setSummaryOpen] = useState(false)
  const [resultOpen, setResultOpen] = useState(false)
  const [resultGenerate, setResultGenerate] = useState<SummaryGenerateParams | null>(null)
  const [hasSavedSummary, setHasSavedSummary] = useState(false)
  // 总结模板选中项（与 SummaryDialog 共享；主页面的行内选择器已移除，选择在弹窗内进行）
  const [summaryPromptId, setSummaryPromptId] = useState('simple')

  // 实时结果：录音时保存的段落（source 为 'Audio'/空）；离线结果：再次优化识别的段落
  const realtimeSegments = details?.segments.filter((s) => s.source !== 'offline_asr') ?? []
  const offlineSegments = details?.segments.filter((s) => s.source === 'offline_asr') ?? []
  const segments = activeTab === 'realtime' ? realtimeSegments : offlineSegments

  // 读会议文件夹里的说话人命名表（speakers.json）+ 离线识别摘要
  //
  // ⚠️ 2026-09-22 修复：这些值只依赖 folder_path，而 folder_path 在同一条录音上
  // **永远不变** —— 重识别完成后 metadata.json 已更新为「豆包」，但 effect 不会重跑，
  // 离线 tab 顶部一直显示上一次的模型（用户反馈：「选了豆包，显示的却是 deep 模型」）。
  // 现在依赖整个 details（每次 loadDetails 都会换新对象），重识别完成后立即刷新。
  useEffect(() => {
    let cancelled = false
    const folder = details?.folder_path
    if (!folder) {
      setSpeakerNames({})
      setRetranscribedModel('')
      setOfflineDurationSec(0)
      setOfflineElapsedSec(0)
      setOfflineWarnings([])
      return
    }
    getSpeakerNames(folder)
      .then((names) => {
        if (!cancelled) setSpeakerNames(names ?? {})
      })
      .catch(() => {
        if (!cancelled) setSpeakerNames({})
      })
    getOfflineRecognitionInfo(folder)
      .then((info) => {
        if (cancelled) return
        setRetranscribedModel(info?.model ?? '')
        setOfflineDurationSec(info?.duration_seconds ?? 0)
        setOfflineElapsedSec(info?.elapsed_seconds ?? 0)
        setOfflineWarnings(Array.isArray(info?.warnings) ? info!.warnings! : [])
      })
      .catch(() => {
        if (cancelled) return
        setRetranscribedModel('')
        setOfflineDurationSec(0)
        setOfflineElapsedSec(0)
        setOfflineWarnings([])
      })
    return () => {
      cancelled = true
    }
  }, [details])

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

  // 离线模型 id → 显示名（与网关目录显示名对齐；未知 id 原样显示）
  const OFFLINE_MODEL_LABELS: Record<string, string> = {
    'qwen-audio-3.0-asr-flash-filetrans': 'Qwen-ASR',
    'qwen-audio-3.0-asr-flash': 'Qwen-ASR',
    'doubao-asr-file-2.0': 'Doubao-ASR',
    'mimo-v2.5-asr': 'MIMO-ASR',
    'deepgram-nova-3': 'Deepgram-ASR',
    'gpt-4o-mini-transcribe': 'GPT-4o mini Transcribe',
    'sense-voice': `SenseVoice${t.mdLocalSuffix}`,
    'x-asr-480ms': `X-ASR${t.mdLocalSuffix}`,
  }
  const retranscribedModelLabel = retranscribedModel
    ? (OFFLINE_MODEL_LABELS[retranscribedModel] ?? retranscribedModel)
    : ''

  /** mm:ss（音频时长展示） */
  const formatClock = (sec: number) => {
    if (!sec || sec <= 0) return ''
    const total = Math.round(sec)
    const m = Math.floor(total / 60)
    const s = total % 60
    return `${m}:${String(s).padStart(2, '0')}`
  }

  // 说话人显示名：自定义名优先，回退「说话人N」
  const speakerDisplay = useCallback(
    (id: string | null | undefined) => {
      if (!id) return ''
      return speakerNames[id] || t.histSpeakerDefault.replace('{n}', id)
    },
    [speakerNames, t]
  )
  // 按首次出现顺序收集离线片段的说话人 id
  const speakerIds: string[] = []
  for (const s of segments) {
    if (s.speaker && !speakerIds.includes(s.speaker)) speakerIds.push(s.speaker)
  }

  // 保存说话人重命名
  const handleSpeakerRename = async (speakerId: string) => {
    const name = speakerDraft.trim()
    const folder = details?.folder_path
    if (!folder) return
    try {
      await setSpeakerName(folder, speakerId, name)
      setSpeakerNames((prev) => {
        const next = { ...prev }
        if (name) next[speakerId] = name
        else delete next[speakerId]
        return next
      })
      setRenamingSpeakerId(null)
    } catch (e) {
      toast.error(String(e))
    }
  }

  const summaryTranscript = segments
    .map((s) => {
      const sp = speakerDisplay(s.speaker)
      const head = sp ? sp + ' ' : ''
      return head + '[' + formatSegmentTime(s.start_ms) + '] ' + s.text
    })
    .join('\n')

  /** 复制当前 tab 的语音识别结果（纯文本，逐段一行，带说话人前缀；有译文的段译文紧随其后）。
   *  会话总结按钮旁的操作入口（用户 2026-09-22 需求）。 */
  const handleCopyTranscript = async () => {
    const text = segments
      .map((s) => {
        const sp = speakerDisplay(s.speaker)
        const line = (sp ? sp + '：' : '') + s.text
        return s.translation ? line + '\n' + s.translation : line
      })
      .join('\n')
    if (!text.trim()) return
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      toast.success(t.histCopyTranscriptDone)
      window.setTimeout(() => setCopied(false), 1500)
    } catch {
      toast.error(t.histCopyTranscriptFailed)
    }
  }

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

          {/* 离线 tab：显示实际用于离线识别的模型 + 音频时长 + 识别耗时（metadata.json 记录） */}
          {activeTab === 'offline' && (retranscribedModelLabel || offlineDurationSec > 0) && (
            <p className="shrink-0 px-1 pb-1 text-[11px] text-muted-foreground">
              {retranscribedModelLabel
                ? `${t.histOfflineAsrModelLabel}：${retranscribedModelLabel}`
                : ''}
              {offlineDurationSec > 0
                ? `${retranscribedModelLabel ? ' · ' : ''}${t.histAudioDurationLabel}：${formatClock(offlineDurationSec)}`
                : ''}
              {offlineElapsedSec > 0 ? ` · ${t.histAsrElapsedLabel}：${offlineElapsedSec.toFixed(1)}s` : ''}
            </p>
          )}

          {/* 离线 tab：上游告警（内容风控部分拦截 / 档位降级 / 部分分片失败）。
              明确写「不是软件故障」并给出建议，避免用户误会。 */}
          {activeTab === 'offline' && offlineWarnings.length > 0 && (
            <div className="shrink-0 mx-1 mb-1 rounded-md border border-amber-300/70 bg-amber-50 px-3 py-2">
              <p className="text-[11px] font-medium text-amber-900">⚠️ {t.histOfflineWarningsTitle}</p>
              <ul className="mt-0.5 list-disc pl-4 space-y-0.5">
                {offlineWarnings.map((w, i) => (
                  <li key={i} className="text-[11px] leading-relaxed text-amber-900/90">
                    {w}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* 会议总结行：生成入口 + 查看已保存（模板选择在总结弹窗内进行） */}
          <div className="shrink-0 flex items-center gap-2 py-2">
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
            {/* 复制当前 tab 的语音识别结果（与总结按钮并列，实时/离线两个 tab 都有） */}
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              disabled={segments.length === 0}
              title={t.histCopyTranscriptHint}
              onClick={handleCopyTranscript}
            >
              {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              {copied ? t.histCopied : t.histCopyTranscript}
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
            {/* 说话人图例（离线识别开启说话人分离时显示；点击名字可重命名） */}
            {activeTab === 'offline' && speakerIds.length > 0 && (
              <div className="flex flex-wrap items-center gap-1.5 px-2 pb-2">
                <span className="text-xs text-muted-foreground">{t.histSpeakerLegend}：</span>
                {speakerIds.map((id) =>
                  renamingSpeakerId === id ? (
                    <span key={id} className="inline-flex items-center gap-1 rounded-full border border-primary/30 bg-primary/5 px-2 py-0.5">
                      <input
                        autoFocus
                        className="w-20 bg-transparent text-xs outline-none"
                        value={speakerDraft}
                        onChange={(e) => setSpeakerDraft(e.target.value)}
                        onBlur={() => handleSpeakerRename(id)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleSpeakerRename(id)
                          if (e.key === 'Escape') setRenamingSpeakerId(null)
                        }}
                      />
                    </span>
                  ) : (
                    <button
                      key={id}
                      type="button"
                      title={t.histSpeakerRename}
                      className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/50 px-2 py-0.5 text-xs text-foreground/80 hover:border-primary/40 hover:bg-primary/5 transition-colors"
                      onClick={() => {
                        setRenamingSpeakerId(id)
                        setSpeakerDraft(speakerNames[id] ?? '')
                      }}
                    >
                      {speakerDisplay(id)}
                    </button>
                  )
                )}
              </div>
            )}
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
              ) : activeTab === 'offline' && retranscribing ? (
                // 识别进行中但还没有任何 chunk 落定（远程批量模型把整段音频合成
                // 1 个 chunk，中途不会有增量结果）：显示等待态 + 已等待秒数，
                // 而不是误导性的「暂无离线识别结果」
                <div className="mt-1 rounded-md border border-primary/20 bg-primary/5 px-4 py-3">
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <span className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-primary/30 border-t-primary" />
                    {t.histRetranscribing}
                  </div>
                  <div className="mt-0.5 text-sm tabular-nums text-muted-foreground">
                    {retranscribeMessage || t.histRetranscribeRemoteSlow}
                  </div>
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
              segments.map((seg, index) =>
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
                    <div className="flex-1 min-w-0">
                      {/* 说话人变化时显示名字徽标 */}
                      {seg.speaker && (index === 0 || segments[index - 1]?.speaker !== seg.speaker) && (
                        <span className="mb-0.5 inline-block rounded bg-primary/10 px-1.5 py-px text-[11px] font-medium text-primary">
                          {speakerDisplay(seg.speaker)}
                        </span>
                      )}
                      <p className="text-sm leading-relaxed text-foreground/80">{seg.text}</p>
                      {/* 实时内嵌翻译的定稿译文（有才显示，小号弱化色） */}
                      {seg.translation && (
                        <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{seg.translation}</p>
                      )}
                    </div>
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
            title={details.title}
            source={activeTab}
            generate={resultGenerate}
            onSaved={refreshSavedSummary}
          />
        </>
      )}
    </div>
  )
}
