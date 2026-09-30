'use client'

// 离线（重）识别事件监听：**全局只注册一次**（AppShell 挂载）。
//
// 见 stores/retranscriptionStore.ts 顶部注释：过去监听挂在 /history 页面里，
// 切页面即丢失 → 任务其实还在跑，但用户看不到进度、完成也没有任何反馈，
// 体验上等同「切换页面就中断了」。放到 AppShell 后任务生命周期与页面解耦。

import { useEffect } from 'react'
import { toast } from 'sonner'
import {
  onRetranscriptionCancelled,
  onRetranscriptionComplete,
  onRetranscriptionError,
  onRetranscriptionPartial,
  onRetranscriptionProgress,
} from '@/services/ipc'
import { useRetranscriptionStore } from '@/stores/retranscriptionStore'
import { useMessages } from '@/i18n/useMessages'

export function useRetranscriptionEvents() {
  const t = useMessages()

  useEffect(() => {
    let disposed = false
    let unlistens: Array<() => void> = []

    Promise.all([
      onRetranscriptionProgress((p) => useRetranscriptionStore.getState().setProgress(p)),
      onRetranscriptionPartial((p) => useRetranscriptionStore.getState().setPartial(p)),
      onRetranscriptionComplete((r) => {
        useRetranscriptionStore.getState().finish(r)
        const warnings = r.warnings ?? []
        if (warnings.length > 0) {
          // 有上游告警（内容风控部分拦截 / 档位降级 / 部分分片失败）时用**警告**样式提示，
          // 并把原因写清楚 —— 否则用户会把「结果不完整」当成软件故障（2026-09-22 用户要求）。
          toast.warning(t.histRetranscribeDoneWithWarnings.replace('{count}', String(r.segments_count)), {
            description: warnings.join('\n'),
            duration: 15000,
          })
        } else {
          toast.success(t.histRetranscribeDone.replace('{count}', String(r.segments_count)))
        }
      }),
      onRetranscriptionError((e) => {
        useRetranscriptionStore.getState().conclude()
        toast.error(t.histRetranscribeFailed.replace('{error}', e.error))
      }),
      // 用户主动「停止识别」：后端已中断在途请求（不是故障）
      onRetranscriptionCancelled(() => {
        useRetranscriptionStore.getState().conclude()
        toast.success(t.histRetranscribeStopped)
      }),
    ]).then((fns) => {
      if (disposed) fns.forEach((f) => f())
      else unlistens = fns
    })

    return () => {
      disposed = true
      unlistens.forEach((f) => f())
    }
  }, [t])
}
