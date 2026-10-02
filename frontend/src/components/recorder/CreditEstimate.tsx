'use client'

import { useEffect, useState } from 'react'
import { normalizeAsrModelName, useAsrModelOptions } from '@/components/models/AsrModelPicker'
import {
  creditsPerSecond,
  estimateRecordableSeconds,
  formatRecordableDuration,
  getCachedCredits,
  minimumCreditsToStart,
} from '@/lib/creditsEstimate'
import { useMessages } from '@/i18n/useMessages'
import { cn } from '@/lib/utils'

/**
 * 「余额 xx 积分 · 约可录 x 小时 y 分钟」+ 口径说明。
 *
 * 2026-10-02 从 `AsrModelPicker` 里**搬出来**（原来贴在模型列表下方）：
 * 用户反馈那块位置太靠上、和"能不能开始录音"这件事离得远，
 * 要求在**开始录音按钮同一行的左侧空白处**显示 —— 一眼就能看到"这点余额够录多久"。
 *
 * 做成独立组件而不是把状态往上提，是为了保持 `AsrModelPicker` 干净
 * （它还要给历史页重识别弹窗复用，那边不需要这个提示）。
 *
 * 口径（严格复刻网关的冻结规则，见 `lib/creditsEstimate.ts`）：
 * 先冻 2 分钟额度、之后每 60 秒阶梯补冻，所以预估是**保守**的。
 * 余额不足 2 分钟额度时明确提示「无法开始」—— 那种情况网关建会话就会被拒，
 * 整场都不会有字幕，不如提前说清楚。
 */
export function CreditEstimate({ value }: { value: string }) {
  const t = useMessages()
  const { remoteAsr, remoteEnabled } = useAsrModelOptions('realtime')
  const remoteSelected = normalizeAsrModelName(value) === 'remote'
  const selectedRemote = remoteAsr.models.find((m: { id: string }) => m.id === remoteAsr.value)
  const [credits, setCredits] = useState<number | null>(null)

  useEffect(() => {
    // 只在「实时转录 + 确实选了远程模型」时才查余额；拉不到就整块不渲染 ——
    // 绝不因为拿不到余额而影响开始录音（余额走 60s 缓存，getRemoteUsage 每次实打网关）。
    if (!remoteSelected || !remoteEnabled) return
    let alive = true
    getCachedCredits()
      .then((c) => {
        if (alive) setCredits(c)
      })
      .catch(() => {})
    return () => {
      alive = false
    }
  }, [remoteSelected, remoteEnabled, remoteAsr.value])

  if (!remoteSelected || !remoteEnabled || credits === null) return null
  const cps = selectedRemote ? creditsPerSecond(selectedRemote) : null
  if (cps === null) return null

  const seconds = estimateRecordableSeconds(credits, cps)
  const tooLow = seconds <= 0

  return (
    <div className="min-w-0 space-y-0.5 text-right">
      <p
        className={cn(
          'truncate text-[11px] leading-relaxed',
          tooLow ? 'font-medium text-red-600 dark:text-red-400' : 'text-muted-foreground'
        )}
        title={
          tooLow
            ? t.recCreditTooLow.replace('{min}', String(minimumCreditsToStart(cps)))
            : t.recCreditEstimate
                .replace('{credits}', String(Number(credits.toFixed(2))))
                .replace('{duration}', formatRecordableDuration(seconds, t))
        }
      >
        {tooLow
          ? t.recCreditTooLow.replace('{min}', String(minimumCreditsToStart(cps)))
          : t.recCreditEstimate
              .replace('{credits}', String(Number(credits.toFixed(2))))
              .replace('{duration}', formatRecordableDuration(seconds, t))}
      </p>
      {/* 口径说明：窄屏（flex-wrap 换行后）会被挤没，所以用 hidden + sm:block 控制，
          并把超长文案截断 —— 完整内容在 title 里，鼠标悬停可见。 */}
      <p className="hidden truncate text-[10px] leading-relaxed text-muted-foreground/70 sm:block">
        {t.recCreditEstimateHint}
      </p>
    </div>
  )
}
