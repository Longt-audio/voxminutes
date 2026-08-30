'use client'

import { useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { X, Rocket } from 'lucide-react'
import { fetchRemoteMessages, type RemoteLatestVersion } from '@/services/ipc'

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

/** 软件更新提醒横幅：远程 latest-version 高于当前版本时，显示在页面最上方。 */
export function UpdateBanner() {
  const [latest, setLatest] = useState<RemoteLatestVersion | null>(null)
  const [current, setCurrent] = useState('')
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    ;(async () => {
      try {
        setCurrent(await getVersion())
      } catch {}
      try {
        const res = await fetchRemoteMessages()
        setLatest(res.latestVersion)
      } catch {
        // 网关未配置/不可达：静默忽略
      }
    })()
  }, [])

  const show = latest && !dismissed && current && isNewer(latest.version, current)
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
      <button className="shrink-0 opacity-60 hover:opacity-100" onClick={() => setDismissed(true)} title="关闭">
        <X className="h-4 w-4" />
      </button>
    </div>
  )
}
