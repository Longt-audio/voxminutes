'use client'

import { useEffect, useMemo, useState } from 'react'
import { X } from 'lucide-react'
import { fetchRemoteMessages, type RemoteMessage } from '@/services/ipc'

interface FooterItem {
  id: string
  title: string
  body: string
  url?: string
}

/** 底部推送条：无推送时显示默认文案，有推送时轮播 tips/公告（可关闭）。
 * 更新提醒已由页面顶部的 UpdateBanner 承担。 */
export function RemoteMessages({ defaultText }: { defaultText: string }) {
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
    return messages
      .filter((m) => !dismissed.has(m.id))
      .map((m) => ({ id: m.id, title: m.title, body: m.body }))
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
