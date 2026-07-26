'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { getRemoteAsrConfig, setRemoteAsrEndpoint, checkRemoteAsrHealth } from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 设置页 API tab：远程 ASR endpoint 配置（功能自原 AdvancedSection 搬移，保持不变） */
export function RemoteAsrSection() {
  const t = useMessages()
  const [endpoint, setEndpoint] = useState('')
  const [remoteModel, setRemoteModel] = useState('')
  const [health, setHealth] = useState<boolean | null>(null)
  const [saving, setSaving] = useState(false)
  const [checking, setChecking] = useState(false)

  useEffect(() => {
    getRemoteAsrConfig()
      .then((cfg) => {
        setEndpoint(cfg.endpoint || '')
        setRemoteModel(cfg.model || '')
      })
      .catch(() => {})
  }, [])

  const handleSave = async () => {
    const value = endpoint.trim()
    if (!value) {
      toast.error(t.setEnterEndpoint)
      return
    }
    setSaving(true)
    try {
      await setRemoteAsrEndpoint(value)
      toast.success(t.setEndpointSaved)
    } catch (e) {
      toast.error(t.setSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  const handleCheck = async () => {
    const value = endpoint.trim()
    if (!value) {
      toast.error(t.setEnterEndpoint)
      return
    }
    setChecking(true)
    setHealth(null)
    try {
      setHealth(await checkRemoteAsrHealth(value))
    } catch {
      setHealth(false)
    } finally {
      setChecking(false)
    }
  }

  return (
    <SettingsSection title={t.setRemoteAsrTitle}>
      <div className="flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.setRemoteAsrAddress}</span>
        <div className="flex items-center gap-2">
          <Input
            className="flex-1 min-w-0"
            value={endpoint}
            onChange={(e) => {
              setEndpoint(e.target.value)
              setHealth(null)
            }}
            placeholder="http://192.168.1.100:8000"
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
        {remoteModel && (
          <span className="text-xs text-muted-foreground/70">{t.setModelLabel.replace('{name}', remoteModel)}</span>
        )}
      </div>

      <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-700">
        {t.setRemoteNote}
      </div>
    </SettingsSection>
  )
}
