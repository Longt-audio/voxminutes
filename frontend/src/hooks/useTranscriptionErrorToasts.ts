import { useEffect, useRef } from 'react'
import { toast } from 'sonner'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onTranscriptionError, onTranscriptionWarning, onTranslationWarning } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'

/** 解析 transcription-warning 负载：后端可能发纯文本（网关错误消息）或带 code 的 JSON。 */
export function parseTranscriptionWarning(raw: string): { code?: string; message: string } {
  try {
    const v = JSON.parse(raw)
    if (v && typeof v === 'object' && typeof v.message === 'string') {
      return { code: typeof v.code === 'string' ? v.code : undefined, message: v.message }
    }
  } catch {}
  return { message: raw }
}

/**
 * 全局监听转录链路的错误/警告事件（此前后端 emit 了但前端无人接收，远程 ASR 故障完全无感）。
 * error → 红色 toast（8s）；warning → 黄色 toast（5s）。
 * 带 code 的告警用 i18n 文案（中/英/日/韩）；没有对应文案时回退显示 message。
 */
export function useTranscriptionErrorToasts() {
  const t = useMessages()
  const tRef = useRef(t)
  tRef.current = t

  useEffect(() => {
    let unlistenError: UnlistenFn | undefined
    let unlistenWarning: UnlistenFn | undefined
    let unlistenTranslate: UnlistenFn | undefined
    // 翻译链路告警（空译文等）：后端已做 60s 节流，这里直接提示即可。
    onTranslationWarning((e) => {
      toast.warning(e.message, { duration: 12000 })
    })
      .then((fn) => {
        unlistenTranslate = fn
      })
      .catch(() => {})
    onTranscriptionError((e) => {
      toast.error(e.userMessage || e.error, { duration: 8000 })
    })
      .then((fn) => {
        unlistenError = fn
      })
      .catch(() => {})
    onTranscriptionWarning((raw) => {
      const { code, message } = parseTranscriptionWarning(raw)
      const text = code ? tRef.current.recAsrWarningText(code, message) : message
      // 断线较久的重复提示（reconnecting_long）时长更长，避免用户错过
      toast.warning(text, { duration: code === 'reconnecting_long' ? 8000 : 5000 })
    })
      .then((fn) => {
        unlistenWarning = fn
      })
      .catch(() => {})
    return () => {
      unlistenError?.()
      unlistenWarning?.()
      unlistenTranslate?.()
    }
  }, [])
}
