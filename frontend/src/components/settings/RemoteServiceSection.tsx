'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Loader2, Pencil } from 'lucide-react'
import {
  getRemoteConfig,
  getRemoteEnabled,
  setRemoteEnabled,
  setRemoteEndpoint,
  listRemoteModels,
  type RemoteModelItem,
} from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingsSection } from './SettingsSection'
import { RemoteServerStatus } from '@/components/remote/RemoteServerStatus'
import { useMessages } from '@/i18n/useMessages'
import { formatModelPrice, remoteModelDisplayName } from '@/lib/remoteModelChoice'
import { dispatchRemoteConfigChanged, onRemoteConfigChanged } from '@/lib/remoteConfigSync'

/** 用户中心「远程服务」卡片：
 *  - 服务器状态行（默认只读）+ 编辑按钮：点开可改自定义地址（留空保存 = 恢复内置默认）
 *  - 总开关（授权码不在此显示/编辑：展示在左侧「我的授权码」卡，手动填码在欢迎弹窗 P2）
 *  - 只读展示可用模型（名称 / 模式 / 积分单价；模型选择请到各功能使用处）
 *    分组按网关下发的 usage 过滤：翻译专用模型（如豆包机器翻译）不进「总结」组，
 *    总结专用模型（如 DeepSeek Flash）不进「翻译」组（2026-09-24 修复）。
 *  测速功能已移除（误导大于价值，连通性看测试连接的延迟即可）
 */
