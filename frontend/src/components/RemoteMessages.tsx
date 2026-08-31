'use client'

import { useEffect, useMemo, useState } from 'react'
import { X } from 'lucide-react'
import { fetchRemoteMessages, type RemoteMessage } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'

interface FooterItem {
  id: string
  title: string
  body: string
  type: string
  url?: string
}

/** type → 颜色：tip 蓝 / announcement 绿 / update 黄（三类推送消息的语义区分）。 */
function typeColor(type: string): string {
  if (type === 'announcement') return 'text-emerald-600'
  if (type === 'update') return 'text-amber-600'
  return 'text-blue-600' // tip
}

function typeLabel(type: string, t: ReturnType<typeof useMessages>): string {
  if (type === 'announcement') return t.msgTypeAnnouncement
  if (type === 'update') return t.msgTypeUpdate
  return t.msgTypeTip
}

/** 底部推送条：无推送时显示默认文案，有推送时轮播「底部短信息」（channel=footer）。
 *  启动时长信息（channel=startup）由 StartupNoticeDialog 承担；更新提醒由顶部 UpdateBanner 承担。 */
export function RemoteMessages({ defaultText }: { defaultText: string }) {
  const t = useMessages()
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [dismissed, setDismissed] = useState<Set<string>>(new Set())
  const [index, setIndex] = useState(0)

  useEffect(() => {
    fetchRemoteMessages()
      .then((r) => setMessages(r.announcements || []))
      .catch(() => {
        // 网关未配置/不可达：静默忽略
      })
  }, [])

  const items = useMemo<FooterItem[]>(() => {
    // 只显示「底部短信息」渠道（channel 缺省视为 footer，兼容旧数据）
    return messages
      .filter((m) => (m.channel || 'footer') === 'footer')
      .filter((m) => !dismissed.has(m.id))
      .map((m) => ({ id: m.id, title: m.title, body: m.body, type: m.type }))
  }, [messages, dismissed])

  // 轮播：多则每 5 秒切一条
  useEffect(() => {
    if (items.length <= 1) {
      setIndex(0)
      return
    }
    const timer = setInterval(() => setIndex((i) => (i + 1) % items.length), 5000)
    return () => clearInterval(timer)
  }, [items.length])

  const item = items.length > 0 ? items[Math.min(index, items.length - 1)] : null

  if (!item) {
    return <span className="text-[10px] text-muted-foreground/60">{defaultText}</span>
  }

  const dismiss = () => setDismissed((prev) => new Set(prev).add(item.id))

  return (
    <span className="flex min-w-0 items-center gap-2 text-[10px] text-muted-foreground/80">
      <span className={`shrink-0 font-medium ${typeColor(item.type)}`}>[{typeLabel(item.type, t)}]</span>
      <span className="truncate">
        <span className="font-medium">{item.title}</span>
        {item.body && <span> · {item.body}</span>}
        {item.url && (
          <a
            className="ml-1 underline underline-offset-2 hover:text-foreground"
            href={item.url}
            target="_blank"
            rel="noreferrer"
          >
            下载
          </a>
        )}
      </span>
      <button className="shrink-0 opacity-50 hover:opacity-100" onClick={dismiss} title="关闭">
        <X className="h-3 w-3" />
      </button>
    </span>
  )
}
