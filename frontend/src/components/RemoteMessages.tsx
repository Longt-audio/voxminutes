'use client'

import { useEffect, useMemo, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { X } from 'lucide-react'
import { fetchRemoteMessages, type RemoteMessage, type RemoteLatestVersion } from '@/services/ipc'

/** 简单的 semver 比较：a > b 返回 true。 */
function isNewer(a: string, b: string): boolean {
  const pa = a.split('.').map((n) => parseInt(n, 10) || 0)
  const pb = b.split('.').map((n) => parseInt(n, 10) || 0)
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const x = pa[i] || 0
    const y = pb[i] || 0
    if (x !== y) return x > y
  }
  return false
}

interface FooterItem {
  id: string
  title: string
  body: string
  url?: string
}

/** 底部推送条：无推送时显示默认文案，有推送时轮播（更新提醒优先），可关闭。 */
export function RemoteMessages({ defaultText }: { defaultText: string }) {
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [latest, setLatest] = useState<RemoteLatestVersion | null>(null)
  const [current, setCurrent] = useState('')
  const [dismissed, setDismissed] = useState<Set<string>>(new Set())
  const [updateDismissed, setUpdateDismissed] = useState(false)
  const [index, setIndex] = useState(0)

  useEffect(() => {
    ;(async () => {
      try {
        setCurrent(await getVersion())
      } catch {}
      try {
        const res = await fetchRemoteMessages()
        setMessages(res.announcements || [])
        setLatest(res.latestVersion)
      } catch {
        // 网关未配置/不可达：静默忽略
      }
    })()
  }, [])

  const items = useMemo<FooterItem[]>(() => {
    const out: FooterItem[] = []
    const showUpdate = latest && !updateDismissed && current && isNewer(latest.version, current)
    if (showUpdate && latest) {
      out.push({
        id: '__update__',
        title: `新版本 v${latest.version} 可用`,
        body: latest.release_notes || '',
        url: latest.download_url,
      })
    }
    for (const m of messages) {
      if (!dismissed.has(m.id)) out.push({ id: m.id, title: m.title, body: m.body })
    }
    return out
  }, [latest, updateDismissed, current, messages, dismissed])

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

  const dismiss = () => {
    if (item.id === '__update__') {
      setUpdateDismissed(true)
    } else {
      setDismissed((prev) => new Set(prev).add(item.id))
    }
  }

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
