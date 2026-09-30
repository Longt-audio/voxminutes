import { useEffect, useCallback, useRef } from 'react'
import { useRouter } from 'next/navigation'
import { UnlistenFn } from '@tauri-apps/api/event'
import { useAppStore } from '@/state'
import {
  startRecording as ipcStartRecording,
  stopRecording as ipcStopRecording,
  pauseRecording as ipcPauseRecording,
  resumeRecording as ipcResumeRecording,
  onRecordingStarted,
  onRecordingStopped,
  onRecordingPaused,
  onRecordingResumed,
  onMicMuteChanged,
  onDefaultDeviceChanged,
  onWaitingForAudioDevice,
  apiSaveTranscript,
  apiSaveTranscriptConfig,
  sherpaOnnxLoadModel,
  getRecordingState,
  setLanguagePreference,
  setMicMute as ipcSetMicMute,
} from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { useRemoteModelChoiceStore } from '@/stores/remoteModelChoiceStore'
import { toast } from 'sonner'

export const DEFAULT_ASR_MODEL = 'x-asr-480ms'

/** 「远程模型」在前端的占位名。真实远程模型 id 由 useRemoteModelChoice 解析，
 *  这里只是让 UI 有一个稳定的「走远程」标记（后端识别 qwen3-asr-remote 前缀）。 */
export const REMOTE_ASR_PLACEHOLDER = 'qwen3-asr-remote'

