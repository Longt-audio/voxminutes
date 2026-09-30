'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { listRemoteModels, summaryGetConfig, summarySaveConfig, type RemoteModelItem } from '@/services/ipc'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'
import { remoteModelOptionLabel } from '@/lib/remoteModelChoice'
import { catalogByKind } from '@/stores/remoteCatalogStore'

/** 设置页：会议总结 / AI —— 走远程服务网关（去掉了自定义 API），选择总结模型。
 *  与远程 ASR/翻译/TTS 平行：列出网关已上架的 LLM 模型并显示积分单价。 */
export function SummarySection() {
  const t = useMessages()
  const [model, setModel] = useState('')
  const [models, setModels] = useState<RemoteModelItem[]>([])
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    listRemoteModels()
      .then((list) => setModels(catalogByKind(list, 'translate')))
      .catch(() => {})
    summaryGetConfig()
      .then((c) => {
        if (c?.model) setModel(c.model)
      })
      .catch(() => {})
  }, [])

  const handleSave = async () => {
    setSaving(true)
    try {
      await summarySaveConfig({ protocol: 'gateway', endpoint: '', apiKey: '', model: model.trim() })
      toast.success(t.sumApiSaved)
    } catch (e) {
      toast.error(t.sumApiSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  return (
    <SettingsSection title={t.sumSettingsTitle} description={t.sumSettingsHint}>
      <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-700">
        {t.sumRemoteNote}
      </div>

      <div className="mt-4 flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.sumApiModel}</span>
        {models.length > 0 ? (
          <Select value={model} onValueChange={setModel}>
            <SelectTrigger className="text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {models.map((m) => (
                <SelectItem key={m.id} value={m.id}>
                  {remoteModelOptionLabel(m, t)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <Input value={model} onChange={(e) => setModel(e.target.value)} placeholder="deepseek-chat" />
        )}
      </div>

      <div className="mt-4">
        <Button disabled={saving} onClick={handleSave}>
          {saving ? t.comLoading : t.comSave}
        </Button>
      </div>
    </SettingsSection>
  )
}
