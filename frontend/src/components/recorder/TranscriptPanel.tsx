'use client'

import { useEffect, useRef } from 'react'
import { Mic } from 'lucide-react'
import { useAppStore } from '@/state'
import { useTranscripts } from '@/hooks/useTranscripts'
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
  const t = useMessages()
  useTranscripts()

  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = scrollRef.current
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: 'instant' })
  }, [transcripts, translations, partialTranslations])

  return (
    <div className="flex-1 min-h-0 flex flex-col">
      <div
        ref={scrollRef}
        className="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-2"
        style={{ overflowAnchor: 'none' }}
      >
        {transcripts.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center gap-2 text-center">
            <Mic className="h-5 w-5 text-muted-foreground" />
            <div className="text-sm font-medium text-muted-foreground">{t.emptyTitle}</div>
            <div className="text-xs text-muted-foreground/60">{t.emptyHint}</div>
          </div>
        ) : (
          transcripts.map((seg: TranscriptSegment) => {
            const inlineTranslation = translations.get(seg.sequence_id)
            const partialTranslation = partialTranslations.get(seg.sequence_id)
            return seg.is_partial ? (
              <div key={seg.id} className="px-2 py-1.5 rounded-md">
                <p className="text-sm leading-relaxed italic text-muted-foreground">{seg.text}</p>
                {translateEnabled && inlineTranslation ? (
                  <p className="text-xs leading-relaxed italic text-blue-500/80 mt-0.5 pl-2 border-l-2 border-blue-200">
                    {inlineTranslation}
                  </p>
                ) : (
                  translateEnabled &&
                  partialTranslation && (
                    <p className="text-xs leading-relaxed italic text-blue-400/60 mt-0.5 pl-2 border-l-2 border-blue-200/60">
                      {partialTranslation}
                    </p>
                  )
                )}
              </div>
            ) : (
              <div key={seg.id} className="flex items-start gap-3 px-2 py-1.5 rounded-md hover:bg-muted/60">
                <span className="min-w-[50px] shrink-0 mt-0.5 text-right text-xs tabular-nums text-muted-foreground/60">
                  {formatTime(seg.audio_start_time)}
                </span>
                <div className="flex-1 min-w-0">
                  <p className="text-sm leading-relaxed text-foreground/80">{seg.text}</p>
                  {translateEnabled && inlineTranslation ? (
                    <p className="text-xs leading-relaxed text-blue-600 mt-0.5 pl-2 border-l-2 border-blue-300">
                      {inlineTranslation}
                    </p>
                  ) : (
                    translateEnabled &&
                    partialTranslation && (
                      <p className="text-xs leading-relaxed italic text-blue-400/70 mt-0.5 pl-2 border-l-2 border-blue-200">
                        {partialTranslation}
                      </p>
                    )
                  )}
                </div>
              </div>
            )
          })
        )}
      </div>
    </div>
  )
}
