'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'
import { checkRemoteAsrHealth, getRemoteConfig, setRemoteEndpoint } from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useMessages } from '@/i18n/useMessages'
import { dispatchRemoteConfigChanged } from '@/lib/remoteConfigSync'

/** 设置页「自定义 LLM」tab 顶部的高级卡片：自定义远程服务器地址。
 *  输入框值为「用户自定义地址」（留空 = 用内置默认 https://api.voxmin.top）；
 *  保存走 set_remote_endpoint（只动地址，不动授权码/模型选择），
 *  保存后广播 remoteConfigSync 事件让欢迎弹窗/账户页的只读状态行同步。 */
export function RemoteServerCard() {
  const t = useMessages()
  // 输入框内容 = 自定义地址（空 = 默认）；effective = 当前生效地址（只读展示）
  const [custom, setCustom] = useState('')
  const [effective, setEffective] = useState('')
  const [saving, setSaving] = useState(false)
  const [health, setHealth] = useState<boolean | null>(null)
  const [checking, setChecking] = useState(false)

  useEffect(() => {
    getRemoteConfig()
      .then((cfg) => {
        setCustom(cfg.customServerUrl || '')
        setEffective(cfg.serverUrl || '')
      })
      .catch(() => {})
  }, [])

  // 保存/恢复默认共用：写后端 → 重读配置 → 广播同步
  const apply = async (value: string) => {
    setSaving(true)
    try {
      await setRemoteEndpoint(value.trim())
      const cfg = await getRemoteConfig().catch(() => null)
      if (cfg) {
        setCustom(cfg.customServerUrl || '')
        setEffective(cfg.serverUrl || '')
        dispatchRemoteConfigChanged({ serverUrl: cfg.serverUrl, isDefault: cfg.isDefault })
      }
      setHealth(null)
      toast.success(t.setEndpointSaved)
    } catch (e) {
      toast.error(t.setSaveFailed.replace('{error}', String(e)))
    } finally {
      setSaving(false)
    }
  }

  // 测输入框里的地址；留空时测当前生效地址
  const handleTest = async () => {
    const target = custom.trim() || effective
    if (!target) return
    setChecking(true)
    setHealth(null)
    try {
      // 这里只关心「服务器是否可达」；流式通道检查在账户页/欢迎弹窗的状态行里给出
      const res = await checkRemoteAsrHealth(target)
      setHealth(res.ok)
    } catch {
      setHealth(false)
    } finally {
      setChecking(false)
    }
  }

  return (
    <section className="flex flex-col gap-3 rounded-md border border-border/60 p-4">
      <div>
        <div className="text-sm font-medium">{t.setRemoteServerTitle}</div>
        <p className="mt-0.5 text-xs text-muted-foreground">{t.setRemoteServerDesc}</p>
      </div>
      <Input
        className="h-8 text-xs"
        value={custom}
        placeholder="https://api.voxmin.top"
        onChange={(e) => {
          setCustom(e.target.value)
          setHealth(null)
        }}
      />
      <div className="flex items-center gap-2">
        <Button size="sm" disabled={saving} onClick={() => void apply(custom)}>
          {saving ? t.setSaving : t.comSave}
        </Button>
        <Button variant="outline" size="sm" disabled={saving || !custom.trim()} onClick={() => void apply('')}>
          {t.setRemoteRestoreDefault}
        </Button>
        <Button variant="outline" size="sm" disabled={checking || saving} onClick={() => void handleTest()}>
          {checking && <Loader2 className="h-3 w-3 animate-spin" />}
          {checking ? t.setChecking : t.setTestConnection}
        </Button>
        {health !== null && (
          <Badge variant={health ? 'success' : 'destructive'}>{health ? t.setOnline : t.setOffline}</Badge>
        )}
      </div>
      {effective && (
        <p className="text-xs text-muted-foreground">{t.setRemoteServerCurrent.replace('{url}', effective)}</p>
      )}
    </section>
  )
}
