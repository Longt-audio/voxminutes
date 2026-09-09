'use client'

import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import { RefreshCw } from 'lucide-react'
import {
  getRemoteUsage,
  redeemCode,
  registerDevice,
  getRemoteLedger,
  getRemoteUsageByModel,
  getRemoteConfig,
  submitFeedback,
  fetchRemoteMessages,
  type RemoteMessage,
  type LedgerItem,
  type ModelUsageItem,
} from '@/services/ipc'
import { RemoteServiceSection } from '@/components/settings/RemoteServiceSection'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

const LEDGER_TYPE_LABEL: Record<string, string> = {
  gift: '赠送',
  redeem: '兑换',
  consume: '消费',
  adjust: '调整',
  refund: '退款',
  expire: '过期',
}

/** 用户中心：积分余额（+刷新+预警） + 充值兑换 + 积分明细 + 远程服务 + 信息区 + 反馈。 */
export default function AccountPage() {
  const t = useMessages()
  const [usage, setUsage] = useState<{ name: string; credits: number; low_balance?: boolean } | null>(null)
  const [license, setLicense] = useState('')
  const [ledger, setLedger] = useState<LedgerItem[]>([])
  const [modelUsage, setModelUsage] = useState<ModelUsageItem[]>([])
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [feedback, setFeedback] = useState('')
  const [screenshot, setScreenshot] = useState<string | null>(null)
  const [contact, setContact] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  const [redeemInput, setRedeemInput] = useState('')
  const [redeeming, setRedeeming] = useState(false)
  const [registering, setRegistering] = useState(false)
  const [refreshKey, setRefreshKey] = useState(0)

  const load = useCallback(() => {
    getRemoteUsage()
      .then((u) => setUsage({ name: u.name, credits: u.credits, low_balance: u.low_balance }))
      .catch(() => setUsage(null))
    getRemoteConfig()
      .then((c) => setLicense(c.license || ''))
      .catch(() => setLicense(''))
    getRemoteLedger()
      .then((r) => setLedger(r.items || []))
      .catch(() => setLedger([]))
    getRemoteUsageByModel()
      .then((r) => setModelUsage(r.items || []))
      .catch(() => setModelUsage([]))
    fetchRemoteMessages()
      .then((r) => setMessages(r.announcements || []))
      .catch(() => setMessages([]))
  }, [])

  useEffect(() => {
    load()
  }, [load, refreshKey])

  const handleRefresh = useCallback(() => {
    setRefreshing(true)
    load()
    setRefreshKey((k) => k + 1)
    setTimeout(() => setRefreshing(false), 400)
  }, [load])

  const handleRedeem = useCallback(async () => {
    const code = redeemInput.trim()
    if (!code) {
      toast.error(t.accRedeemPlaceholder)
      return
    }
    setRedeeming(true)
    try {
      const r = await redeemCode(code)
      toast.success(t.accRedeemSuccess.replace('{added}', String(r.added)))
      setRedeemInput('')
      handleRefresh()
    } catch (e) {
      toast.error(String(e))
    } finally {
      setRedeeming(false)
    }
  }, [redeemInput, t, handleRefresh])

  const handleRegister = useCallback(async () => {
    setRegistering(true)
    try {
      const r = await registerDevice()
      toast.success(t.accRegisterSuccess.replace('{credits}', String(r.credits)))
      handleRefresh()
    } catch (e) {
      toast.error(String(e))
    } finally {
      setRegistering(false)
    }
  }, [t, handleRefresh])

  const handleScreenshot = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => setScreenshot(reader.result as string)
    reader.readAsDataURL(file)
  }

  const handleSubmit = async () => {
    if (!feedback.trim()) {
      toast.error(t.accFeedbackEmpty)
      return
    }
    setSubmitting(true)
    try {
      await submitFeedback(feedback.trim(), screenshot, contact.trim() || null)
      toast.success(t.accFeedbackSent)
      setFeedback('')
      setScreenshot(null)
      setContact('')
    } catch (e) {
      toast.error(t.accFeedbackFailed.replace('{error}', String(e)))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <h1 className="text-lg font-semibold">{t.accTitle}</h1>

      <div className="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
        {/* 左列：积分 + 充值兑换 + 积分明细 + 使用说明 + 反馈 */}
        <div className="flex flex-col gap-4">
          <div className="rounded-md border border-border/60 p-4">
            <div className="flex items-center justify-between">
              <div className="text-xs text-muted-foreground">{t.accCredits}</div>
              <Button variant="ghost" size="sm" className="h-6 gap-1 px-2 text-xs" disabled={refreshing} onClick={handleRefresh}>
                <RefreshCw className={`h-3 w-3 ${refreshing ? 'animate-spin' : ''}`} />
                {t.accRefresh}
              </Button>
            </div>
            <div className="mt-1 text-2xl font-semibold tabular-nums">
              {usage ? usage.credits : '—'}
            </div>
            {usage?.name && <div className="mt-1 text-xs text-muted-foreground">{usage.name}</div>}
            {usage?.low_balance && (
              <div className="mt-2 rounded bg-red-500/10 px-2 py-1.5 text-xs text-red-600">
                {t.accLowBalance.replace('{credits}', String(usage.credits))}
              </div>
            )}
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="flex items-center justify-between">
              <div className="text-sm font-medium">我的授权码</div>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-xs"
                onClick={() => {
                  navigator.clipboard?.writeText(license).then(() => toast.success('已复制授权码'))
                }}
              >
                复制
              </Button>
            </div>
            <div className="mt-1 break-all text-xs text-muted-foreground">{license || '未配置'}</div>
            <p className="mt-1 text-xs text-muted-foreground/70">授权码是你的账号，充值/找回都靠它，建议自行备份。</p>
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">{t.accRedeemTitle}</div>
            <div className="mt-2 flex items-center gap-2">
              <input
                className="h-8 flex-1 rounded-md border border-input bg-background px-2 text-sm"
                value={redeemInput}
                onChange={(e) => setRedeemInput(e.target.value)}
                placeholder={t.accRedeemPlaceholder}
                onKeyDown={(e) => e.key === 'Enter' && handleRedeem()}
              />
              <Button size="sm" disabled={redeeming} onClick={handleRedeem}>
                {t.accRedeemBtn}
              </Button>
            </div>
            <div className="mt-2 flex items-center gap-2">
              <Button variant="outline" size="sm" disabled={registering} onClick={handleRegister}>
                {t.accRegisterBtn}
              </Button>
              <span className="text-xs text-muted-foreground">首次使用请先点此领取，绑定本机设备</span>
            </div>
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">{t.accLedgerTitle}</div>
            {ledger.length === 0 ? (
              <p className="mt-1 text-xs text-muted-foreground">{t.accLedgerEmpty}</p>
            ) : (
              <div className="mt-2 flex flex-col gap-1">
                {ledger.slice(0, 20).map((l) => (
                  <div key={l.id} className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground">{l.created_at}</span>
                    <span className="flex-1 px-2 text-muted-foreground">
                      {LEDGER_TYPE_LABEL[l.type] || l.type}
                      {l.remark ? ` · ${l.remark}` : ''}
                    </span>
                    <span className={l.amount >= 0 ? 'text-green-600' : 'text-red-500'}>
                      {l.amount >= 0 ? '+' : ''}
                      {l.amount}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">按模型消耗</div>
            {modelUsage.length === 0 ? (
              <p className="mt-1 text-xs text-muted-foreground">暂无消耗记录</p>
            ) : (
              <div className="mt-2 flex flex-col gap-1">
                {modelUsage.map((m) => (
                  <div key={m.model} className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground">
                      {m.model}
                      <span className="text-muted-foreground/60"> · {m.kind === 'asr' ? '转写' : m.kind === 'tts' ? '语音' : '翻译'} · {m.calls} 次</span>
                    </span>
                    <span className="text-red-500">-{m.credits.toFixed(2)} 积分</span>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">{t.accBillingTitle}</div>
            <p className="mt-1 text-xs text-muted-foreground">{t.accBillingHint}</p>
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">{t.accFeedback}</div>
            <textarea
              className="mt-2 h-24 w-full rounded-md border border-input bg-background p-2 text-sm"
              value={feedback}
              onChange={(e) => setFeedback(e.target.value)}
              placeholder={t.accFeedbackPlaceholder}
            />
            <div className="mt-2 flex items-center gap-2 text-xs">
              <input type="file" accept="image/*" onChange={handleScreenshot} />
              {screenshot && <span className="text-muted-foreground">{t.accScreenshotAttached}</span>}
            </div>
            <input
              className="mt-2 w-full rounded-md border border-input bg-background p-2 text-sm"
              value={contact}
              onChange={(e) => setContact(e.target.value)}
              placeholder={t.accContact}
            />
            <Button className="mt-3" disabled={submitting} onClick={handleSubmit}>
              {submitting ? t.comLoading : t.accSubmit}
            </Button>
            <p className="mt-3 text-xs text-muted-foreground/80">{t.accLogHint}</p>
          </div>
        </div>

        {/* 右列：远程服务 + 信息区 */}
        <div className="flex flex-col gap-4">
          <RemoteServiceSection key={`remote-${refreshKey}`} onChanged={handleRefresh} />
          {messages.length > 0 && (
            <div className="rounded-md border border-border/60 p-4">
              <div className="text-sm font-medium">{t.accInfo}</div>
              <div className="mt-2 flex flex-col gap-2">
                {messages.map((m) => (
                  <div key={m.id} className="text-xs">
                    <span className="font-medium">{m.title}</span>
                    {m.body && <span className="text-muted-foreground"> · {m.body}</span>}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
