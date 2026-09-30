'use client'

import { useEffect, useRef } from 'react'
import { AlertTriangle, Mic } from 'lucide-react'
import { useAppStore } from '@/state'
import { useMessages } from '@/i18n/useMessages'
import type { TranscriptSegment } from '@/types'

function formatTime(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds))
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

/** 实时转录文本面板：自动滚动到底部，partial 斜体显示 */
export function TranscriptPanel() {
  const transcripts = useAppStore((s) => s.transcripts)
  const translations = useAppStore((s) => s.translations)
  const partialTranslations = useAppStore((s) => s.partialTranslations)
  const translateEnabled = useAppStore((s) => s.translateEnabled)
  const vadSpeaking = useAppStore((s) => s.vadSpeaking)
  const paragraphStarts = useAppStore((s) => s.paragraphStarts)
  const isRecording = useAppStore((s) => s.isRecording)
  const asrStopReason = useAppStore((s) => s.asrStopReason)
  const t = useMessages()

  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = scrollRef.current
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: 'instant' })
  }, [transcripts, translations, partialTranslations, vadSpeaking])

  // VAD 已检测到人声、但当前这句话尚未断句输出文字时的「识别中」提示
  const recognizing = (
    <div className="px-2 py-1.5 flex items-center gap-2 text-muted-foreground">
      <span className="relative flex h-2 w-2">
        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75" />
        <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-500" />
      </span>
      <span className="text-sm italic">{t.recRecognizing}</span>
    </div>
  )

  // 单元内译文（最终版优先，否则草稿），仅翻译开启时显示
  const renderTranslation = (seqId: number, partial: boolean) => {
    if (!translateEnabled) return null
    const finalText = translations.get(seqId)
    if (finalText != null) {
      return partial ? (
        <p className="text-xs leading-relaxed italic text-blue-500/80 mt-0.5 pl-2 border-l-2 border-blue-200">
          {finalText}
        </p>
      ) : (
        <p className="text-xs leading-relaxed text-blue-600 mt-0.5 pl-2 border-l-2 border-blue-300">{finalText}</p>
      )
    }
    const draft = partialTranslations.get(seqId)
    if (!draft) return null
    return partial ? (
      <p className="text-xs leading-relaxed italic text-blue-400/60 mt-0.5 pl-2 border-l-2 border-blue-200/60">
        {draft}
      </p>
    ) : (
      <p className="text-xs leading-relaxed italic text-blue-400/70 mt-0.5 pl-2 border-l-2 border-blue-200">
        {draft}
      </p>
    )
  }

  // 按「连续的相同 paragraph_id」分组；paragraph_id 为空的段每条自成一组（保持原卡片渲染）
  type Group = { kind: 'single'; seg: TranscriptSegment } | { kind: 'para'; pid: number; segs: TranscriptSegment[] }
  const groups: Group[] = []
  for (const seg of transcripts) {
    if (seg.paragraph_id == null) {
      groups.push({ kind: 'single', seg })
      continue
    }
    const last = groups[groups.length - 1]
    if (last && last.kind === 'para' && last.pid === seg.paragraph_id) {
      last.segs.push(seg)
    } else {
      groups.push({ kind: 'para', pid: seg.paragraph_id, segs: [seg] })
    }
  }

  return (
    <div className="flex-1 min-h-0 flex flex-col">
      {/* 识别已停止（2026-09-29）：远程流式 ASR 因积分不足/服务不可用终止后，
          这里常驻提示一次（此前只有一条 5 秒 toast，用户完全无感）。
          纯提示——录音与落盘不受影响，停止后仍可用历史页的离线识别补齐。 */}
      {isRecording && asrStopReason && (
        <div className="shrink-0 flex items-start gap-1.5 border-b border-amber-300/60 bg-amber-50 px-3 py-1.5 text-[11px] leading-relaxed text-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>
            {asrStopReason === 'credits'
              ? t.recAsrStoppedCredits
              : asrStopReason === 'config' || asrStopReason === 'unavailable'
                ? t.recAsrStoppedConfig
                : t.recAsrStopped}
          </span>
        </div>
      )}
      <div
        ref={scrollRef}
        className="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-2"
        style={{ overflowAnchor: 'none' }}
      >
        {transcripts.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center gap-2 text-center">
            {vadSpeaking ? (
              <>
                <span className="relative flex h-3 w-3">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-3 w-3 bg-blue-500" />
                </span>
                <div className="text-sm font-medium text-muted-foreground">{t.recRecognizing}</div>
                <div className="text-xs text-muted-foreground/60">{t.recRecognizingHint}</div>
              </>
            ) : (
              <>
                <Mic className="h-5 w-5 text-muted-foreground" />
                <div className="text-sm font-medium text-muted-foreground">{t.emptyTitle}</div>
                <div className="text-xs text-muted-foreground/60">{t.emptyHint}</div>
              </>
            )}
          </div>
        ) : (
          <>
            {groups.map((group) => {
              if (group.kind === 'para') {
                const startMs = paragraphStarts[group.pid]
                return (
                  <div key={`para-${group.pid}`} className="px-2 py-1.5 rounded-md hover:bg-muted/60">
                    {/* 段首墙钟时间（paragraphStarts 缺失时退化为首单元音频时间） */}
                    <div className="text-xs tabular-nums text-muted-foreground/60">
                      {startMs != null
                        ? new Date(startMs).toLocaleTimeString()
                        : formatTime(group.segs[0].audio_start_time)}
                    </div>
                    {/* 段内句级交错：每句原文 + 其译文，句间紧凑 */}
                    {group.segs.map((seg) => (
                      <div key={seg.id} className="mt-0.5 first:mt-1">
                        {seg.is_partial ? (
                          <p className="text-sm leading-relaxed italic text-muted-foreground">{seg.text}</p>
                        ) : (
                          <p className="text-sm leading-relaxed text-foreground/80">{seg.text}</p>
                        )}
                        {renderTranslation(seg.sequence_id, seg.is_partial)}
                      </div>
                    ))}
                  </div>
                )
              }

              const seg = group.seg
              return seg.is_partial ? (
                <div key={seg.id} className="px-2 py-1.5 rounded-md">
                  <p className="text-sm leading-relaxed italic text-muted-foreground">{seg.text}</p>
                  {renderTranslation(seg.sequence_id, true)}
                </div>
              ) : (
                <div key={seg.id} className="flex items-start gap-3 px-2 py-1.5 rounded-md hover:bg-muted/60">
                  <span className="min-w-[50px] shrink-0 mt-0.5 text-right text-xs tabular-nums text-muted-foreground/60">
                    {formatTime(seg.audio_start_time)}
                  </span>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm leading-relaxed text-foreground/80">{seg.text}</p>
                    {renderTranslation(seg.sequence_id, false)}
                  </div>
                </div>
              )
            })}
            {vadSpeaking && recognizing}
          </>
        )}
      </div>
    </div>
  )
}