export function RemoteServiceSection({ onChanged }: { onChanged?: () => void }) {
  const t = useMessages()
  const [serverUrl, setServerUrl] = useState('')
  const [isDefault, setIsDefault] = useState(true)
  const [customUrl, setCustomUrl] = useState('')
  const [enabled, setEnabled] = useState(false)
  const [health, setHealth] = useState<boolean | null>(null)
  const [modelList, setModelList] = useState<RemoteModelItem[]>([])
  // 服务器地址编辑态：默认只读，点铅笔进入编辑
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    Promise.all([
      getRemoteConfig().catch(() => null),
      getRemoteEnabled().catch(() => false),
    ]).then(([cfg, en]) => {
      if (cfg) {
        setServerUrl(cfg.serverUrl || '')
        setIsDefault(cfg.isDefault !== false)
        setCustomUrl(cfg.customServerUrl || '')
      }
      setEnabled(!!en)
    })
    // 他处（欢迎弹窗/设置页高级卡片）保存了远程配置 → 同步刷新本卡片
    return onRemoteConfigChanged((d) => {
      if (typeof d.serverUrl === 'string') setServerUrl(d.serverUrl)
      if (typeof d.isDefault === 'boolean') setIsDefault(d.isDefault)
      if (typeof d.enabled === 'boolean') setEnabled(d.enabled)
    })
  }, [])

  useEffect(() => {
    if (enabled) {
      listRemoteModels().then(setModelList).catch(() => setModelList([]))
    } else {
      setModelList([])
    }
  }, [enabled])

  const handleToggle = async (next: boolean) => {
    setEnabled(next)
    try {
      await setRemoteEnabled(next)
      dispatchRemoteConfigChanged({ enabled: next })
      onChanged?.()
    } catch (e) {
      setEnabled(!next)
      toast.error(t.accRemoteSaveFailed.replace('{error}', String(e)))
    }
  }

  // 保存服务器地址：draft 为用户自定义值，留空 = 恢复内置默认
  const handleSaveEndpoint = async (value: string) => {
    setSaving(true)
    try {
      await setRemoteEndpoint(value.trim())
      const cfg = await getRemoteConfig().catch(() => null)
      if (cfg) {
        setServerUrl(cfg.serverUrl || '')
        setIsDefault(cfg.isDefault !== false)
        setCustomUrl(cfg.customServerUrl || '')
        dispatchRemoteConfigChanged({ serverUrl: cfg.serverUrl, isDefault: cfg.isDefault })
      }
      setEditing(false)
      setHealth(null)
      toast.success(t.setEndpointSaved)
    } catch (e) {
      toast.error(t.accRemoteSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  // 模型分组展示：kinds = 网关 kind 字段；excludeUsage = 按网关 usage 字段排除
  // 「只做翻译」（豆包 MT）不进总结组、「只做总结」（DeepSeek Flash）不进翻译组；
  // 旧版网关不下发 usage 时按 both 处理（两组都显示，向后兼容）。
  const modelGroups: Array<{ key: string; label: string; kinds: string[]; excludeUsage?: string[] }> = [
    { key: 'asr', label: t.accModelKindAsr, kinds: ['asr'] },
    { key: 'translate', label: t.accModelKindTranslate, kinds: ['translate'], excludeUsage: ['summary'] },
    { key: 'summary', label: t.accModelKindSummary, kinds: ['translate'], excludeUsage: ['translate'] },
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
          {editing ? (
            <div className="flex flex-col gap-2">
              <Input
                className="h-8 text-xs"
                value={draft}
                placeholder="https://api.voxmin.top"
                autoFocus
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleSaveEndpoint(draft)
                  if (e.key === 'Escape') setEditing(false)
                }}
              />
              <div className="flex items-center gap-2">
                <Button size="sm" className="h-7 text-xs" disabled={saving} onClick={() => void handleSaveEndpoint(draft)}>
                  {saving && <Loader2 className="h-3 w-3 animate-spin" />}
                  {t.comSave}
                </Button>
                <Button variant="outline" size="sm" className="h-7 text-xs" disabled={saving} onClick={() => setEditing(false)}>
                  {t.comCancel}
                </Button>
                {!isDefault && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs"
                    disabled={saving}
                    onClick={() => void handleSaveEndpoint('')}
                  >
                    {t.setRemoteRestoreDefault}
                  </Button>
                )}
              </div>
              <p className="text-[11px] text-muted-foreground/70">{t.accEndpointEmptyHint}</p>
            </div>
          ) : (
            <div className="flex items-center gap-1">
              {/* 只读服务器状态行 + 三态测试按钮；旁边铅笔进入编辑 */}
              <div className="min-w-0 flex-1">
                <RemoteServerStatus serverUrl={serverUrl} isDefault={isDefault} onHealthChange={setHealth} />
              </div>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 w-7 shrink-0 px-0"
                title={t.accEditEndpoint}
                onClick={() => {
                  setDraft(customUrl)
                  setEditing(true)
                }}
              >
                <Pencil className="h-3.5 w-3.5" />
              </Button>
            </div>
          )}
        </div>

        {/* 可用模型（只读展示：名称 / 模式 / 积分单价） */}
        {enabled && modelList.length > 0 && (
          <div className="flex flex-col gap-2 pt-1">
            <span className="text-xs text-muted-foreground">{t.accModelsTitle}</span>
            {modelGroups.map((g) => {
              const models = modelList.filter(
                (m) => g.kinds.includes(m.kind) && !(g.excludeUsage ?? []).includes(m.usage ?? 'both')
              )
              if (models.length === 0) return null
              return (
                <div key={g.key} className="text-xs">
                  <span className="font-medium">{g.label}</span>
                  <div className="mt-1 flex flex-col gap-0.5">
                    {models.map((m) => (
                      <div key={`${m.kind}:${m.id}`} className="flex items-center gap-2 text-muted-foreground">
                        <span className="truncate min-w-0">{remoteModelDisplayName(m)}</span>
                        {m.mode === 'streaming' && (
                          <span className="shrink-0 text-primary/80">· {t.accModeStreaming}</span>
                        )}
                        <span className="shrink-0 text-muted-foreground/60">{formatModelPrice(m, t)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )
            })}
            <p className="text-[11px] text-muted-foreground/70">{t.accModelsHint}</p>
          </div>
        )}
      </div>
    </SettingsSection>
  )
}
