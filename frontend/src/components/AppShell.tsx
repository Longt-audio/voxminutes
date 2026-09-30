'use client'

import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { FileText, History, Settings, Minus, Square, X, Copy, Languages, Eraser, User, Volume2, AlertTriangle } from 'lucide-react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { useModelLoadingToasts } from '@/hooks/useModelLoadingToasts'
import { useTranscripts } from '@/hooks/useTranscripts'
import { useRecordingStateSync } from '@/hooks/useRecordingStateSync'
import { useTranscriptionStatus } from '@/hooks/useTranscriptionStatus'
import { useTranscriptionErrorToasts } from '@/hooks/useTranscriptionErrorToasts'
import { useRetranscriptionEvents } from '@/hooks/useRetranscriptionEvents'
import { useRecordingTimer } from '@/hooks/useRecorder'
import { WelcomeDialog } from '@/components/onboarding/WelcomeDialog'
import { RemoteMessages } from '@/components/RemoteMessages'
import { UpdateBanner } from '@/components/UpdateBanner'
import { ImportantNoticeBanner } from '@/components/ImportantNoticeBanner'
import { LowCreditBanner } from '@/components/LowCreditBanner'
import { clearAllModelBackends, warmRemoteStreaming } from '@/services/ipc'
import { useRemoteCatalogStore } from '@/stores/remoteCatalogStore'
import { useAppStore } from '@/state'
import { useTranslatePageStore } from '@/stores/translatePageStore'
// ⚠️ 必须由**客户端组件**引入前端日志桥：以前它只在 app/layout.tsx（Server Component）
// 里 import，模块只在 Node 侧求值一次（那里没有 window → isTauri=false），
// 浏览器端根本没被执行 → 前端 console.error/warn 从来没进过 app_*.log
// （2026-09-26 排查白屏事故时发现：整个日志里 0 条前端日志）。
// AppShell 是 'use client'，浏览器端会真正求值并接管 console。
import '@/services/logger'

/**
 * VoxMinutes 应用外壳：无边框窗口的自定义标题栏（拖动区 + 导航 + 窗口控制按钮）。
 * MVP 只有三个入口：实时转录 / 历史记录 / 设置。
 */

const NAV_ITEMS = [
  { href: '/', msgKey: 'navTranscribe', icon: FileText },
  { href: '/history', msgKey: 'navHistory', icon: History },
  { href: '/translate', msgKey: 'navTranslate', icon: Languages },
  { href: '/tts', msgKey: 'navTts', icon: Volume2 },
  { href: '/settings', msgKey: 'navSettings', icon: Settings },
  { href: '/account', msgKey: 'navAccount', icon: User },
] as const

const appWindow = () => getCurrentWindow()

