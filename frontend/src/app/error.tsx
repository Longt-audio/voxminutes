'use client'

import { useEffect } from 'react'
import Link from 'next/link'
import { AlertTriangle, Home, RotateCcw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/**
 * 页面级错误边界（Next.js App Router）。
 *
 * 为什么必须有（2026-09-26 白屏事故）：整个项目此前**没有任何 error boundary**，
 * 于是任何一个页面组件在渲染期抛错，React 会把整棵组件树卸载 —— 用户看到的是
 * 一个纯白窗口，连左侧导航都没有，只能强退重启（实测：点翻译页模型选择器右侧的
 * 齿轮进 /translate/models，因 selector 每次返回新对象触发无限重渲染，
 * React 抛 "Maximum update depth exceeded" → 直接白屏）。
 *
 * error.tsx 渲染在 app/layout.tsx **之内**，所以 AppShell（侧边栏）仍然在，
 * 用户可以自己切走，不需要重启应用。
 */
export default function PageError({
  error,
  reset,
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  const t = useMessages()

  useEffect(() => {
    // 这条会经 AppShell 引入的日志桥写进 app_*.log（见 services/logger.ts），
    // 是以后排查白屏类问题唯一的一手线索——务必保留。
    console.error(
      '[page-error]',
      error?.name,
      error?.message,
      error?.digest ? `digest=${error.digest}` : '',
      error?.stack ?? '',
    )
  }, [error])

  return (
    <div className="h-full flex items-center justify-center px-6 py-10">
      <div className="w-full max-w-lg rounded-lg border border-amber-200 bg-amber-50/70 px-5 py-4">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-amber-600" />
          <div className="flex-1">
            <h2 className="text-sm font-semibold text-amber-900">{t.errPageTitle}</h2>
            <p className="mt-1 text-xs text-amber-800">{t.errPageHint}</p>

            <pre className="mt-3 max-h-32 overflow-auto rounded border border-amber-200 bg-white/70 px-2 py-1.5 text-[11px] leading-relaxed text-amber-900 whitespace-pre-wrap break-all custom-scrollbar">
              {error?.name}: {error?.message}
              {error?.digest ? `\n(digest ${error.digest})` : ''}
            </pre>

            <div className="mt-3 flex items-center gap-2">
              <Button size="sm" variant="outline" className="gap-1.5" onClick={() => reset()}>
                <RotateCcw className="h-3.5 w-3.5" />
                {t.comRetry}
              </Button>
              <Button size="sm" variant="outline" className="gap-1.5" asChild>
                <Link href="/">
                  <Home className="h-3.5 w-3.5" />
                  {t.navTranscribe}
                </Link>
              </Button>
              <Button size="sm" variant="ghost" onClick={() => window.location.reload()}>
                {t.errReload}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
