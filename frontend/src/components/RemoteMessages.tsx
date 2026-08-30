'use client'

import { useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { X, Rocket, Lightbulb, Megaphone } from 'lucide-react'
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

function severityClass(severity: string): string {
  switch (severity) {
    case 'success': return 'border-emerald-300 bg-emerald-50 text-emerald-800'
    case 'warning': return 'border-amber-300 bg-amber-50 text-amber-800'
    case 'error': return 'border-red-300 bg-red-50 text-red-800'
    default: return 'border-blue-200 bg-blue-50 text-blue-800'
  }
}

function iconFor(type: string) {
  if (type === 'announcement') return <Megaphone className="h-3.5 w-3.5 shrink-0" />
  if (type === 'update') return <Rocket className="h-3.5 w-3.5 shrink-0" />
  return <Lightbulb className="h-3.5 w-3.5 shrink-0" />
}

/** 远程推送消息：更新提醒 + 使用 tips/公告（来自网关，未配置时静默隐藏）。 */
export function RemoteMessages() {
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [latest, setLatest] = useState<RemoteLatestVersion | null>(null)
  const [current, setCurrent] = useState('')
  const [dismissed, setDismissed] = useState<Set<string>>(new Set())
  const [updateDismissed, setUpdateDismissed] = useState(false)

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

  const showUpdate = latest && !updateDismissed && current && isNewer(latest.version, current)
  const visibleTips = messages.filter((m) => !dismissed.has(m.id))

  if (!showUpdate && visibleTips.length === 0) return null

  return (
    <div className="flex flex-col gap-2">
      {showUpdate && latest && (
        <div className="flex items-start gap-2 rounded-md border border-emerald-300 bg-emerald-50 px-3 py-2 text-xs text-emerald-800">
          <Rocket className="h-3.5 w-3.5 mt-0.5 shrink-0" />
          <div className="flex-1 min-w-0">
            <p className="font-medium">
              新版本 v{latest.version} 可用（当前 v{current}）
            </p>
            {latest.release_notes && <p className="mt-0.5 text-emerald-700/90">{latest.release_notes}</p>}
            {latest.download_url && (
              <a
                className="mt-1 inline-block underline underline-offset-2 hover:text-emerald-900"
                href={latest.download_url}
                target="_blank"
                rel="noreferrer"
              >
                下载
              </a>
            )}
          </div>
          <button className="shrink-0 opacity-60 hover:opacity-100" onClick={() => setUpdateDismissed(true)}>
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      {visibleTips.map((m) => (
        <div key={m.id} className={`flex items-start gap-2 rounded-md border px-3 py-2 text-xs ${severityClass(m.severity)}`}>
          <span className="mt-0.5">{iconFor(m.type)}</span>
          <div className="flex-1 min-w-0">
            <p className="font-medium">{m.title}</p>
            <p className="mt-0.5 opacity-90">{m.body}</p>
          </div>
          {m.dismissible !== false && (
            <button
              className="shrink-0 opacity-60 hover:opacity-100"
              onClick={() => setDismissed((prev) => new Set(prev).add(m.id))}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      ))}
    </div>
  )
}
