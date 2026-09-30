'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { X, Rocket } from 'lucide-react'
import { fetchRemoteMessages, openExternalUrl, type RemoteLatestVersion } from '@/services/ipc'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { LATEST_DOWNLOAD_URL } from '@/lib/site'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { pickLangSegment } from '@/lib/langSegment'

/** 轮询间隔（毫秒）：客户端定时向网关拉取最新版本号，版本更高才显示横幅。 */
const POLL_INTERVAL_MS = 60_000

/** localStorage 键：用户确认「不再提示」后彻底禁用更新横幅 */
const NEVER_REMIND_KEY = 'vox:updateBanner.neverRemind'

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
 *  逻辑：后台「版本更新」发布版本号 → 客户端每 60s（+启动时）拉取 → 更高才显示。
 *  关闭 X：本次会话内记住该版本号（新版本出现会再次显示）；
 *  「不再提示」：弹确认框，确认后写 localStorage 彻底禁用更新横幅。 */
export function UpdateBanner() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const [latest, setLatest] = useState<RemoteLatestVersion | null>(null)
  const [current, setCurrent] = useState('')
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(null)
  // 「不再提示」彻底禁用（localStorage 持久化，挂载后读取避免 SSR/水合不一致）
  const [neverRemind, setNeverRemind] = useState(false)
  const [neverConfirmOpen, setNeverConfirmOpen] = useState(false)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    try {
      setNeverRemind(localStorage.getItem(NEVER_REMIND_KEY) === '1')
    } catch {
      // localStorage 不可用时按未禁用处理
    }
  }, [])

  const poll = useCallback(async (cur: string) => {
    try {
      const res = await fetchRemoteMessages(lang)
      const lv = res.latestVersion
      if (lv && cur && isNewer(lv.version, cur)) {
        setLatest(lv)
      }
    } catch {
      // 网关未配置/不可达：静默忽略
    }
  }, [lang])

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

  // 确认「不再提示」：持久化禁用，之后任何新版本都不再提醒
  const handleNeverRemind = () => {
    try {
      localStorage.setItem(NEVER_REMIND_KEY, '1')
    } catch {
      // 写入失败也至少在本次会话内禁用
    }
    setNeverRemind(true)
    setNeverConfirmOpen(false)
  }

  // 已关闭的版本不再显示；新版本出现则重新显示；「不再提示」后彻底不显示
  const show =
    !neverRemind && latest && current && isNewer(latest.version, current) && dismissedVersion !== latest.version
  if (!show || !latest) return null

  return (
    <>
      <div className="flex shrink-0 items-center gap-2 border-b border-emerald-300 bg-emerald-50 px-4 py-2 text-sm text-emerald-900">
        <Rocket className="h-4 w-4 shrink-0" />
        <span className="flex-1 min-w-0">
          <span className="font-medium">{t.updNewVersion.replace('{version}', latest.version)}</span>
          <span className="text-emerald-800/90">{t.updCurrentVersion.replace('{version}', current)}</span>
          {latest.release_notes && (
            <span className="text-emerald-800/80"> · {pickLangSegment(latest.release_notes, lang)}</span>
          )}
        </span>
        {(latest.download_url || LATEST_DOWNLOAD_URL) && (
          // 必须走 Rust 的 open_external_url：WebView2 / WKWebView 里
          // `<a target="_blank">` 会触发 NewWindowRequested，而 Tauri 没有注册
          // on_new_window 处理器 → 请求被静默取消（点了完全没反应）。
          <button
            type="button"
            className="shrink-0 whitespace-nowrap font-medium underline underline-offset-2 hover:text-emerald-700"
            onClick={() => void openExternalUrl(latest.download_url || LATEST_DOWNLOAD_URL).catch(() => {})}
          >
            {t.comDownload}
          </button>
        )}
        {/* 彻底禁用更新提醒（弱化小字，点击需二次确认，避免误点）；
            whitespace-nowrap：英文 "Don't show again" 较长，换行会被 py-2 的行高截断 */}
        <button
          type="button"
          className="shrink-0 whitespace-nowrap text-xs text-emerald-800/50 underline-offset-2 hover:text-emerald-800/80 hover:underline"
          onClick={() => setNeverConfirmOpen(true)}
        >
          {t.comNeverRemind}
        </button>
        <button
          className="shrink-0 opacity-60 hover:opacity-100"
          onClick={() => setDismissedVersion(latest.version)}
          title={t.comClose}
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {/* 「不再提示」二次确认：说明关闭后将不再提醒任何新版本 */}
      <Dialog open={neverConfirmOpen} onOpenChange={setNeverConfirmOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="text-base">{t.updNeverTitle}</DialogTitle>
            <DialogDescription>{t.updNeverDesc}</DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2 pt-1">
            <Button variant="ghost" size="sm" onClick={() => setNeverConfirmOpen(false)}>
              {t.comCancel}
            </Button>
            <Button size="sm" onClick={handleNeverRemind}>
              {t.comConfirm}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
