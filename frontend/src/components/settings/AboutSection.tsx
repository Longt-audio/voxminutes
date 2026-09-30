'use client'

import { useCallback, useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { Globe, RefreshCw, Rocket } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { fetchRemoteMessages, openExternalUrl } from '@/services/ipc'
import { pickLangSegment } from '@/lib/langSegment'
import { OFFICIAL_WEBSITE_URL, privacyPolicyUrl, termsOfServiceUrl } from '@/lib/site'

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

type CheckState =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'latest' }
  | { status: 'newer'; version: string; notes?: string; url?: string }
  | { status: 'failed' }

/** 设置页「关于我们」：版本号 / 本版本新增功能（占位） / 检查更新 / 访问官网。 */
export function AboutSection() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const [version, setVersion] = useState('')
  const [check, setCheck] = useState<CheckState>({ status: 'idle' })

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {})
  }, [])

  // 检查更新：复用网关 latest-version 通道（与 UpdateBanner 同源）。
  // 注意：仅「检查 + 提示」，自动下载安装尚未实现（留接口）。
  const handleCheckUpdate = useCallback(async () => {
    setCheck({ status: 'checking' })
    try {
      const res = await fetchRemoteMessages(lang)
      const lv = res.latestVersion
      if (lv && version && isNewer(lv.version, version)) {
        setCheck({
          status: 'newer',
          version: lv.version,
          // 版本 notes 支持四段式「中文|English|한국어|日本語」，按界面语言取段
          notes: lv.release_notes ? pickLangSegment(lv.release_notes, lang) : undefined,
          url: lv.download_url,
        })
      } else {
        setCheck({ status: 'latest' })
      }
    } catch {
      setCheck({ status: 'failed' })
    }
  }, [version, lang])

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
        {/* 版本号 */}
        <div className="flex items-center justify-between rounded-md border border-border/60 px-3 py-2.5">
          <span className="text-xs text-muted-foreground">{t.setAboutVersion}</span>
          <span className="text-sm font-medium tabular-nums">VoxMinutes v{version || '—'}</span>
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

        {/* 检查更新（接口已通：查询网关最新版本并提示；自动升级未实现） */}
        <div className="flex items-center gap-3 rounded-md border border-border/60 px-3 py-2.5">
          <Button variant="outline" size="sm" className="gap-1.5" onClick={handleCheckUpdate} disabled={check.status === 'checking'}>
            <RefreshCw className={`h-3.5 w-3.5 ${check.status === 'checking' ? 'animate-spin' : ''}`} />
            {check.status === 'checking' ? t.setAboutChecking : t.setAboutCheckUpdate}
          </Button>
          <div className="min-w-0 flex-1 text-xs">
            {check.status === 'latest' && <span className="text-muted-foreground">{t.setAboutUpToDate}</span>}
            {check.status === 'failed' && <span className="text-muted-foreground">{t.setAboutCheckFailed}</span>}
            {check.status === 'newer' && (
              <span className="text-emerald-700">
                <Rocket className="mr-1 inline h-3.5 w-3.5" />
                {t.setAboutNewVersion.replace('{version}', check.version)}
                {check.url && (
                  <button
                    type="button"
                    className="ml-2 underline underline-offset-2"
                    onClick={() => void openExternalUrl(String(check.url)).catch(() => {})}
                  >
                    {t.setAboutDownload}
                  </button>
                )}
              </span>
            )}
          </div>
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
        </div>
      </div>
    </SettingsSection>
  )
}
