'use client'

import { useEffect, useRef, useState, useCallback } from 'react'
import { UnlistenFn } from '@tauri-apps/api/event'
import {
  startAudioTest,
  stopAudioTest,
  replayAudioTest,
  onAudioTestTranscript,
  onAudioTestPlaybackStarted,
  onAudioLevels,
  onTranscriptionError,
  onTranscriptionWarning,
} from '@/services/ipc'
import { useAppStore } from '@/state'
import { parseTranscriptionWarning } from '@/hooks/useTranscriptionErrorToasts'
import type { TranscriptSegment, TranscriptUpdate } from '@/types'

let idCounter = 0
function generateId(): string {
  idCounter++
  return `audio-test-${Date.now()}-${idCounter}`
}

/**
 * 音频链路自检 hook：
 * - active（弹窗打开且用户点了「开始测试」）时启动后端 start_audio_test
 *   （真实链路：扬声器播放 → 系统回采 + 麦克风 → 模型转写），
 *   关闭/重新开始时 stop_audio_test 并清理。
 * - 监听 audio-test-transcript 累积识别结果（partial 按 sequence_id 替换、final 追加）。
 * - audio-test-playback-started 驱动播放进度条；replay 重新播放。
 * - 播放进度到 100% 后再等 4 秒（让远程模型把尾巴吐完）自动 stop_audio_test 并置 finished，
 *   避免远程流式模型（按音频时长计费）挂着空转烧钱。
 * - runId：弹窗里「重新测试」递增 runId 重跑。
 * - ⚠️ 模型/设备选择不再作为重启依赖：切换 ASR 模型或采集设备不会自动重启测试
 *   （用户要求：每次更换选择都必须重新点击「开始测试」才重新测试）。
 *   启动时从 ref 读取当前选择，保证读到的是最新值。
 * - 后端自检会自行启动 simple_level_monitor 并推 audio-levels，这里只做平滑后供能量条渲染
 *   （不写全局 store，避免与录音态的 useAudioLevel 互相干扰）。
 */
