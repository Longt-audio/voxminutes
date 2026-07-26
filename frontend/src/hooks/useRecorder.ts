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
import { toast } from 'sonner'

export const DEFAULT_ASR_MODEL = 'x-asr-480ms'

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

  const clearStopFallback = useCallback(() => {
    if (stopFallbackRef.current) {
      clearTimeout(stopFallbackRef.current)
      stopFallbackRef.current = null
    }
  }, [])

  useEffect(() => {
    const registerEvents = async () => {
      const u1 = await onRecordingStarted(() => {
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
            const result = await apiSaveTranscript(
              title,
              finals.map((seg) => ({
                id: seg.id,
                text: seg.text,
                timestamp: seg.timestamp,
                start_ms: Math.round(seg.audio_start_time * 1000),
                end_ms: Math.round(seg.audio_end_time * 1000),
                duration: Math.round(seg.duration * 1000),
                source: seg.source,
              })),
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
      unlisteners.current.forEach((u) => u())
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
        const modelToUse = options.modelName || DEFAULT_ASR_MODEL

        // 先写语言偏好：SenseVoice 引擎在模型加载时读取该偏好构造识别器
        if (options.language) {
          await setLanguagePreference(options.language).catch(() => {})
        }
        await sherpaOnnxLoadModel(modelToUse)
        await apiSaveTranscriptConfig(
          modelToUse.startsWith('x-asr-') ? 'x-asr' : 'sherpaonnx',
          modelToUse,
          null
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
      await ipcStopRecording('/dev/null')
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

/** 录音计时：录音中每秒 +1（暂停时冻结）。 */
export function useRecordingTimer() {
  const isRecording = useAppStore((s) => s.isRecording)
  const isPaused = useAppStore((s) => s.isPaused)
  const setRecordingDuration = useAppStore((s) => s.setRecordingDuration)

  useEffect(() => {
    if (!isRecording) {
      setRecordingDuration(0)
      return
    }
    if (isPaused) return
    const timer = setInterval(() => {
      useAppStore.setState((s) => ({ recordingDuration: s.recordingDuration + 1 }))
    }, 1000)
    return () => clearInterval(timer)
  }, [isRecording, isPaused, setRecordingDuration])
}
