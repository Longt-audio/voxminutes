'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import {
  getRemoteConfig,
  setRemoteConfig,
  checkRemoteAsrHealth,
  getRemoteEnabled,
  setRemoteEnabled,
  listRemoteModels,
  getRemoteModelChoice,
  setRemoteModelChoice,
  type RemoteModelItem,
} from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 格式化模型单价为可读积分消耗文案：ASR「积分/小时」，LLM「积分/千token」，TTS「积分/千字符」。 */
function formatModelPrice(m: RemoteModelItem): string {
  const price = m.price ?? 0
  if (price <= 0) return '免费'
  if (m.price_unit === 'second') return `${(price * 3600).toFixed(2)} 积分/小时`
  if (m.price_unit === 'char') return `${(price * 1000).toFixed(2)} 积分/千字符`
  return `${(price * 1000).toFixed(2)} 积分/千token`
}

/** 设置页 API tab：远程服务配置（服务器地址 + 授权码 + 总开关），ASR/翻译/TTS 三者共用 */
export function RemoteAsrSection() {
  const t = useMessages()
  const [serverUrl, setServerUrl] = useState('')
  const [license, setLicense] = useState('')
  const [remoteModel, setRemoteModel] = useState('')
  const [enabled, setEnabled] = useState(false)
  const [health, setHealth] = useState<boolean | null>(null)
  const [saving, setSaving] = useState(false)
  const [checking, setChecking] = useState(false)
  const [modelList, setModelList] = useState<RemoteModelItem[]>([])
  const [choice, setChoice] = useState<{ asr: string; translate: string; tts: string }>({ asr: '', translate: '', tts: '' })

  useEffect(() => {
    getRemoteConfig()
      .then((cfg) => {
        setServerUrl(cfg.serverUrl || '')
        setLicense(cfg.license || '')
        setRemoteModel(cfg.model || '')
      })
      .catch(() => {})
    getRemoteEnabled()
      .then(setEnabled)
      .catch(() => {})
    listRemoteModels().then(setModelList).catch(() => {})
    getRemoteModelChoice().then(setChoice).catch(() => {})
  }, [])

  const updateChoice = (kind: 'asr' | 'translate' | 'tts', value: string) => {
    const next = { ...choice, [kind]: value }
    setChoice(next)
    setRemoteModelChoice({ [kind]: value }).catch(() => {})
  }

  const handleToggle = async (next: boolean) => {
    setEnabled(next)
    try {
      await setRemoteEnabled(next)
      toast.success(next ? t.setRemoteEnabledOn : t.setRemoteEnabledOff)
    } catch (e) {
      setEnabled(!next)
      toast.error(t.setSaveFailed.replace('{error}', String(e)))
    }
  }

  const handleSave = async () => {
    const url = serverUrl.trim()
    if (!url) {
      toast.error(t.setEnterEndpoint)
      return
    }
    if (!license.trim()) {
      toast.error(t.setEnterLicense)
      return
    }
    setSaving(true)
    try {
      await setRemoteConfig(url, license.trim())
      toast.success(t.setEndpointSaved)
    } catch (e) {
      toast.error(t.setSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  const handleCheck = async () => {
    const url = serverUrl.trim()
    if (!url) {
      toast.error(t.setEnterEndpoint)
      return
    }
    setChecking(true)
    setHealth(null)
    try {
      setHealth(await checkRemoteAsrHealth(url))
    } catch {
      setHealth(false)
    } finally {
      setChecking(false)
    }
  }

  return (
    <SettingsSection title={t.setRemoteAsrTitle}>
      <div className="flex flex-col gap-3">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => handleToggle(e.target.checked)}
            className="h-4 w-4"
          />
          <span>{t.setRemoteEnable}</span>
          {enabled && <Badge variant="success">{t.setOnline}</Badge>}
        </label>

        <div className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.setRemoteAsrAddress}</span>
          <div className="flex items-center gap-2">
            <Input
              className="flex-1 min-w-0"
              value={serverUrl}
              onChange={(e) => {
                setServerUrl(e.target.value)
                setHealth(null)
              }}
              placeholder="https://your-server.com"
            />
            <Button className="shrink-0" onClick={handleSave} disabled={saving}>
              {saving ? t.setSaving : t.comSave}
            </Button>
            <Button variant="outline" className="shrink-0" onClick={handleCheck} disabled={checking}>
              {checking ? t.setChecking : t.setTestConnection}
            </Button>
            {health !== null && (
              <Badge variant={health ? 'success' : 'destructive'}>{health ? t.setOnline : t.setOffline}</Badge>
            )}
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.setRemoteLicense}</span>
          <Input
            className="w-full"
            value={license}
            onChange={(e) => setLicense(e.target.value)}
            placeholder="sk-…"
            type="password"
          />
        </div>

        {modelList.length > 0 && (
          <div className="flex flex-col gap-2 pt-1">
            <span className="text-xs text-muted-foreground">{t.setRemoteModelsTitle}</span>
            {(['asr', 'translate', 'tts'] as const).map((kind) => (
              <div key={kind} className="flex items-center gap-2">
                <span className="w-10 text-xs text-muted-foreground shrink-0">
                  {kind === 'asr' ? 'ASR' : kind === 'translate' ? t.setRemoteKindTranslate : 'TTS'}
                </span>
                <Select value={choice[kind]} onValueChange={(v) => updateChoice(kind, v)}>
                  <SelectTrigger className="flex-1 min-w-0 h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {modelList
                      .filter((m) => m.kind === kind)
                      .map((m) => (
                        <SelectItem key={m.id} value={m.id} className="text-xs">
                          {m.owned_by} / {m.id}
                          <span className="ml-1 text-muted-foreground/70">
                            {m.mode === 'streaming' ? '· 流式' : ''}
                            {formatModelPrice(m)}
                          </span>
                        </SelectItem>
                      ))}
                  </SelectContent>
                </Select>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-700">
        {t.setRemoteNote}
      </div>
    </SettingsSection>
  )
}
