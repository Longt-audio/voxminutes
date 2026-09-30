import { Toaster } from 'sonner'
import './globals.css'
import { AppShell } from '@/components/AppShell'
// 前端日志桥（services/logger）在 AppShell 里引入——必须由客户端组件引，
// 见 AppShell.tsx 顶部注释（Server Component 引它不会在浏览器里执行）。

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <body className="h-screen overflow-hidden">
        <AppShell>{children}</AppShell>
        <Toaster position="bottom-center" richColors closeButton className="pointer-events-none" />
      </body>
    </html>
  )
}
