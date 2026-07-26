import { Toaster } from 'sonner'
import './globals.css'
import { AppShell } from '@/components/AppShell'
import '@/services/logger'

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
