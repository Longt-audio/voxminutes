'use client'

import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import { RefreshCw } from 'lucide-react'
import {
  getRemoteUsage,
  submitFeedback,
  fetchRemoteMessages,
  type RemoteMessage,
} from '@/services/ipc'
import { RemoteAsrSection } from '@/components/settings/RemoteAsrSection'
import { SummarySection } from '@/components/settings/SummarySection'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 用户中心：积分余额 + 远程服务设置 + 信息区 + 反馈（文字/截图）。
 *  顶部「刷新」按钮重拉积分/模型/消息，让后台改动的模型与价格立即生效。 */
export default function AccountPage() {
  const t = useMessages()
  const [usage, setUsage] = useState<{ name: string; credits: number } | null>(null)
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [feedback, setFeedback] = useState('')
  const [screenshot, setScreenshot] = useState<string | null>(null)
  const [contact, setContact] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  // 刷新计数：作为 Key 传给两个设置区，触发它们重新挂载并重拉模型列表
  const [refreshKey, setRefreshKey] = useState(0)

  const load = useCallback(() => {
    getRemoteUsage()
      .then((u) => setUsage({ name: u.name, credits: u.credits }))
      .catch(() => setUsage(null))
    fetchRemoteMessages()
      .then((r) => setMessages(r.announcements || []))
      .catch(() => setMessages([]))
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      load()
      setRefreshKey((k) => k + 1)
      toast.success(t.accRefreshed)
    } finally {
      setRefreshing(false)
    }
  }

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
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">{t.accTitle}</h1>
        <Button variant="outline" size="sm" className="gap-2" disabled={refreshing} onClick={handleRefresh}>
          <RefreshCw className={`h-3.5 w-3.5 ${refreshing ? 'animate-spin' : ''}`} />
          {t.accRefresh}
        </Button>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
        {/* 左列：积分 + 反馈 */}
        <div className="flex flex-col gap-4">
          <div className="rounded-md border border-border/60 p-4">
            <div className="text-xs text-muted-foreground">{t.accCredits}</div>
            <div className="mt-1 text-2xl font-semibold tabular-nums">
              {usage ? usage.credits : '—'}
            </div>
            {usage?.name && <div className="mt-1 text-xs text-muted-foreground">{usage.name}</div>}
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

        {/* 右列：信息区 + 远程服务 */}
        <div className="flex flex-col gap-4">
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
          <SummarySection key={`summary-${refreshKey}`} />
          <RemoteAsrSection key={`remote-${refreshKey}`} />
        </div>
      </div>
    </div>
  )
}
