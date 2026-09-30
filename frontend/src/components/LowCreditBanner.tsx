'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { useRouter } from 'next/navigation'
import { AlertTriangle, X } from 'lucide-react'
import { getRemoteConfig, getRemoteEnabled, getRemoteUsage } from '@/services/ipc'
import { onRemoteConfigChanged } from '@/lib/remoteConfigSync'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 积分预警阈值兜底值（与网关 CREDIT_WARNING_THRESHOLD_MC = 20 积分一致；
 *  正常以网关 /v1/usage 下发的 threshold / low_balance 为准） */
const DEFAULT_THRESHOLD = 20

/** 轮询间隔（毫秒）：余额回升后自动隐藏横幅 / 解除「不再提示」 */
const POLL_INTERVAL_MS = 60_000

/** localStorage 键：「不再提示」禁用标记，值为禁用时的余额（JSON）。
 *  拉到余额 > 阈值即清除——用户充值后自动恢复提醒。 */
const MUTE_KEY = 'vox:lowCreditBanner.muted'

/** 积分不足顶部提醒横幅：启动后（远程服务已启用且有授权码）拉一次余额，低于阈值即显示。
 *  - 文案复用用户中心同款 accLowBalance，点击跳用户中心。
 *  - 关闭 X：本次会话不再显示；「不再提示」：确认后写 localStorage 持久化禁用，
 *    余额回升到阈值以上（用户充值）时自动解除禁用，下次低于阈值仍会提醒。
 *  - 与 UpdateBanner 一样在正常文档流中，同时存在时上下堆叠不重叠。 */
export function LowCreditBanner() {
  const t = useMessages()
  const router = useRouter()
  // null = 尚未拉到有效余额（未启用远程/无授权码/查询失败一律不显示）
  const [balance, setBalance] = useState<number | null>(null)
  const [low, setLow] = useState(false)
  // 关闭 X：本次会话不再显示
  const [dismissed, setDismissed] = useState(false)
  // 「不再提示」持久化禁用（挂载后读 localStorage，避免 SSR/水合不一致）
  const [muted, setMuted] = useState(false)
  const [muteConfirmOpen, setMuteConfirmOpen] = useState(false)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    try {
      setMuted(!!localStorage.getItem(MUTE_KEY))
    } catch {
      // localStorage 不可用时按未禁用处理
    }
  }, [])

  const check = useCallback(async () => {
    try {
      const [enabled, cfg] = await Promise.all([getRemoteEnabled(), getRemoteConfig()])
      if (!enabled || !cfg?.license?.trim()) {
        setBalance(null)
        setLow(false)
        return
      }
      const u = await getRemoteUsage()
      const threshold = u.threshold ?? DEFAULT_THRESHOLD
      // 余额回升到阈值以上 → 自动解除「不再提示」禁用（用户已充值）
      if (u.credits > threshold) {
        try {
          localStorage.removeItem(MUTE_KEY)
        } catch {}
        setMuted(false)
      }
      setBalance(u.credits)
      setLow(u.low_balance ?? u.credits <= threshold)
    } catch {
      // 未配置网关/离线/查询失败：静默，不显示横幅
    }
  }, [])

  // 启动拉一次 + 60s 轮询 + 他处保存远程配置（启用开关/授权码变化）时复查
  useEffect(() => {
    void check()
    timerRef.current = setInterval(() => void check(), POLL_INTERVAL_MS)
    const offSync = onRemoteConfigChanged(() => void check())
    return () => {
      if (timerRef.current) clearInterval(timerRef.current)
      offSync()
    }
  }, [check])

  // 确认「不再提示」：记录禁用时的余额，余额回升后自动解除
  const handleMute = () => {
    try {
      localStorage.setItem(MUTE_KEY, JSON.stringify({ balance }))
    } catch {
      // 写入失败也至少在本次会话内禁用
    }
    setMuted(true)
    setMuteConfirmOpen(false)
  }

  const show = low && balance !== null && !dismissed && !muted
  if (!show) return null

  return (
    <>
      <div className="flex shrink-0 items-center gap-2 border-b border-amber-300 bg-amber-50 px-4 py-2 text-sm text-amber-900">
        <AlertTriangle className="h-4 w-4 shrink-0" />
        <button
          type="button"
          className="flex-1 min-w-0 truncate text-left hover:underline underline-offset-2"
          onClick={() => router.push('/account')}
          title={t.navAccount}
        >
          <span className="font-medium">{t.accLowBalance.replace('{credits}', String(balance))}</span>
        </button>
        <button
          type="button"
          className="shrink-0 font-medium underline underline-offset-2 hover:text-amber-700"
          onClick={() => router.push('/account')}
        >
          {t.lowCreditTopUp}
        </button>
        {/* 彻底禁用提醒（弱化小字，点击需二次确认；余额回升会自动恢复） */}
        <button
          type="button"
          className="shrink-0 text-xs text-amber-800/50 underline-offset-2 hover:text-amber-800/80 hover:underline"
          onClick={() => setMuteConfirmOpen(true)}
        >
          {t.comNeverRemind}
        </button>
        <button className="shrink-0 opacity-60 hover:opacity-100" onClick={() => setDismissed(true)} title={t.comClose}>
          <X className="h-4 w-4" />
        </button>
      </div>

      {/* 「不再提示」二次确认：说明余额回升到预警值以上时会自动恢复提醒 */}
      <Dialog open={muteConfirmOpen} onOpenChange={setMuteConfirmOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="text-base">{t.lowCreditNeverTitle}</DialogTitle>
            <DialogDescription>{t.lowCreditNeverDesc}</DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2 pt-1">
            <Button variant="ghost" size="sm" onClick={() => setMuteConfirmOpen(false)}>
              {t.comCancel}
            </Button>
            <Button size="sm" onClick={handleMute}>
              {t.comConfirm}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
