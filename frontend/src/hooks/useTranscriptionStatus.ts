'use client'

import { useEffect } from 'react'
import { onTranscriptionStatus, onRecordingStarted, onRecordingStopped } from '@/services/ipc'
import { useAppStore } from '@/state'

/**
 * 识别链路状态的**全局**同步（2026-09-29）。
 *
 * 背景（真实事故）：用户用远程流式 ASR 录一场 13 分钟的 vlog，中途积分用尽 → 网关下发
 * 「积分不足」并断开 → 识别停止，但界面**没有任何持久痕迹**：底部常驻胶囊依然显示
 * 「● 12:34 实时转写中…」，用户以为还在识别，直到发现整场没有字幕。
 * 那条 5 秒 toast 很容易被错过。
 *
 * 这个 hook 挂在常驻的 `AppShell` 上，只做状态同步：
 *   · `running=true`（含重连成功）→ 清空 asrStopReason（恢复「实时转写中」）；
 *   · `running=false` + reason → 记录原因，界面改成「▲ 识别已停止（积分不足）」；
 *   · 开始/停止录音 → 复位（新一场不该背上一场的状态）。
 *
 * ⚠️ 纯显示态：不碰录音、不碰音频、不碰落盘。识别停止后录音照常继续，
 * 用户仍可用历史页的离线识别补齐。
 */
export function useTranscriptionStatus() {
  useEffect(() => {
    let disposed = false
    let unlisteners: Array<() => void> = []

    const register = async () => {
      const us = await Promise.all([
        onTranscriptionStatus((s) => {
          const { setAsrStopReason: set } = useAppStore.getState()
          if (s?.running) {
            set(null)
            return
          }
          const reason = s?.reason === 'credits' || s?.reason === 'config' || s?.reason === 'unavailable'
            ? s.reason
            : 'ended'
          // 「正常结束」（录音停止）不提示——那时胶囊本来就消失了
          set(reason === 'ended' ? null : reason)
        }),
        onRecordingStarted(() => useAppStore.getState().setAsrStopReason(null)),
        onRecordingStopped(() => useAppStore.getState().setAsrStopReason(null)),
      ])
      if (disposed) {
        us.forEach((u) => {
          try { u() } catch {}
        })
        return
      }
      unlisteners = us
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
