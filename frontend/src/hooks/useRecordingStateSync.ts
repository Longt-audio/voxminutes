'use client'

import { useEffect } from 'react'
import {
  getRecordingState,
  onRecordingPaused,
  onRecordingResumed,
  onRecordingStarted,
  onRecordingStopped,
} from '@/services/ipc'
import { useAppStore } from '@/state'

/**
 * 录音状态的**全局**同步（2026-09-24）。
 *
 * 为什么单独抽出来：录音/识别本身都在 Rust 侧跑（切页面、最小化都不受影响），
 * 但「isRecording / isPaused」这两个 store 状态原来是靠 `useRecorder`（挂在录音页
 * `RecorderPanel`）里的监听维持的 —— 切到别的页面就退订，于是：
 *   · 从托盘/系统推送停止录音时，别的页面看到的还是「录音中」（全局指示条卡住）；
 *   · 暂停/继续状态在跨页面时发散。
 * 这个 hook 只做**状态同步**（不保存、不跳转），因此与 useRecorder 重复监听也无副作用；
 * 录音页仍然负责保存转录与跳转历史。
 */
export function useRecordingStateSync() {
  useEffect(() => {
    let disposed = false
    let unlisteners: Array<() => void> = []

    const register = async () => {
      const { setRecording, setPaused } = useAppStore.getState()
      const us = await Promise.all([
        onRecordingStarted(() => setRecording(true)),
        onRecordingStopped(() => setRecording(false)),
        onRecordingPaused(() => setPaused(true)),
        onRecordingResumed(() => setPaused(false)),
      ])
      if (disposed) {
        us.forEach((u) => {
          try { u() } catch {}
        })
        return
      }
      unlisteners = us
      // 与后端核对一次（页面重挂载/热重载后状态不会丢）
      try {
        const state = await getRecordingState()
        setRecording(!!state.is_recording)
        setPaused(!!state.is_paused)
      } catch {}
    }

    void register()
    return () => {
      disposed = true
      unlisteners.forEach((u) => {
        try { u() } catch {}
      })
      unlisteners = []
    }
  }, [])
}
