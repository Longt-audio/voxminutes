'use client'

import { useEffect } from 'react'

/**
 * 根级错误边界：兜住 app/layout.tsx 本身（以及 AppShell）抛错的情况。
 *
 * 它**替换整个 <html>**，因此不能用项目里的 UI 组件 / 样式（globals.css 不生效），
 * 全部内联写死；文案也不走 i18n store（store 初始化本身可能就是出错原因）。
 * 目的只有一个：任何情况下都别给用户一个纯白窗口。
 */
export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  useEffect(() => {
    // 尽力而为地留痕：日志桥此时可能没装上，浏览器控制台仍能看到
    console.error('[global-error]', error?.name, error?.message, error?.digest ?? '', error?.stack ?? '')
  }, [error])

  return (
    <html lang="zh-CN">
      <body
        style={{
          margin: 0,
          height: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: '#0b0b0c',
          color: '#e7e7ea',
          fontFamily:
            '-apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", sans-serif',
        }}
      >
        <div style={{ maxWidth: 560, padding: '0 24px', textAlign: 'center' }}>
          <div style={{ fontSize: 15, fontWeight: 600, marginBottom: 6 }}>
            应用启动出错 · The app failed to start
          </div>
          <div style={{ fontSize: 12.5, opacity: 0.75, lineHeight: 1.7 }}>
            录音与历史记录仍保存在本地，未丢失。
            <br />
            Your recordings and history are still on disk.
          </div>
          <pre
            style={{
              marginTop: 14,
              padding: '10px 12px',
              borderRadius: 8,
              background: 'rgba(255,255,255,0.06)',
              fontSize: 11,
              lineHeight: 1.6,
              textAlign: 'left',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
              maxHeight: 160,
              overflow: 'auto',
            }}
          >
            {error?.name}: {error?.message}
            {error?.digest ? `\n(digest ${error.digest})` : ''}
          </pre>
          <div style={{ marginTop: 16, display: 'flex', gap: 8, justifyContent: 'center' }}>
            <button
              onClick={() => reset()}
              style={{
                padding: '6px 14px',
                borderRadius: 6,
                border: '1px solid rgba(255,255,255,0.25)',
                background: 'transparent',
                color: 'inherit',
                fontSize: 12.5,
                cursor: 'pointer',
              }}
            >
              重试 Retry
            </button>
            <button
              onClick={() => window.location.reload()}
              style={{
                padding: '6px 14px',
                borderRadius: 6,
                border: '1px solid rgba(255,255,255,0.25)',
                background: 'transparent',
                color: 'inherit',
                fontSize: 12.5,
                cursor: 'pointer',
              }}
            >
              重新加载 Reload
            </button>
          </div>
        </div>
      </body>
    </html>
  )
}
