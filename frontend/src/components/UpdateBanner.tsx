'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { X, Rocket } from 'lucide-react'
import { fetchRemoteMessages, type RemoteLatestVersion } from '@/services/ipc'

/** 轮询间隔（毫秒）：客户端定时向网关拉取最新版本号，版本更高才显示横幅。 */
const POLL_INTERVAL_MS = 60_000

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

/** 软件更新提醒横幅：客户端定时轮询网关 latest-version，高于当前版本时显示在页面最上方。
 *  逻辑：后台「版本更新」发布版本号 → 客户端每 60s（+启动时）拉取 → 更高才显示，可关闭（本次会话内记住）。 */
export function UpdateBanner() {
  const [latest, setLatest] = useState<RemoteLatestVersion | null>(null)
  const [current, setCurrent] = useState('')
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(null)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const poll = useCallback(async (cur: string) => {
    try {
      const res = await fetchRemoteMessages()
      const lv = res.latestVersion
      if (lv && cur && isNewer(lv.version, cur)) {
        setLatest(lv)
      }
    } catch {
      // 网关未配置/不可达：静默忽略
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      let cur = ''
      try {
        cur = await getVersion()
        if (!cancelled) setCurrent(cur)
      } catch {}
      // 启动时先拉一次
      if (cur) void poll(cur)
      // 定时轮询
      timerRef.current = setInterval(() => {
        if (cur) void poll(cur)
      }, POLL_INTERVAL_MS)
    })()
    return () => {
      cancelled = true
      if (timerRef.current) clearInterval(timerRef.current)
    }
  }, [poll])

  // 已关闭的版本不再显示；新版本出现则重新显示
  const show = latest && current && isNewer(latest.version, current) && dismissedVersion !== latest.version
  if (!show || !latest) return null

  return (
    <div className="flex shrink-0 items-center gap-2 border-b border-emerald-300 bg-emerald-50 px-4 py-2 text-sm text-emerald-900">
      <Rocket className="h-4 w-4 shrink-0" />
      <span className="flex-1 min-w-0">
        <span className="font-medium">新版本 v{latest.version} 可用</span>
        <span className="text-emerald-800/90">（当前 v{current}）</span>
        {latest.release_notes && <span className="text-emerald-800/80"> · {latest.release_notes}</span>}
      </span>
      {latest.download_url && (
        <a
          className="shrink-0 font-medium underline underline-offset-2 hover:text-emerald-700"
          href={latest.download_url}
          target="_blank"
          rel="noreferrer"
        >
          下载
        </a>
      )}
      <button
        className="shrink-0 opacity-60 hover:opacity-100"
        onClick={() => setDismissedVersion(latest.version)}
        title="关闭"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  )
}
