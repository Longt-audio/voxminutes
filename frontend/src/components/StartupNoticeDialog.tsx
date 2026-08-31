'use client'

import { useCallback, useEffect, useState } from 'react'
import {
  fetchRemoteMessages,
  apiGetSettings,
  apiSaveSetting,
  type RemoteMessage,
} from '@/services/ipc'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** type → 颜色（与底部短信息一致）：tip 蓝 / announcement 绿 / update 黄。 */
function typeColor(type: string): string {
  if (type === 'announcement') return 'text-emerald-600'
  if (type === 'update') return 'text-amber-600'
  return 'text-blue-600'
}
function typeLabel(type: string, t: ReturnType<typeof useMessages>): string {
  if (type === 'announcement') return t.msgTypeAnnouncement
  if (type === 'update') return t.msgTypeUpdate
  return t.msgTypeTip
}

/** 设置键：记录用户选择「不再显示」的启动弹窗消息 id；以及是否禁用启动弹窗。 */
const DISMISSED_ID_KEY = 'startup_notice.dismissed_id'
const DISABLED_KEY = 'startup_notice.disabled'

/** 启动时长信息弹窗（VSCode 风格）：
 *  每次打开软件时，若有新的 channel=startup 推送消息，弹窗展示（支持图片）。
 *  「关闭」本次关闭；「不再显示本条」记住该消息 id；「不再显示」禁用后续启动弹窗（可在设置中恢复）。
 *  所有推送消息（短/长）都可在用户中心「信息发布」区域再次查看。 */
export function StartupNoticeDialog() {
  const t = useMessages()
  const [message, setMessage] = useState<RemoteMessage | null>(null)
  const [open, setOpen] = useState(false)

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const settings = await apiGetSettings()
        if (settings[DISABLED_KEY]) return
        const res = await fetchRemoteMessages()
        // 取最新一条启动弹窗消息
        const startup = (res.announcements || [])
          .filter((m) => m.channel === 'startup')
          .sort((a, b) => (b.created_at || '').localeCompare(a.created_at || ''))[0]
        if (!startup || cancelled) return
        const dismissedId = settings[DISMISSED_ID_KEY]
        if (dismissedId === startup.id) return // 本条已选「不再显示」
        setMessage(startup)
        setOpen(true)
      } catch {
        // 网关未配置/不可达：静默忽略
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  const closeOnly = useCallback(() => {
    setOpen(false)
  }, [])

  const dismissThis = useCallback(() => {
    if (message) {
      apiSaveSetting(DISMISSED_ID_KEY, message.id).catch(() => {})
    }
    setOpen(false)
  }, [message])

  const disableAll = useCallback(() => {
    apiSaveSetting(DISABLED_KEY, 'true').catch(() => {})
    setOpen(false)
  }, [])

  if (!message) return null

  return (
    <Dialog open={open} onOpenChange={(o) => (o ? setOpen(true) : closeOnly())}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <div className={`text-xs font-medium ${typeColor(message.type)}`}>[{typeLabel(message.type, t)}]</div>
          <DialogTitle>{message.title}</DialogTitle>
        </DialogHeader>
        {message.image && (
          <div className="rounded-md overflow-hidden border border-border/60">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src={message.image} alt={message.title} className="w-full max-h-64 object-cover" />
          </div>
        )}
        {message.body && (
          <DialogDescription className="text-sm whitespace-pre-wrap">{message.body}</DialogDescription>
        )}
        <div className="flex items-center justify-between gap-2 pt-2">
          <Button variant="ghost" size="sm" onClick={disableAll}>
            {t.onbNoticeDisableAll}
          </Button>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={dismissThis}>
              {t.onbNoticeDismissThis}
            </Button>
            <Button size="sm" onClick={closeOnly}>
              {t.onbNoticeClose}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

/** 设置页「恢复启动弹窗」开关用：读取/设置 startup_notice.disabled。 */
export async function getStartupNoticeDisabled(): Promise<boolean> {
  const settings = await apiGetSettings().catch(() => ({} as Record<string, string>))
  return !!settings[DISABLED_KEY]
}
export async function setStartupNoticeDisabled(disabled: boolean): Promise<void> {
  await apiSaveSetting(DISABLED_KEY, disabled ? 'true' : null).catch(() => {})
}
