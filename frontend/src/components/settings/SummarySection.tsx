'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import {
  summaryGetConfig,
  summaryListModels,
  summarySaveConfig,
  summaryTestConnection,
} from '@/services/ipc'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 设置页：会议总结 / AI —— API 协议、端点、密钥、模型配置 */
export function SummarySection() {
  const t = useMessages()
  const [protocol, setProtocol] = useState('openai')
  const [endpoint, setEndpoint] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [model, setModel] = useState('')
  const [fetchedModels, setFetchedModels] = useState<string[]>([])
  const [testing, setTesting] = useState(false)
  const [fetching, setFetching] = useState(false)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    summaryGetConfig()
      .then((config) => {
        if (!config) return
        setProtocol(config.protocol || 'openai')
        setEndpoint(config.endpoint)
        setApiKey(config.apiKey)
        setModel(config.model)
      })
      .catch(() => {})
  }, [])

  const currentConfig = () => ({ protocol, endpoint: endpoint.trim(), apiKey, model: model.trim() })

  const handleFetchModels = async () => {
    setFetching(true)
    try {
      const models = await summaryListModels(currentConfig())
      setFetchedModels(models)
      if (models.length > 0 && !models.includes(model.trim())) setModel(models[0])
    } catch (e) {
      toast.error(t.sumFetchModelsFailed.replace('{error}', String(e)))
    } finally {
      setFetching(false)
    }
  }

  const handleTest = async () => {
    setTesting(true)
    try {
      await summaryTestConnection(currentConfig())
      toast.success(t.sumApiTestOk)
    } catch (e) {
      toast.error(t.sumApiTestFailed.replace('{error}', String(e)))
    } finally {
      setTesting(false)
    }
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      await summarySaveConfig(currentConfig())
      toast.success(t.sumApiSaved)
    } catch (e) {
      toast.error(t.sumApiSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  return (
    <SettingsSection title={t.sumSettingsTitle} description={t.sumSettingsHint}>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.sumApiProtocol}</span>
          <Select value={protocol} onValueChange={setProtocol}>
            <SelectTrigger className="text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="openai">{t.sumProtocolOpenAI}</SelectItem>
              <SelectItem value="anthropic">{t.sumProtocolAnthropic}</SelectItem>
            </SelectContent>
          </Select>
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.sumApiEndpoint}</span>
          <Input
            value={endpoint}
            placeholder={t.sumEndpointPlaceholder}
            onChange={(e) => setEndpoint(e.target.value)}
          />
        </label>
      </div>

      <div className="mt-4 flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.sumApiKey}</span>
        <Input type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
        <span className="text-[11px] text-muted-foreground/80">{t.sumApiKeyHint}</span>
      </div>

      <div className="mt-4 flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.sumApiModel}</span>
        <div className="flex items-center gap-2">
          <Input className="flex-1 min-w-0" value={model} onChange={(e) => setModel(e.target.value)} />
          <Button variant="outline" className="shrink-0" disabled={fetching} onClick={handleFetchModels}>
            {fetching ? t.comLoading : t.sumApiFetchModels}
          </Button>
        </div>
        {fetchedModels.length > 0 && (
          <Select value={model} onValueChange={setModel}>
            <SelectTrigger className="text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {fetchedModels.map((m) => (
                <SelectItem key={m} value={m}>
                  {m}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      <div className="mt-4 flex items-center gap-2">
        <Button variant="outline" disabled={testing} onClick={handleTest}>
          {testing ? t.comLoading : t.sumApiTest}
        </Button>
        <Button disabled={saving} onClick={handleSave}>
          {t.comSave}
        </Button>
      </div>
    </SettingsSection>
  )
}
