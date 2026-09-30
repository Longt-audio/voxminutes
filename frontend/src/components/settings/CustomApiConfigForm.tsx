'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useMessages } from '@/i18n/useMessages'
import {
  summaryGetConfig,
  summaryListModels,
  summarySaveConfig,
  summaryTestConnection,
} from '@/services/ipc'
import type { SummaryApiConfig } from '@/types'

interface CustomApiConfigFormProps {
  /** 保存成功后回调（携带已保存的配置，供父组件刷新「已配置」状态/摘要） */
  onSaved?: (config: SummaryApiConfig) => void
}

/**
 * 自定义 API 配置表单（设置页「自定义 LLM」tab）。
 * 受控组件：协议 openai/anthropic + 端点 + 密钥 + 模型 + 获取模型列表 + 测试连接 + 保存，
 * 加载/获取/测试/保存状态内部自管；挂载时自动读回已保存配置并预填表单
 * （仅 openai/anthropic 是自定义 API 配置；gateway 配置不带端点，表单保持默认不覆盖）。
 */
export function CustomApiConfigForm({ onSaved }: CustomApiConfigFormProps) {
  const t = useMessages()
  const [form, setForm] = useState<SummaryApiConfig>({
    protocol: 'openai',
    endpoint: '',
    apiKey: '',
    model: '',
  })
  const [fetchedModels, setFetchedModels] = useState<string[]>([])
  const [fetching, setFetching] = useState(false)
  const [testing, setTesting] = useState(false)
  const [saving, setSaving] = useState(false)

  // 挂载时读回已保存配置并预填表单
  useEffect(() => {
    let cancelled = false
    summaryGetConfig()
      .then((config) => {
        if (cancelled) return
        if (config && (config.protocol === 'openai' || config.protocol === 'anthropic')) {
          setForm(config)
        }
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [])

  // 当前表单内容（端点/模型去空格），供测试/拉模型/保存共用
  const currentForm = (): SummaryApiConfig => ({
    protocol: form.protocol,
    endpoint: form.endpoint.trim(),
    apiKey: form.apiKey,
    model: form.model.trim(),
  })

  const handleFetchModels = async () => {
    setFetching(true)
    try {
      const models = await summaryListModels(currentForm())
      setFetchedModels(models)
      if (models.length > 0 && !models.includes(form.model.trim())) {
        setForm((f) => ({ ...f, model: models[0] }))
      }
    } catch (e) {
      toast.error(t.sumFetchModelsFailed.replace('{error}', String(e)))
    } finally {
      setFetching(false)
    }
  }

  const handleTest = async () => {
    setTesting(true)
    try {
      await summaryTestConnection(currentForm())
      toast.success(t.sumApiTestOk)
    } catch (e) {
      toast.error(t.sumApiTestFailed.replace('{error}', String(e)))
    } finally {
      setTesting(false)
    }
  }

  // 保存：持久化到 settings（summary.api_config），并通知父组件
  const handleSave = async () => {
    setSaving(true)
    try {
      const config = currentForm()
      await summarySaveConfig(config)
      toast.success(t.sumApiSaved)
      onSaved?.(config)
    } catch (e) {
      toast.error(t.sumApiSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="flex flex-col gap-2">
      {/* 协议 + 端点 */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">{t.sumApiProtocol}</span>
          <Select value={form.protocol} onValueChange={(v) => setForm((f) => ({ ...f, protocol: v }))}>
            <SelectTrigger className="h-8 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="openai" className="text-xs">
                {t.sumProtocolOpenAI}
              </SelectItem>
              <SelectItem value="anthropic" className="text-xs">
                {t.sumProtocolAnthropic}
              </SelectItem>
            </SelectContent>
          </Select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">{t.sumApiEndpoint}</span>
          <Input
            className="h-8 text-xs"
            value={form.endpoint}
            placeholder={t.sumEndpointPlaceholder}
            onChange={(e) => setForm((f) => ({ ...f, endpoint: e.target.value }))}
          />
        </label>
      </div>

      {/* 密钥 */}
      <label className="flex flex-col gap-1">
        <span className="text-xs text-muted-foreground">{t.sumApiKey}</span>
        <Input
          type="password"
          className="h-8 text-xs"
          value={form.apiKey}
          onChange={(e) => setForm((f) => ({ ...f, apiKey: e.target.value }))}
        />
        <span className="text-[11px] text-muted-foreground/80">{t.sumApiKeyHint}</span>
      </label>

      {/* 模型 + 获取模型列表 */}
      <div className="flex flex-col gap-1">
        <span className="text-xs text-muted-foreground">{t.sumApiModel}</span>
        <div className="flex items-center gap-2">
          <Input
            className="h-8 flex-1 min-w-0 text-xs"
            value={form.model}
            onChange={(e) => setForm((f) => ({ ...f, model: e.target.value }))}
          />
          <Button
            variant="outline"
            size="sm"
            className="shrink-0"
            disabled={fetching || !form.endpoint.trim()}
            onClick={handleFetchModels}
          >
            {fetching ? t.comLoading : t.sumApiFetchModels}
          </Button>
        </div>
        {fetchedModels.length > 0 && (
          <Select value={form.model} onValueChange={(v) => setForm((f) => ({ ...f, model: v }))}>
            <SelectTrigger className="h-8 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {fetchedModels.map((m) => (
                <SelectItem key={m} value={m} className="text-xs">
                  {m}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      {/* 测试连接 + 保存 */}
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={testing || !form.endpoint.trim()}
          onClick={handleTest}
        >
          {testing ? t.comLoading : t.sumApiTest}
        </Button>
        <Button size="sm" disabled={saving || !form.endpoint.trim()} onClick={handleSave}>
          {saving ? t.comLoading : t.comSave}
        </Button>
      </div>
    </div>
  )
}
