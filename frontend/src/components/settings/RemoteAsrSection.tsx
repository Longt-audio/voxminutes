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
  type RemoteModelItem,
} from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'
import { remoteModelOptionLabel, modelUsageAllows } from '@/lib/remoteModelChoice'
import { catalogByKind } from '@/stores/remoteCatalogStore'
import { useRemoteModelChoiceStore } from '@/stores/remoteModelChoiceStore'

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
  // 远程模型选择走全局共享 store（与录音弹窗/历史页重识别共用同一份值，
  // 在设置页改选后各处立即同步）
  const choice = useRemoteModelChoiceStore((s) => s.values)
  const loadChoice = useRemoteModelChoiceStore((s) => s.load)
  const persistChoice = useRemoteModelChoiceStore((s) => s.persist)

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
    loadChoice()
  }, [loadChoice])

  const updateChoice = (kind: 'asr' | 'translate' | 'summary' | 'tts', value: string) => {
    // ASR 必须带 mode（目录下发）：后端据此决定流式/非流式。此前漏传 asr_mode，
    // 兜底靠模型名后缀判断——豆包模型名以 -2.0 结尾（doubao-asr-streaming-2.0），
    // 被误判成非流式走批量端点 → 400「该模型是流式 ASR」（2026-09-20 音频测试排查）。
    const m = modelList.find((mm) => mm.id === value)
    persistChoice(kind, value, kind === 'asr' ? m?.mode : undefined).catch(() => {})
  }

  /** 某用途下可选的模型。
   *
   *  翻译与总结共用 kind='translate' 的目录，但**合法集合不相交**（网关后台按用途标注）：
   *  翻译下拉不列「只总结」的（deepseek-flash），总结下拉不列「只翻译」的（豆包机器翻译）。
   *  与 SummaryDialog / 翻译页同一口径；缺 usage 字段的旧网关按 both 放行。 */
  const optionsFor = (kind: 'asr' | 'translate' | 'summary' | 'tts') => {
    if (kind === 'translate' || kind === 'summary') {
      return catalogByKind(modelList, 'translate').filter((m) => modelUsageAllows(m, kind))
    }
    return catalogByKind(modelList, kind)
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
      // 这里只关心「服务器是否可达」；流式通道检查在账户页/欢迎弹窗的状态行里给出
      const res = await checkRemoteAsrHealth(url)
      setHealth(res.ok)
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
            {(['asr', 'translate', 'summary', 'tts'] as const).map((kind) => (
              <div key={kind} className="flex items-center gap-2">
                <span className="w-16 text-xs text-muted-foreground shrink-0">
                  {kind === 'asr'
                    ? t.accModelKindAsr
                    : kind === 'translate'
                      ? t.setRemoteKindTranslate
                      : kind === 'summary'
                        ? t.setRemoteKindSummary
                        : 'TTS'}
                </span>
                {/* 翻译与总结的合法模型集不相交（网关后台按用途标注），所以这里各自过滤：
                    翻译下拉不列「只总结」的（deepseek-flash），总结下拉不列「只翻译」的
                    （豆包机器翻译）。与 SummaryDialog / 翻译页同一口径。 */}
                <Select value={choice[kind]} onValueChange={(v) => updateChoice(kind, v)}>
                  <SelectTrigger className="flex-1 min-w-0 h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {optionsFor(kind).map((m) => (
                      <SelectItem key={m.id} value={m.id} className="text-xs">
                        {remoteModelOptionLabel(m, t)}
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
