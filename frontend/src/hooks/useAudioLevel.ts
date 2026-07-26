'use client'

import { useEffect, useRef } from 'react'
import { useAppStore } from '@/state'
import {
  startAudioLevelMonitoring,
  stopAudioLevelMonitoring,
  onAudioLevels,
} from '@/services/ipc'

/**
 * 录音期间监听 Rust 推送的真实音频数据（50ms/帧）：
 * - 频谱做「快上升、慢下落」平滑后写入 store，供频谱组件渲染
 * - 每路设备（麦克风/系统音频）的 RMS 电平也写入 store，用于诊断采集
 */
export function useAudioLevel() {
  const isRecording = useAppStore((s) => s.isRecording)
  const setAudioSpectrum = useAppStore((s) => s.setAudioSpectrum)
  const setAudioActive = useAppStore((s) => s.setAudioActive)
  const setAudioLevels = useAppStore((s) => s.setAudioLevels)
  const smoothRef = useRef<number[]>([])

  useEffect(() => {
    if (!isRecording) {
      smoothRef.current = []
      setAudioSpectrum([])
      setAudioActive(false)
      setAudioLevels({ mic: 0, system: 0 })
      stopAudioLevelMonitoring().catch(() => {})
      return
    }

    let disposed = false
    let unlisten: (() => void) | undefined

    // 使用当前默认设备名监听（名称仅用于 Rust 侧标注 input/output）
    const { defaultDevices } = useAppStore.getState()
    const names = [defaultDevices.microphone, defaultDevices.speaker]
      .filter((n): n is string => !!n)
    startAudioLevelMonitoring(names.length > 0 ? names : ['Microphone']).catch(() => {})

    onAudioLevels((update) => {
      const entry = update.levels.find((l) => l.spectrum && l.spectrum.length > 0)
      if (entry) {
        const raw = entry.spectrum
        const prev = smoothRef.current
        const next = raw.map((v, i) => {
          const p = prev[i] ?? 0
          return v > p ? v : p * 0.86 // 快上升、慢下落（PotPlayer 手感）
        })
        smoothRef.current = next
        setAudioSpectrum(next)
      }
      setAudioActive(update.levels.some((l) => l.is_active))
      setAudioLevels({
        mic: update.levels.find((l) => l.device_type === 'input')?.rms_level ?? 0,
        system: update.levels.find((l) => l.device_type === 'output')?.rms_level ?? 0,
      })
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })

    return () => {
      disposed = true
      unlisten?.()
      stopAudioLevelMonitoring().catch(() => {})
    }
  }, [isRecording, setAudioSpectrum, setAudioActive, setAudioLevels])
}