function formatDuration(totalSeconds: number): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  return `${pad(h)}:${pad(m)}:${pad(s)}`
}

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
  const router = useRouter()
  const rawPathname = usePathname()
  const pathname = rawPathname.replace(/\/$/, '') || '/'
  const t = useMessages()
  useModelLoadingToasts()
  useTranscriptionErrorToasts()
  // 实时转写事件全局监听（2026-09-24）：以前这个订阅挂在录音页的 TranscriptPanel 里，
  // **切到别的页面就退订**→ 那段音频虽然照常识别/落盘，界面上却永久缺一截
  // （用户实测「切页面时音频没被识别」，其实文件里是完整的）。挂到 AppShell 后
  // 面板只负责渲染 store，不再拥有订阅。
  useTranscripts()
  // 录音状态全局同步（切页面后指示条/状态不丢；录音页仍负责保存与跳转）
  useRecordingStateSync()
  // 识别链路状态（2026-09-29）：识别因积分不足等原因停止后，界面必须看得出来
  // （此前胶囊一直显示「实时转写中」，用户以为还在识别）
  useTranscriptionStatus()
  // 离线（重）识别事件全局监听：切页面不丢进度、完成仍有提示（见该 hook 注释）
  useRetranscriptionEvents()
  // 版本号从 Tauri 运行时读取（跟随 tauri.conf.json，不再硬编码）
  const [version, setVersion] = useState('')
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {})
  }, [])

  // 远程模型目录：初次强刷 + 60s 轮询（后台改了前端实时生效），卸载时停止
  useEffect(() => {
    const catalog = useRemoteCatalogStore.getState()
    catalog.startPolling()
    void catalog.refresh(true)
    return () => useRemoteCatalogStore.getState().stopPolling()
  }, [])

  // 流式识别通道提前预热（fire-and-forget）：把 ~3.5s 的跨境握手挪到启动后的空闲时间，
  // 点「开始录音」时命中 5 分钟预检缓存即免等待。未配置远程时后端直接跳过。
  useEffect(() => {
    warmRemoteStreaming()
  }, [])

  // 录音进行中的全局指示（2026-09-24）：录音/识别都在 Rust 侧跑，切页面不受影响；
  // 这里只做「看得见」——底部栏右侧常驻「● 时长 实时转写中」，任何页面可见，
  // 点击回到实时转录页。时长与录音页主计时器共用 recordingDuration（暂停时冻结）；
  // 计时器全局挂载在这里，切到别的页面也不会停走。
  const isRecording = useAppStore((s) => s.isRecording)
  const recordingDuration = useAppStore((s) => s.recordingDuration)
  const asrStopReason = useAppStore((s) => s.asrStopReason)
  useRecordingTimer()

  // 「识别已停止」胶囊文案：原因来自 Rust 事件（积分不足/服务不可用），
  // 点击仍回到实时转录页（那里可切模型或停止录音）。
  const stoppedText =
    asrStopReason === 'credits'
      ? t.recAsrStoppedCredits
      : asrStopReason === 'config' || asrStopReason === 'unavailable'
        ? t.recAsrStoppedConfig
        : t.recAsrStopped
  const stoppedTitle = stoppedText

  // 清空：实时转录/翻译页的显示文本随时可清（纯显示态，不影响后端录音数据与已保存文件）；
  // 模型后台只在非录音时清（录音中本地 ASR/翻译模型正在工作，卸载会中断转写——用户已拍板）
  const handleClearMemory = async () => {
    const recording = useAppStore.getState().isRecording
    useAppStore.getState().clearTranscripts()
    useTranslatePageStore.getState().setInput('')
    useTranslatePageStore.getState().setOutput('')
    if (recording) {
      toast.success(t.setClearTextDone)
      return
    }
    try {
      await clearAllModelBackends()
      toast.success(t.setClearAllDone)
    } catch (e) {
      toast.error(t.setClearFailed.replace('{error}', String(e)))
    }
  }

  return (
    // 窗口是 decorations:false（自绘标题栏）：Windows 上还关掉了系统阴影（shadow:false），
    // 于是窗口四周**没有任何边界**，桌面上是白底时整个窗口糊成一片（2026-09-30 真机反馈）。
    // 加一圈 1px 描边把窗口轮廓勾出来。box-sizing 是 border-box，不会撑出滚动条。
    <div className="flex flex-col h-screen bg-background border border-border">
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
            <h1 data-tauri-drag-region className="text-sm font-semibold tracking-tight text-foreground/90">
              VoxMinutes
            </h1>
          </div>

          {/* 导航（按钮间隙也作为拖动区） */}
          <nav data-tauri-drag-region className="flex items-center gap-0.5">
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

        <div data-tauri-drag-region className="flex items-center gap-3 self-stretch">
          <Button
            variant="ghost"
            size="sm"
            className="h-8 px-2 gap-1.5 text-xs"
            onClick={handleClearMemory}
            title={t.setClearMemoryHint}
          >
            <Eraser className="h-3.5 w-3.5" />
            {t.setClearMemoryShort}
          </Button>
          <WindowControls />
        </div>
      </header>

      {/* 软件更新提醒横幅（页首，新版本可用时显示）+ 重要信息推送（网关下发，同一条关闭后不再弹出）
          + 积分不足提醒（低于阈值时显示，三条横幅文档流内上下堆叠） */}
      <UpdateBanner />
      <ImportantNoticeBanner />
      <LowCreditBanner />

      <main className="flex-1 overflow-hidden relative">{children}</main>

      {/* 欢迎弹窗：欢迎词 + 公告 + 远程/本地模型设置（每次启动弹出，可勾选不再打开） */}
      <WelcomeDialog />

      {/* 底部栏：左侧技巧/推送轮播；右侧录音中指示（点击回实时转录页）+ 版本号 */}
      <footer className="shrink-0 h-8 flex items-center justify-between gap-3 border-t border-border/60 bg-card/50 backdrop-blur-sm select-none px-4 min-w-0">
        <RemoteMessages />
        <div className="flex shrink-0 items-center gap-3">
          {isRecording && (
            <button
              type="button"
              onClick={() => router.push('/')}
              className={
                asrStopReason
                  ? 'flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-medium text-amber-700 transition-colors hover:bg-muted dark:text-amber-400'
                  : 'flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-medium text-red-600 transition-colors hover:bg-muted dark:text-red-400'
              }
              title={asrStopReason ? stoppedTitle : t.recBackToLive}
            >
              {asrStopReason ? (
                // 识别已停止：不再脉冲（脉冲=还在识别），改成静态三角，颜色转琥珀
                <AlertTriangle className="h-3 w-3" />
              ) : (
                <span className="h-1.5 w-1.5 rounded-full bg-red-500 animate-pulse" />
              )}
              <span className="tabular-nums">{formatDuration(recordingDuration)}</span>
              <span>{asrStopReason ? stoppedText : t.recLiveTranscribing}</span>
            </button>
          )}
          {version && (
            <span className="text-[10px] tabular-nums text-muted-foreground/60">v{version}</span>
          )}
        </div>
      </footer>
    </div>
  )
}
