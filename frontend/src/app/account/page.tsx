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
import { RemoteServiceSection } from '@/components/settings/RemoteServiceSection'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 用户中心：积分余额（+刷新） + 积分使用说明 + 远程服务 + 信息区 + 反馈。
 *  远程服务区域（改名「远程服务」）放在积分显示区域下方；模型只展示不选择，
 *  选择请到各功能使用处（录制对话框 / 翻译页 / 会议总结）。 */
export default function AccountPage() {
  const t = useMessages()
  const [usage, setUsage] = useState<{ name: string; credits: number } | null>(null)
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [feedback, setFeedback] = useState('')
  const [screenshot, setScreenshot] = useState<string | null>(null)
  const [contact, setContact] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
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
  }, [load, refreshKey])

  const handleRefresh = useCallback(() => {
    setRefreshing(true)
    load()
    setRefreshKey((k) => k + 1)
    setTimeout(() => setRefreshing(false), 400)
  }, [load])

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
        {/* 左列：积分（+刷新）+ 积分使用说明 + 反馈 */}
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

        {/* 右列：远程服务（积分下方区域）+ 信息区 */}
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
