'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { FileText, History, Settings, Minus, Square, X, Copy, Languages } from 'lucide-react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Button } from '@/components/ui/button'
import { LanguageSwitcher } from '@/components/LanguageSwitcher'
import { useMessages } from '@/i18n/useMessages'
import { useModelLoadingToasts } from '@/hooks/useModelLoadingToasts'
import { OnboardingDialog } from '@/components/onboarding/OnboardingDialog'

/**
 * VoxMinutes 应用外壳：无边框窗口的自定义标题栏（拖动区 + 导航 + 窗口控制按钮）。
 * MVP 只有三个入口：实时转录 / 历史记录 / 设置。
 */

const NAV_ITEMS = [
  { href: '/', msgKey: 'navTranscribe', icon: FileText },
  { href: '/history', msgKey: 'navHistory', icon: History },
  { href: '/translate', msgKey: 'navTranslate', icon: Languages },
  { href: '/settings', msgKey: 'navSettings', icon: Settings },
] as const

const appWindow = () => getCurrentWindow()

function WindowControls() {
  const [maximized, setMaximized] = useState(false)
  const t = useMessages()

  useEffect(() => {
    appWindow().isMaximized().then(setMaximized).catch(() => {})
    const unlisten = appWindow().onResized(() => {
      appWindow().isMaximized().then(setMaximized).catch(() => {})
    })
    return () => {
      unlisten.then((fn) => fn()).catch(() => {})
    }
  }, [])

  return (
    <div className="flex items-center self-stretch">
      <button
        className="h-full w-11 flex items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        onClick={() => appWindow().minimize().catch(() => {})}
        title={t.winMinimize}
      >
        <Minus className="h-4 w-4" />
      </button>
      <button
        className="h-full w-11 flex items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        onClick={() => appWindow().toggleMaximize().catch(() => {})}
        title={maximized ? t.winRestore : t.winMaximize}
      >
        {maximized ? <Copy className="h-3.5 w-3.5" /> : <Square className="h-3.5 w-3.5" />}
      </button>
      <button
        className="h-full w-11 flex items-center justify-center text-muted-foreground transition-colors hover:bg-destructive hover:text-white"
        onClick={() => appWindow().close().catch(() => {})}
        title={t.winClose}
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  )
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const rawPathname = usePathname()
  const pathname = rawPathname.replace(/\/$/, '') || '/'
  const t = useMessages()
  useModelLoadingToasts()
  // 版本号从 Tauri 运行时读取（跟随 tauri.conf.json，不再硬编码）
  const [version, setVersion] = useState('')
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {})
  }, [])

  return (
    <div className="flex flex-col h-screen bg-background">
      <header
        data-tauri-drag-region
        className="flex items-center justify-between pl-4 h-12 border-b border-border/60 shrink-0 bg-card/50 backdrop-blur-sm select-none"
      >
        <div data-tauri-drag-region className="flex items-center gap-4 min-w-0">
          {/* Logo */}
          <div data-tauri-drag-region className="flex items-center gap-2">
            <div className="w-6 h-6 rounded-full bg-primary flex items-center justify-center shrink-0 overflow-hidden">
              <svg width="13" height="10" viewBox="0 0 20 14" fill="none">
                <rect x="0" y="6" width="3" height="8" rx="1" fill="white" />
                <rect x="4.25" y="0" width="3" height="14" rx="1" fill="white" />
                <rect x="8.5" y="4" width="3" height="10" rx="1" fill="white" />
                <rect x="12.75" y="0" width="3" height="14" rx="1" fill="white" />
                <rect x="17" y="5" width="3" height="9" rx="1" fill="white" />
              </svg>
            </div>
            <h1 className="text-sm font-semibold tracking-tight text-foreground/90">
              VoxMinutes
            </h1>
          </div>

          {/* 导航 */}
          <nav className="flex items-center gap-0.5">
            {NAV_ITEMS.map(({ href, msgKey, icon: Icon }) => (
              <Button
                key={href}
                variant={pathname === href ? 'secondary' : 'ghost'}
                size="sm"
                className="h-8"
                asChild
              >
                <Link href={href}>
                  <Icon className="h-3.5 w-3.5 mr-1.5" /> {t[msgKey]}
                </Link>
              </Button>
            ))}
          </nav>
        </div>

        <div className="flex items-center gap-3 self-stretch">
          <LanguageSwitcher />
          <WindowControls />
        </div>
      </header>

      <main className="flex-1 overflow-hidden relative">{children}</main>

      {/* 首次启动引导：模型下载/导入向导（全局，不随路由卸载） */}
      <OnboardingDialog />

      {/* 底部栏：版本号 + 完整 slogan（随语言切换），靠左显示；刻意弱化不抢眼 */}
      <footer className="shrink-0 h-8 flex items-center border-t border-border/60 bg-card/50 backdrop-blur-sm select-none px-4">
        <span className="text-[10px] text-muted-foreground/60">VoxMinutes{version ? ` v${version}` : ''} · {t.sloganFooter}</span>
      </footer>
    </div>
  )
}