export function useAudioTest(
  modelName: string,
  active: boolean,
  micDeviceName?: string | null,
  systemDeviceName?: string | null,
  runId = 0,
  remoteAsrModel?: string | null,
) {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [segments, setSegments] = useState<TranscriptSegment[]>([])
  const [duration, setDuration] = useState(0)
  const [progress, setProgress] = useState(0)
  const [isPlaying, setIsPlaying] = useState(false)
  const [finished, setFinished] = useState(false)
  const [levelPeak, setLevelPeak] = useState(0)
  const [levelActive, setLevelActive] = useState(false)
  const clearTranscripts = useAppStore((s) => s.clearTranscripts)

  // 启动参数走 ref：effect 只在 active/runId 变化时执行，启动时读最新值，
  // 模型/设备切换不会触发 effect 重跑（不自动重启测试）。
  const modelNameRef = useRef(modelName)
  modelNameRef.current = modelName
  const remoteAsrModelRef = useRef(remoteAsrModel ?? null)
  remoteAsrModelRef.current = remoteAsrModel ?? null
  const micDeviceRef = useRef(micDeviceName ?? null)
  micDeviceRef.current = micDeviceName ?? null
  const systemDeviceRef = useRef(systemDeviceName ?? null)
  systemDeviceRef.current = systemDeviceName ?? null

  const progressTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const finishTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const transcriptUnlistenRef = useRef<UnlistenFn | null>(null)
  const playbackUnlistenRef = useRef<UnlistenFn | null>(null)
  const levelsUnlistenRef = useRef<UnlistenFn | null>(null)
  const errorUnlistenRef = useRef<UnlistenFn | null>(null)
  const warningUnlistenRef = useRef<UnlistenFn | null>(null)
  const playbackStartedRef = useRef(false)
  const smoothPeakRef = useRef(0)

  const stopProgressTimer = useCallback(() => {
    if (progressTimerRef.current) {
      clearInterval(progressTimerRef.current)
      progressTimerRef.current = null
    }
  }, [])

  const stopFinishTimer = useCallback(() => {
    if (finishTimerRef.current) {
      clearTimeout(finishTimerRef.current)
      finishTimerRef.current = null
    }
  }, [])

  const startProgressTimer = useCallback(
    (totalSeconds: number) => {
      stopProgressTimer()
      stopFinishTimer()
      const startTime = Date.now()
      const totalMs = totalSeconds * 1000
      setProgress(0)
      setIsPlaying(true)

      progressTimerRef.current = setInterval(() => {
        const elapsed = Date.now() - startTime
        const pct = Math.min(100, (elapsed / totalMs) * 100)
        setProgress(pct)
        if (pct >= 100) {
          stopProgressTimer()
          setIsPlaying(false)
          // 播放结束后多等 4 秒让远程模型吐出尾部结果，然后自动停止测试（远程按时长计费）
          finishTimerRef.current = setTimeout(() => {
            finishTimerRef.current = null
            stopAudioTest().catch(() => {})
            setFinished(true)
          }, 4000)
        }
      }, 100)
    },
    [stopProgressTimer, stopFinishTimer]
  )

  useEffect(() => {
    if (!active) {
      // 弹窗关闭/回到待开始时重置状态
      setError(null)
      setSegments([])
      setProgress(0)
      setIsPlaying(false)
      setFinished(false)
      setDuration(0)
      setLevelPeak(0)
      setLevelActive(false)
      smoothPeakRef.current = 0
      playbackStartedRef.current = false
      return
    }

    let mounted = true

    const setup = async () => {
      setIsLoading(true)
      setError(null)
      setSegments([])
      setFinished(false)
      // 自检转写不进入实时转写 store，先清空防串扰
      clearTranscripts()

      try {
        // 先确保旧会话已停（cleanup 里的 stop 是 fire-and-forget，这里显式等一次，
        // 保证后端只存在一个音频测试管线——此前 stop/start 竞态会产生两条采集链）。
        await stopAudioTest().catch(() => {})

        // 先注册事件监听再调 startAudioTest，避免错过立即触发的 playback-started
        playbackUnlistenRef.current = await onAudioTestPlaybackStarted((payload) => {
          if (!mounted) return
          playbackStartedRef.current = true
          setDuration(payload.duration)
          startProgressTimer(payload.duration)
        })

        transcriptUnlistenRef.current = await onAudioTestTranscript((payload: TranscriptUpdate) => {
          if (!mounted) return
          setSegments((prev) => {
            const segment: TranscriptSegment = { id: generateId(), ...payload }
            // 与会议转写展示逻辑一致：同 sequence_id 的 partial/final 互相替换并保留 React key
            const existingIndex = prev.findIndex((s) => s.sequence_id === segment.sequence_id)
            const next = [...prev]
            if (existingIndex >= 0) {
              next[existingIndex] = { ...segment, id: next[existingIndex].id }
            } else {
              next.push(segment)
            }
            next.sort((a, b) => a.sequence_id - b.sequence_id)
            return next
          })
        })

        // 转写链路的错误/警告（自检与录音共用同一链路事件）直接显示到弹窗错误区
        errorUnlistenRef.current = await onTranscriptionError((e) => {
          if (!mounted) return
          setError(e.userMessage || e.error)
        })
        warningUnlistenRef.current = await onTranscriptionWarning((raw) => {
          if (!mounted) return
          // 后端 warning 可能是带 code 的 JSON（如自动重连中），只取 message 展示
          const { message } = parseTranscriptionWarning(raw)
          setError(message)
        })

        // 能量条数据源：后端自检启动的 simple_level_monitor 推送的 audio-levels
        levelsUnlistenRef.current = await onAudioLevels((update) => {
          if (!mounted || !update.levels || update.levels.length === 0) return
          const maxPeak = Math.max(...update.levels.map((l) => l.peak_level))
          // EMA 平滑，波形手感与 mig 版一致
          smoothPeakRef.current = 0.25 * maxPeak + 0.75 * smoothPeakRef.current
          setLevelPeak(smoothPeakRef.current)
          setLevelActive(update.levels.some((l) => l.is_active))
        })

        // 注意：start_audio_test 会把被测模型保存为「当前转写配置」——这是后端既有行为，
        // 自检选用的模型与随后开始录音的模型一致，因此保留该行为。
        // 参数从 ref 读取：此刻的值即用户点击「开始测试」时看到的选择。
        // remoteAsrModelRef：远程时把当前真实模型 id 一并传给后端（2026-09-28 竞态修复：
        // 选择器的异步持久化可能未落盘，不显式传会拿旧模型跑测试）。
        const wavDuration = await startAudioTest(
          modelNameRef.current,
          micDeviceRef.current,
          systemDeviceRef.current,
          remoteAsrModelRef.current,
        )
        if (!mounted) return
        // 兜底：后端未发 playback-started 时，用命令返回值启动进度条
        if (!playbackStartedRef.current) {
          setDuration(wavDuration)
          startProgressTimer(wavDuration)
        }
      } catch (e: any) {
        if (!mounted) return
        setError(e?.message || String(e))
      } finally {
        if (mounted) setIsLoading(false)
      }
    }

    setup()

    return () => {
      mounted = false
      stopProgressTimer()
      stopFinishTimer()
      transcriptUnlistenRef.current?.()
      transcriptUnlistenRef.current = null
      playbackUnlistenRef.current?.()
      playbackUnlistenRef.current = null
      levelsUnlistenRef.current?.()
      levelsUnlistenRef.current = null
      errorUnlistenRef.current?.()
      errorUnlistenRef.current = null
      warningUnlistenRef.current?.()
      warningUnlistenRef.current = null
      stopAudioTest().catch(() => {})
      // 兜底清理，确保自检转写不泄漏进实时转写 store
      clearTranscripts()
    }
  }, [active, runId, startProgressTimer, stopProgressTimer, stopFinishTimer, clearTranscripts])

  const replay = useCallback(async () => {
    try {
      setIsPlaying(false)
      setProgress(0)
      await replayAudioTest()
    } catch (e: any) {
      setError(e?.message || String(e))
    }
  }, [])

  return {
    isLoading,
    error,
    segments,
    duration,
    progress,
    isPlaying,
    finished,
    levelPeak,
    levelActive,
    replay,
  }
}