function generateRecordingTitle(base: string): string {
  const now = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${base}_${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}_${pad(now.getHours())}-${pad(now.getMinutes())}`
}

export interface StartOptions {
  modelName: string
  micDeviceName?: string | null
  systemDeviceName?: string | null
  language?: string
}

// CJK（含日文假名、韩文音节）前后不加空格
const CJK_RE = /[\u2e80-\u9fff\uf900-\ufaff\u3040-\u30ff\uac00-\ud7af]/
const PUNCT_RE = /[\p{P}\p{S}]/u

/** 同段落内相邻转写单元的智能拼接：空白/CJK/标点相邻直接连，否则补空格 */
function joinParagraphText(parts: string[]): string {
  let out = ''
  for (const raw of parts) {
    const part = raw.trim()
    if (!part) continue
    if (!out) {
      out = part
      continue
    }
    const tail = out[out.length - 1]
    const head = part[0]
    if (/\s/.test(tail) || /\s/.test(head) || CJK_RE.test(tail) || CJK_RE.test(head) || PUNCT_RE.test(head)) {
      out += part
    } else {
      out += ' ' + part
    }
  }
  return out
}

export function useRecorder() {
  const router = useRouter()
  const {
    isRecording,
    isPaused,
    isProcessing,
    setRecording,
    setPaused,
    setProcessing,
    clearTranscripts,
    setMeetingName,
    setMeetingFolderPath,
    setAsrModelStatus,
    setMicMuted,
    setDefaultDevices,
    setLatestRecordingId,
  } = useAppStore()

  // 事件监听只在挂载时注册一次，用 ref 让回调始终拿到当前语言的文案
  const t = useMessages()
  const tRef = useRef(t)
  useEffect(() => {
    tRef.current = t
  })

  const unlisteners = useRef<UnlistenFn[]>([])
  const startingRef = useRef(false)
  const stopFallbackRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  // 录音开始的墙钟时刻：流式段落合并落库时作为 start_ms 的基准（0 = 未知）
  const recordingStartRef = useRef(0)

  const clearStopFallback = useCallback(() => {
    if (stopFallbackRef.current) {
      clearTimeout(stopFallbackRef.current)
      stopFallbackRef.current = null
    }
  }, [])

  useEffect(() => {
    let disposed = false
    const registerEvents = async () => {
      const u1 = await onRecordingStarted(() => {
        recordingStartRef.current = Date.now()
        setRecording(true)
        setProcessing(false)
        // set_mic_mute 需要活动录音：录音管理器已就绪后，把开始前的静音选择
        // （子窗口与主窗口共享的 store 状态）下发到音频管线
        const muted = useAppStore.getState().isMicMuted
        ipcSetMicMute(muted).catch(() => {})
      })

      const u2 = await onRecordingStopped(async (payload) => {
        clearStopFallback()
        setRecording(false)
        setPaused(false)
        setProcessing(false)
        if (payload.folder_path) setMeetingFolderPath(payload.folder_path)

        // 自动保存转录结果到历史记录
        const state = useAppStore.getState()
        const finals = state.transcripts.filter((t) => !t.is_partial && t.text.trim().length > 0)
        if (finals.length > 0) {
          const title = payload.meeting_name || state.meetingName || generateRecordingTitle(tRef.current.recDefaultTitle)
          try {
            // 定稿译文（Rust 在 emit recording-stopped 前已排空翻译队列，此刻 store 已齐）
            const translations = state.translations
            // 段落落库时间基准：录音开始墙钟；缺失时退化为最早的段落到达时刻
            const paraStarts = state.paragraphStarts
            const paraEnds = state.paragraphEnds
            const paraStartValues = Object.values(paraStarts)
            const baseMs =
              recordingStartRef.current > 0
                ? recordingStartRef.current
                : paraStartValues.length > 0
                  ? Math.min(...paraStartValues)
                  : 0

            // paragraph_id 为空的段逐条原样存；连续相同 paragraph_id 的单元合并为一条
            type SegmentRecord = {
              id: string
              text: string
              timestamp?: string
              start_ms?: number
              end_ms?: number
              duration?: number
              speaker?: string
              source?: string
              translation?: string
            }
            const segments: SegmentRecord[] = []
            let i = 0
            while (i < finals.length) {
              const seg = finals[i]
              if (seg.paragraph_id == null) {
                segments.push({
                  id: seg.id,
                  text: seg.text,
                  timestamp: seg.timestamp,
                  start_ms: Math.round(seg.audio_start_time * 1000),
                  end_ms: Math.round(seg.audio_end_time * 1000),
                  duration: Math.round(seg.duration * 1000),
                  source: seg.source,
                  translation: translations.get(seg.sequence_id) || undefined,
                })
                i++
                continue
              }
              const pid = seg.paragraph_id
              const group: typeof finals = []
              while (i < finals.length && finals[i].paragraph_id === pid) {
                group.push(finals[i])
                i++
              }
              const startMs = paraStarts[pid] != null ? Math.round(paraStarts[pid] - baseMs) : undefined
              const endMs =
                paraEnds[pid] != null && startMs != null
                  ? Math.max(startMs, Math.round(paraEnds[pid] - baseMs))
                  : startMs
              // 译文与原文同样的方式拼接（组内各单元译文按序智能连接）
              const groupTranslation = joinParagraphText(
                group.map((g) => translations.get(g.sequence_id)).filter((t): t is string => !!t)
              )
              segments.push({
                id: group[0].id,
                text: joinParagraphText(group.map((g) => g.text)),
                timestamp: group[0].timestamp,
                start_ms: startMs,
                end_ms: endMs,
                duration: startMs != null && endMs != null ? endMs - startMs : undefined,
                source: group[0].source,
                translation: groupTranslation || undefined,
              })
            }

            const result = await apiSaveTranscript(
              title,
              segments,
              payload.folder_path || state.meetingFolderPath || undefined
            )
            if (result.recording_id) {
              setLatestRecordingId(result.recording_id)
              toast.success(tRef.current.recSavedToHistory)
            }
          } catch (e) {
            console.error('保存转录记录失败:', e)
            toast.error(tRef.current.recSaveFailed, { description: e instanceof Error ? e.message : String(e) })
          }
        }
        setTimeout(() => router.push('/history'), 400)
      })

      const u3 = await onRecordingPaused(() => setPaused(true))
      const u4 = await onRecordingResumed(() => setPaused(false))
      const u5 = await onMicMuteChanged((payload) => setMicMuted(payload.muted))
      const u6 = await onDefaultDeviceChanged((payload) => {
        setDefaultDevices({
          microphone: payload.microphone ?? null,
          speaker: payload.system_audio ?? null,
        })
        const m = tRef.current
        toast.info(m.recDeviceSwitched, {
          description: m.recDeviceSwitchedDesc
            .replace('{mic}', payload.microphone ?? m.recNone)
            .replace('{sys}', payload.system_audio ?? m.recNone),
        })
      })
      const u7 = await onWaitingForAudioDevice(() => {
        toast.warning(tRef.current.recDeviceDisconnected, {
          description: tRef.current.recWaitingForDevice,
          duration: 5000,
        })
      })
      // 若在 await 注册过程中组件已卸载，立即释放已注册的监听，避免泄漏
      if (disposed) {
        const arr = [u1, u2, u3, u4, u5, u6, u7]
        arr.forEach((u) => { try { u() } catch {} })
        return
      }
      unlisteners.current = [u1, u2, u3, u4, u5, u6, u7]

      // 与后端同步状态（防止组件重挂载后丢失事件）
      try {
        const state = await getRecordingState()
        setRecording(!!state.is_recording)
        setPaused(!!state.is_paused)
        if (!state.is_recording && !state.is_active) setProcessing(false)
      } catch (e) {
        console.warn('[useRecorder] 状态同步失败:', e)
      }
    }
    registerEvents()
    return () => {
      disposed = true
      unlisteners.current.forEach((u) => {
        try {
          u()
        } catch {
          // 忽略重复 unlisten 的竞态错误
        }
      })
      unlisteners.current = []
      clearStopFallback()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  /** 选择模型 + 设备后开始录音 */
  const startRecording = useCallback(
    async (options: StartOptions) => {
      if (startingRef.current) return
      startingRef.current = true
      setProcessing(true)
      setAsrModelStatus('loading')
      try {
        let modelToUse = options.modelName || DEFAULT_ASR_MODEL
        // 前端「远程模型」占位名统一映射到后端识别的 qwen3-asr-remote
        if (modelToUse === 'remote') {
          modelToUse = 'qwen3-asr-remote'
        }
        // 远程时把「当前真实选择」一并传给后端（2026-09-28 竞态修复）：
        // 全局 store 是前端唯一权威内存副本，选择器改选后这里立刻能读到，
        // 而后端 remote_asr_config.json 的异步持久化可能还没落盘 ——
        // 不显式传，引擎会拿旧模型跑整场录音（UI 显示与计费模型对不上）。
        const isRemoteModel = modelToUse.startsWith('qwen3-asr-remote')
        const remoteAsrModel = isRemoteModel
          ? useRemoteModelChoiceStore.getState().values.asr || null
          : null

        // 先写语言偏好：SenseVoice 引擎在模型加载时读取该偏好构造识别器
        if (options.language) {
          await setLanguagePreference(options.language).catch(() => {})
        }
        await sherpaOnnxLoadModel(modelToUse)
        await apiSaveTranscriptConfig(
          modelToUse.startsWith('x-asr-')
            ? 'x-asr'
            : isRemoteModel
              ? 'remote-qwen3-asr'
              : 'sherpaonnx',
          modelToUse,
          null,
          remoteAsrModel
        )
        setAsrModelStatus('loaded')
        // 回写实际使用的模型名，保持左侧栏信息框显示与当前引擎一致
        useAppStore.getState().setSelectedModel(modelToUse)

        const title = generateRecordingTitle(tRef.current.recDefaultTitle)
        setMeetingName(title)
        clearTranscripts()
        await ipcStartRecording(title, options.micDeviceName, options.systemDeviceName)
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        setRecording(false)
        setProcessing(false)
        setAsrModelStatus('error')
        toast.error(tRef.current.recStartFailed, { description: msg })
      } finally {
        startingRef.current = false
      }
    },
    [setProcessing, setAsrModelStatus, setMeetingName, clearTranscripts, setRecording]
  )

  const stopRecording = useCallback(async () => {
    clearStopFallback()
    try {
      setProcessing(true)
      // 停止录音不需要保存路径（数据由 Rust 侧落盘到会议目录）。
      // 这里**必须**传空串：旧代码传 POSIX 占位符 '/dev/null'，在 Windows 上它是
      // 「驱动器相对路径」，Rust 侧取 parent() 得到 `\dev` → 每次停止录音都会在盘根
      // 创建多余的 C:\dev 目录（盘根不可写时还会让已经成功的停止动作报错）。
      await ipcStopRecording('')
      setAsrModelStatus('idle')
      stopFallbackRef.current = setTimeout(() => {
        stopFallbackRef.current = null
        setProcessing(false)
      }, 15000)
    } catch (e) {
      clearStopFallback()
      console.error('停止录音失败:', e)
      setProcessing(false)
      toast.error(tRef.current.recStopFailed, { description: e instanceof Error ? e.message : String(e) })
    }
  }, [setProcessing, setAsrModelStatus, clearStopFallback])

  const togglePause = useCallback(async () => {
    try {
      if (isPaused) {
        await ipcResumeRecording()
      } else {
        await ipcPauseRecording()
      }
    } catch (e) {
      toast.error(tRef.current.recActionFailed, { description: e instanceof Error ? e.message : String(e) })
    }
  }, [isPaused])

  return { isRecording, isPaused, isProcessing, startRecording, stopRecording, togglePause }
}

/** 录音计时：每秒以 Rust 侧真实时长为准（active_duration 已扣除暂停，采样时钟驱动）。
 *  不在前端数 tick —— 窗口被遮挡/最小化时 WebView 会节流 setInterval，朴素计数会越落越慢；
 *  轮询真值即使被节流，下一次触发也会自动对齐。 */
export function useRecordingTimer() {
  const isRecording = useAppStore((s) => s.isRecording)
  const setRecordingDuration = useAppStore((s) => s.setRecordingDuration)

  useEffect(() => {
    if (!isRecording) {
      setRecordingDuration(0)
      return
    }
    const tick = () => {
      getRecordingState()
        .then((st) => {
          const secs = st.active_duration ?? st.recording_duration
          if (secs != null) setRecordingDuration(Math.floor(secs))
        })
        .catch(() => {})
    }
    tick()
    const timer = setInterval(tick, 1000)
    return () => clearInterval(timer)
  }, [isRecording, setRecordingDuration])
}
