'use client'

import { useCallback, useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { Globe, RefreshCw, Rocket , Download} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { fetchRemoteMessages, openExternalUrl } from '@/services/ipc'
import { pickLangSegment } from '@/lib/langSegment'
import { OFFICIAL_WEBSITE_URL, LATEST_DOWNLOAD_URL, privacyPolicyUrl, termsOfServiceUrl } from '@/lib/site'
import { progressPercent, useAppUpdater } from '@/hooks/useAppUpdater'
import { getBuildInfo } from '@/services/ipc'

// 官网地址已挪到 @/lib/site 共享（欢迎弹窗 P2 也用）；保留再导出兼容旧引用
export { OFFICIAL_WEBSITE_URL }

/** 简单的 semver 比较：a > b 返回 true（与 UpdateBanner 口径一致）。 */
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

/** 设置页「关于我们」：版本号 / 本版本新增功能（占位） / 检查更新 / 访问官网。 */
export function AboutSection() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const [version, setVersion] = useState('')
  const [builtAt, setBuiltAt] = useState<Date | null>(null)

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {})
    // 构建/安装时间：版本号不变时用来分辨「装的是哪一版」
    getBuildInfo()
      .then((b) => {
        if (b.exeModifiedUnix) setBuiltAt(new Date(b.exeModifiedUnix * 1000))
      })
      .catch(() => {})
  }, [])

  // 自动更新（2026-09-30）：走 Tauri updater —— 真正的下载 + 静默安装 + 自动重启。
  // 任何环节失败都能退回到「下载完整安装包」与「去官网」，不会让用户卡死。
  const updater = useAppUpdater()
  const { phase } = updater

  const handleVisitWebsite = useCallback(() => {
    if (OFFICIAL_WEBSITE_URL) {
      // window.open(…, '_blank') 在 Tauri 的 webview 里会被静默取消
      // （没有 on_new_window 处理器 → NewWindowRequested 直接被拒），
      // 统一改走 Rust 的 open_external_url 用系统浏览器打开。
      void openExternalUrl(OFFICIAL_WEBSITE_URL).catch(() => {})
    }
  }, [])

  return (
    <SettingsSection title={t.setAboutTitle} description={t.setAboutDesc}>
      <div className="flex flex-col gap-4">
        {/* 版本号 + 构建/安装时间。
            ⚠️ 时间这一项**必须留着**（2026-10-01 用户要求）：
            版本号长期停留在 0.2.0，光看版本号无法分辨「装的是哪一版」，
            之前就因此产生过「是不是旧版本没改好」的误会。 */}
        <div className="flex items-center justify-between gap-3 rounded-md border border-border/60 px-3 py-2.5">
          <span className="shrink-0 text-xs text-muted-foreground">{t.setAboutVersion}</span>
          <span className="min-w-0 text-right text-sm font-medium tabular-nums">
            VoxMinutes v{version || '—'}
            {builtAt && (
              <span className="ml-2 font-normal text-muted-foreground">
                {t.setAboutBuiltAt.replace('{time}', builtAt.toLocaleString())}
              </span>
            )}
          </span>
        </div>

        {/* 法律条款（常驻入口）：首次启动的同意提示只出现一次，
            这里保证用户任何时候都能重新查阅《用户协议》与《隐私政策》。 */}
        <div className="flex items-center justify-between rounded-md border border-border/60 px-3 py-2.5">
          <span className="text-xs text-muted-foreground">{t.setAboutLegal}</span>
          <div className="flex items-center gap-3 text-xs">
            <button
              type="button"
              className="text-primary underline underline-offset-2 hover:opacity-80"
              onClick={() => void openExternalUrl(termsOfServiceUrl(lang)).catch(() => {})}
            >
              {t.welTermsLink}
            </button>
            <button
              type="button"
              className="text-primary underline underline-offset-2 hover:opacity-80"
              onClick={() => void openExternalUrl(privacyPolicyUrl(lang)).catch(() => {})}
            >
              {t.welPrivacyLink}
            </button>
          </div>
        </div>

        {/* 本版本新增功能（文案占位，发布前再定稿） */}
        <div className="rounded-md border border-border/60 px-3 py-2.5">
          <div className="text-xs font-medium">{t.setAboutChangelogTitle}</div>
          <p className="mt-1 whitespace-pre-line text-xs leading-relaxed text-muted-foreground">
            {t.setAboutChangelogPlaceholder}
          </p>
        </div>

        {/* 自动更新（真正的下载 + 安装 + 重启；失败有手动下载/官网兜底） */}
        <div className="rounded-md border border-border/60 px-3 py-2.5">
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              onClick={() => void updater.checkNow()}
              disabled={phase.kind === 'checking' || phase.kind === 'downloading' || phase.kind === 'installing'}
            >
              <RefreshCw className={`h-3.5 w-3.5 ${phase.kind === 'checking' ? 'animate-spin' : ''}`} />
              {phase.kind === 'checking' ? t.setAboutChecking : t.setAboutCheckUpdate}
            </Button>

            <div className="min-w-0 flex-1 text-xs">
              {phase.kind === 'latest' && <span className="text-muted-foreground">{t.setAboutUpToDate}</span>}
              {phase.kind === 'available' && (
                <span className="text-emerald-700 dark:text-emerald-500">
                  <Rocket className="mr-1 inline h-3.5 w-3.5" />
                  {t.setUpdFound.replace('{version}', phase.version)}
                </span>
              )}
              {phase.kind === 'downloading' && (
                <span className="text-muted-foreground">
                  {t.setUpdDownloading.replace('{percent}', String(progressPercent(phase.received, phase.total) ?? 0))}
                </span>
              )}
              {phase.kind === 'installing' && <span className="text-muted-foreground">{t.setUpdInstalling}</span>}
              {phase.kind === 'error' && (
                <span className="text-muted-foreground">{t.setAboutCheckFailed}</span>
              )}
            </div>

            {phase.kind === 'available' && (
              <Button size="sm" className="shrink-0 gap-1.5" onClick={() => void updater.installNow()}>
                <Download className="h-3.5 w-3.5" />
                {t.setUpdNow}
              </Button>
            )}
          </div>

          {/* 更新说明 */}
          {phase.kind === 'available' && phase.notes && (
            <p className="mt-2 whitespace-pre-line text-xs leading-relaxed text-muted-foreground">{phase.notes}</p>
          )}

          {/* 下载进度条：总长度未知时走不确定动画，不让用户以为卡住 */}
          {phase.kind === 'downloading' &&
            (() => {
              const pct = progressPercent(phase.received, phase.total)
              return (
                <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    className={`h-full rounded-full bg-primary transition-[width] duration-200 ${pct === null ? 'animate-pulse' : ''}`}
                    style={{ width: pct === null ? '100%' : `${pct}%` }}
                  />
                </div>
              )
            })()}

          {/* ⚠️ 兜底出口：检查失败 / 更新失败时，明确引导用户手动下载或去官网。
              升级链路跨 VPS、网络、签名校验、NSIS 安装器四环，任何一环翻车都不该让用户卡死。 */}
          {phase.kind === 'error' && (
            <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
              <span className="text-amber-700 dark:text-amber-500">
                {t.setUpdFailed.replace('{error}', phase.message.slice(0, 120))}
              </span>
              <span className="text-muted-foreground">{t.setUpdManualHint}</span>
              <Button variant="outline" size="sm" className="h-6 gap-1 px-2 text-xs" onClick={updater.downloadFullInstaller}>
                <Download className="h-3 w-3" />
                {t.setAboutDownload}
              </Button>
              <Button variant="ghost" size="sm" className="h-6 gap-1 px-2 text-xs" onClick={updater.openWebsite}>
                <Globe className="h-3 w-3" />
                {t.setAboutWebsite}
              </Button>
            </div>
          )}
        </div>

        {/* 访问官网（发卡站，可购买充值卡） */}
        <div className="flex items-center gap-3 rounded-md border border-border/60 px-3 py-2.5">
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={handleVisitWebsite}
            title={OFFICIAL_WEBSITE_URL}
          >
            <Globe className="h-3.5 w-3.5" />
            {t.setAboutWebsite}
          </Button>
          {/* 直接下载最新完整安装包（VPS 直供，国内比 GitHub 快）。
              与「检查更新」互补：检查更新只提示，这里是拿到包的入口。 */}
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={() => void openExternalUrl(LATEST_DOWNLOAD_URL).catch(() => {})}
            title={LATEST_DOWNLOAD_URL}
          >
            <Download className="h-4 w-4" />
            {t.setAboutDownload}
          </Button>
        </div>
      </div>
    </SettingsSection>
  )
}
