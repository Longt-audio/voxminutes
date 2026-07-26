import { useEffect, useRef } from 'react'
import { UnlistenFn } from '@tauri-apps/api/event'
import { useAppStore } from '@/state'
import { onTranscriptUpdate, onTranslateUpdate } from '@/services/ipc'
import type { TranscriptSegment } from '@/types'

let idCounter = 0
function generateId(): string {
  idCounter++
  return `seg-${Date.now()}-${idCounter}`
}

/** 订阅 transcript-update / translate-update 事件，写入全局 store（仅在录音中生效）。 */
export function useTranscripts() {
  const addTranscript = useAppStore((s) => s.addTranscript)
  const addTranslation = useAppStore((s) => s.addTranslation)
  const addPartialTranslation = useAppStore((s) => s.addPartialTranslation)
  const isRecording = useAppStore((s) => s.isRecording)
  const unlistenRef = useRef<UnlistenFn | null>(null)
  const translateUnlistenRef = useRef<UnlistenFn | null>(null)

  useEffect(() => {
    if (!isRecording) {
      if (unlistenRef.current) {
        unlistenRef.current()
        unlistenRef.current = null
      }
      if (translateUnlistenRef.current) {
        translateUnlistenRef.current()
        translateUnlistenRef.current = null
      }
      return
    }

    let cancelled = false
    ;(async () => {
      const unlisten = await onTranscriptUpdate((update) => {
        const segment: TranscriptSegment = {
          id: generateId(),
          text: update.text,
          timestamp: update.timestamp,
          sequence_id: update.sequence_id,
          chunk_start_time: update.chunk_start_time,
          is_partial: update.is_partial,
          confidence: update.confidence,
          audio_start_time: update.audio_start_time,
          audio_end_time: update.audio_end_time,
          duration: update.duration,
          source: update.source,
        }
        addTranscript(segment)
      })
      const translateUnlisten = await onTranslateUpdate((update) => {
        if (!update.translated_text) return
        // partial 是全文快照：覆盖式存入 partialTranslations；最终版写正式译文并清 partial
        if (update.is_partial) {
          addPartialTranslation(update.sequence_id, update.translated_text)
        } else {
          addTranslation(update.sequence_id, update.translated_text)
        }
      })
      if (cancelled) {
        unlisten()
        translateUnlisten()
      } else {
        unlistenRef.current = unlisten
        translateUnlistenRef.current = translateUnlisten
      }
    })()

    return () => {
      cancelled = true
      if (unlistenRef.current) {
        unlistenRef.current()
        unlistenRef.current = null
      }
      if (translateUnlistenRef.current) {
        translateUnlistenRef.current()
        translateUnlistenRef.current = null
      }
    }
  }, [isRecording, addTranscript, addTranslation, addPartialTranslation])
}
