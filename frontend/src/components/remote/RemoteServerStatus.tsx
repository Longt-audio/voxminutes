'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { Loader2 } from 'lucide-react'
import { checkRemoteAsrHealth } from '@/services/ipc'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { cn } from '@/lib/utils'

/** 只读服务器状态行 + 三态测试按钮（欢迎弹窗 P2 与账户页远程卡片共用）：
 *  - 未测：「服务器：api.voxmin.top · 默认」+「测试连接」
 *  - 测试中：转圈 + 禁用
 *  - 成功：状态行变绿「✓ 已连接」，按钮变次要样式「重新测试」
 *  - 失败：状态行变红 + 错误提示，按钮变「重试」
 *
 *  两段式检查（2026-09-22 起）：先打网关 `/health`（HTTP，免鉴权、不要求已填授权码），
 *  再对「已选的流式识别模型」**真实握手一次 wss**。只测 /health 会漏掉流式通道自身的问题
 *  （客户端 TLS 未编入、鉴权、路由），曾导致「测试连接 OK 但一开录 0 字符」。
 *  `streamingOk === false` 时状态行显示琥珀色告警 + 原因，服务器仍算在线。 */
export function RemoteServerStatus({
  serverUrl,
  isDefault,
  autoTest,
  onAutoTested,
  onHealthChange,
}: {
  /** 生效服务器地址（getRemoteConfig().serverUrl；空串 = 配置未加载完成，不渲染） */
  serverUrl: string
  /** 是否内置默认地址（显示「默认」标注） */
  isDefault: boolean
  /** true 时挂载后自动测一次（父级用 state 保证只传一次 true，配合 onAutoTested 关闭） */
  autoTest?: boolean
  onAutoTested?: () => void
  /** 健康状态变化通知父级（账户页开关旁的在线徽标用；只反映服务器可达性，不含流式通道） */
  onHealthChange?: (health: boolean | null) => void
}) {
  const t = useMessages()
  const [health, setHealth] = useState<boolean | null>(null)
  const [streamingOk, setStreamingOk] = useState<boolean | null>(null)
  const [streamError, setStreamError] = useState<string | null>(null)
  const [latencyMs, setLatencyMs] = useState<number | null>(null)
  const [checking, setChecking] = useState(false)
  const autoTestedRef = useRef(false)
  const prevUrlRef = useRef(serverUrl)

  const runTest = useCallback(async () => {
    const url = serverUrl.trim()
    if (!url) return
    setChecking(true)
    const started = performance.now()
    try {
      const res = await checkRemoteAsrHealth(url)
      setLatencyMs(res.ok ? Math.round(performance.now() - started) : null)
      setHealth(res.ok)
      setStreamingOk(res.ok ? res.streamingOk : null)
      setStreamError(res.ok ? res.streamingError : null)
    } catch {
      setLatencyMs(null)
      setHealth(false)
      setStreamingOk(null)
      setStreamError(null)
    } finally {
      setChecking(false)
    }
  }, [serverUrl])

  // 地址变化（设置页高级卡片改了自定义地址/恢复默认）后旧测试结果失效；
  // 配置尚未加载（'' → 生效地址）的初次过渡不算变化。
  useEffect(() => {
    if (prevUrlRef.current !== serverUrl) {
      if (prevUrlRef.current && serverUrl) {
        setHealth(null)
        setLatencyMs(null)
        setStreamingOk(null)
        setStreamError(null)
      }
      prevUrlRef.current = serverUrl
    }
  }, [serverUrl])

  // 自动测试：仅触发一次（ref 防 StrictMode 双跑；父级 onAutoTested 负责关闭 autoTest）
  useEffect(() => {
    if (!autoTest || autoTestedRef.current || !serverUrl.trim()) return
    autoTestedRef.current = true
    onAutoTested?.()
    void runTest()
  }, [autoTest, serverUrl, runTest, onAutoTested])

  useEffect(() => {
    onHealthChange?.(health)
  }, [health, onHealthChange])

  if (!serverUrl.trim()) return null

  const host = serverUrl.replace(/^https?:\/\//, '').replace(/\/+$/, '')
  // 服务器在线但流式通道不通：单独一档告警色（红=服务器不可达，不混淆）
  const streamingBroken = health === true && streamingOk === false

  return (
    <div className="flex items-center gap-2">
      <span
        className={cn(
          'flex-1 min-w-0 truncate text-xs',
          health === false
            ? 'text-destructive'
            : streamingBroken
              ? 'text-amber-600'
              : health === true
                ? 'text-green-600'
                : 'text-muted-foreground'
        )}
        title={streamingBroken && streamError ? streamError : undefined}
      >
        {t.accServerLabel}
        {host}
        {isDefault && <span className="text-muted-foreground/70"> · {t.accServerDefaultBadge}</span>}
        {health === true && !streamingBroken && ` · ✓ ${t.accConnected}`}
        {health === true && !streamingBroken && latencyMs !== null && ` · ${latencyMs}ms`}
        {streamingBroken && ` · ⚠ ${t.accStreamingUnavailable}`}
        {streamingBroken && streamError && `：${streamError}`}
        {health === false && ` · ${t.accRemoteTestFailed}`}
      </span>
      <Button
        variant={health === true && !streamingBroken ? 'ghost' : 'outline'}
        size="sm"
        className="h-7 shrink-0 px-2 text-xs"
        onClick={() => void runTest()}
        disabled={checking}
      >
        {checking && <Loader2 className="h-3 w-3 animate-spin" />}
        {checking
          ? t.accRemoteTesting
          : health === true && !streamingBroken
            ? t.accRetest
            : health === false || streamingBroken
              ? t.accRetry
              : t.accRemoteTest}
      </Button>
    </div>
  )
}
