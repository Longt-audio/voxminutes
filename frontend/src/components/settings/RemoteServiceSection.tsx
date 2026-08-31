'use client'

import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import {
  getRemoteConfig,
  setRemoteConfig,
  checkRemoteAsrHealth,
  getRemoteEnabled,
  setRemoteEnabled,
  listRemoteModels,
  runSpeedTest,
  type RemoteModelItem,
  type SpeedTestResult,
} from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 格式化模型单价为可读积分消耗文案，如「0.05 积分/秒」。 */
function formatModelPrice(m: RemoteModelItem): string {
  const price = m.price ?? 0
  if (price <= 0) return '免费'
  const unit = m.price_unit === 'second' ? '秒' : m.price_unit === 'char' ? '字符' : 'token'
  const priceStr = price >= 0.01 ? price.toFixed(2) : price.toFixed(4)
  return `${priceStr} 积分/${unit}`
}

/** 用户中心「远程服务」卡片：
 *  - 服务器地址 + 授权码 + 总开关（自动保存，切换页面不丢）
 *  - 测试连接（未填授权码时不允许测试）
 *  - 只读展示可用模型（名称 / 模式 / 积分单价；模型选择请到各功能使用处）
 *  - 模型测速（仅测往返延迟，不扣积分）
 */
export function RemoteServiceSection({ onChanged }: { onChanged?: () => void }) {
  const t = useMessages()
  const [serverUrl, setServerUrl] = useState('')
  const [license, setLicense] = useState('')
  const [enabled, setEnabled] = useState(false)
  const [health, setHealth] = useState<boolean | null>(null)
  const [checking, setChecking] = useState(false)
  const [modelList, setModelList] = useState<RemoteModelItem[]>([])
  const [speed, setSpeed] = useState<Record<string, SpeedTestResult | 'running'>>({})
  const [speedRunning, setSpeedRunning] = useState(false)

  // 记录已加载的初始值，避免启动时误触发自动保存
  const loadedRef = useRef(false)
  const initialRef = useRef<{ url: string; key: string }>({ url: '', key: '' })
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    Promise.all([
      getRemoteConfig().catch(() => null),
      getRemoteEnabled().catch(() => false),
    ]).then(([cfg, en]) => {
      if (cfg) {
        setServerUrl(cfg.serverUrl || '')
        setLicense(cfg.license || '')
        initialRef.current = { url: cfg.serverUrl || '', key: cfg.license || '' }
      }
      setEnabled(!!en)
      loadedRef.current = true
    })
  }, [])

  useEffect(() => {
    if (enabled) {
      listRemoteModels().then(setModelList).catch(() => setModelList([]))
    } else {
      setModelList([])
    }
  }, [enabled])

  // 自动保存：地址/授权码变化后防抖 800ms，且两者都填写过才保存
  useEffect(() => {
    if (!loadedRef.current) return
    const url = serverUrl.trim()
    const key = license.trim()
    if (url === initialRef.current.url && key === initialRef.current.key) return
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    saveTimerRef.current = setTimeout(async () => {
      if (!url || !key) return // 未填齐不保存（避免把空值写进去）
      try {
        await setRemoteConfig(url, key)
        initialRef.current = { url, key }
        onChanged?.()
      } catch (e) {
        toast.error(t.accRemoteSaveFailed.replace('{error}', String(e)))
      }
    }, 800)
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serverUrl, license])

  const handleToggle = async (next: boolean) => {
    setEnabled(next)
    try {
      await setRemoteEnabled(next)
      onChanged?.()
    } catch (e) {
      setEnabled(!next)
      toast.error(t.accRemoteSaveFailed.replace('{error}', String(e)))
    }
  }

  const handleCheck = async () => {
    const url = serverUrl.trim()
    if (!url) {
      toast.error(t.accRemoteNeedUrl)
      return
    }
    if (!license.trim()) {
      toast.error(t.accRemoteNeedKey)
      return
    }
    setChecking(true)
    setHealth(null)
    try {
      setHealth(await checkRemoteAsrHealth(url))
      onChanged?.()
    } catch {
      setHealth(false)
    } finally {
      setChecking(false)
    }
  }

  // 每个模型单独测速：key 为 kind:id
  const handleModelSpeedTest = async (kind: 'asr' | 'llm' | 'tts', id: string) => {
    const key = `${kind}:${id}`
    setSpeed((s) => ({ ...s, [key]: 'running' }))
    try {
      const r = await runSpeedTest(kind, id)
      setSpeed((s) => ({ ...s, [key]: r }))
    } catch (e) {
      setSpeed((s) => ({ ...s, [key]: { kind, model: id, ok: false, ms: 0, detail: String(e) } }))
    }
  }

  const handleSpeedTestAll = async () => {
    setSpeedRunning(true)
    const tasks: Array<['asr' | 'llm' | 'tts', string]> = []
    for (const m of modelList) {
      const kind = m.kind === 'translate' ? 'llm' : (m.kind as 'asr' | 'tts')
      tasks.push([kind, m.id])
    }
    for (const [kind, id] of tasks) {
      await handleModelSpeedTest(kind, id)
    }
    setSpeedRunning(false)
  }

  const modelGroups: Array<{ key: string; label: string; kinds: string[] }> = [
    { key: 'asr', label: t.accModelKindAsr, kinds: ['asr'] },
    { key: 'translate', label: t.accModelKindTranslate, kinds: ['translate'] },
    { key: 'summary', label: t.accModelKindSummary, kinds: ['translate'] },
    { key: 'tts', label: t.accModelKindTts, kinds: ['tts'] },
  ]

  return (
    <SettingsSection title={t.accRemoteTitle} description={t.accRemoteAutoSaved}>
      <div className="flex flex-col gap-3">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => handleToggle(e.target.checked)}
            className="h-4 w-4"
          />
          <span>{t.accRemoteEnable}</span>
          {enabled && health === true && <Badge variant="success">{t.accRemoteOnline}</Badge>}
        </label>

        <div className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.accRemoteUrl}</span>
          <Input
            className="w-full"
            value={serverUrl}
            onChange={(e) => {
              setServerUrl(e.target.value)
              setHealth(null)
            }}
            placeholder="http://127.0.0.1:8788"
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.accRemoteKey}</span>
          <div className="flex items-center gap-2">
            <Input
              className="flex-1 min-w-0"
              value={license}
              onChange={(e) => setLicense(e.target.value)}
              placeholder="sk-…"
              type="password"
            />
            <Button variant="outline" className="shrink-0" onClick={handleCheck} disabled={checking}>
              {checking ? t.accRemoteTesting : t.accRemoteTest}
            </Button>
            {health !== null && (
              <Badge variant={health ? 'success' : 'destructive'}>
                {health ? t.accRemoteOnline : t.accRemoteOffline}
              </Badge>
            )}
          </div>
        </div>

        {/* 可用模型（只读展示，不选择） */}
        {enabled && modelList.length > 0 && (
          <div className="flex flex-col gap-2 pt-1">
            <span className="text-xs text-muted-foreground">{t.accModelsTitle}</span>
            {modelGroups.map((g) => {
              const models = modelList.filter((m) => g.kinds.includes(m.kind))
              if (models.length === 0) return null
              return (
                <div key={g.key} className="text-xs">
                  <span className="font-medium">{g.label}</span>
                  <div className="mt-1 flex flex-col gap-0.5">
                    {models.map((m) => (
                      <div key={`${g.key}-${m.id}`} className="flex items-center gap-2 text-muted-foreground">
                        <span className="truncate">{m.owned_by} / {m.id}</span>
                        {m.mode === 'streaming' && (
                          <span className="shrink-0 text-primary/80">· {t.accModeStreaming}</span>
                        )}
                        <span className="shrink-0 text-muted-foreground/60">{formatModelPrice(m)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )
            })}
            <p className="text-[11px] text-muted-foreground/70">{t.accModelsHint}</p>
          </div>
        )}

        {/* 模型测速（#9：仅测延迟，不扣积分；每个模型单独测速） */}
        {enabled && modelList.length > 0 && (
          <div className="flex flex-col gap-2 pt-1">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">{t.accSpeedTitle}</span>
              <Button variant="outline" size="sm" onClick={handleSpeedTestAll} disabled={speedRunning}>
                {speedRunning ? t.accSpeedRunning : t.accSpeedRunAll}
              </Button>
            </div>
            {modelGroups.map((g) => {
              const models = modelList.filter((m) => g.kinds.includes(m.kind))
              if (models.length === 0) return null
              return (
                <div key={`speed-${g.key}`} className="text-xs">
                  <span className="font-medium">{g.label}</span>
                  <div className="mt-1 flex flex-col gap-0.5">
                    {models.map((m) => {
                      const kind = m.kind === 'translate' ? 'llm' : (m.kind as 'asr' | 'tts')
                      const key = `${kind}:${m.id}`
                      const r = speed[key]
                      return (
                        <div key={key} className="flex items-center gap-2 text-muted-foreground">
                          <span className="truncate flex-1 min-w-0">{m.owned_by} / {m.id}</span>
                          {r === 'running' ? (
                            <span className="shrink-0 text-muted-foreground/70">{t.accSpeedRunning}</span>
                          ) : r && r.ok ? (
                            <span className="shrink-0 text-primary font-medium tabular-nums">{r.ms} ms</span>
                          ) : r && !r.ok ? (
                            <span className="shrink-0 text-destructive">{r.detail || '—'}</span>
                          ) : null}
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-5 px-2 text-[11px] shrink-0"
                            disabled={speedRunning || r === 'running'}
                            onClick={() => void handleModelSpeedTest(kind, m.id)}
                          >
                            {t.accSpeedTest}
                          </Button>
                        </div>
                      )
                    })}
                  </div>
                </div>
              )
            })}
            <p className="text-[11px] text-muted-foreground/70">{t.accSpeedHint}</p>
          </div>
        )}
      </div>
    </SettingsSection>
  )
}
