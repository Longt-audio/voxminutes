'use client'

import { useCallback, useEffect, useState } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { apiGetSettings, apiSaveSetting, fetchNoticeDocuments, type NoticeDocument } from '@/services/ipc'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 设置键：是否禁用启动弹窗（弹窗里「不再显示」= 设置这个键）。 */
const DISABLED_KEY = 'startup_notice.disabled'

/** 设置页「再次弹出」通过该窗口事件让弹窗重新拉取并显示（不受禁用开关影响）。 */
export const RESHOW_STARTUP_NOTICE_EVENT = 'vox:reshow-startup-notice'

/** 启动时长信息弹窗（文档式分页）：
 *  - 每次打开软件时拉取启动文档列表（本地缓存优先，离线可读；后台有新版本则更新缓存）。
 *  - 分页显示：最新文档在第一页，老文档往后翻页。
 *  - 图片按原始比例显示（object-contain，不拉伸）。
 *  「关闭」本次关闭；「不再显示」全局禁用启动弹窗（可在设置中恢复）。
 *  设置页可点「再次弹出」强制显示。 */
export function StartupNoticeDialog() {
  const t = useMessages()
  const [docs, setDocs] = useState<NoticeDocument[]>([])
  const [index, setIndex] = useState(0)
  const [open, setOpen] = useState(false)

  useEffect(() => {
    let cancelled = false

    const load = async (ignoreDisabled: boolean) => {
      try {
        const settings = await apiGetSettings()
        if (!ignoreDisabled && settings[DISABLED_KEY]) return
        const items = await fetchNoticeDocuments()
        if (!items.length || cancelled) return
        setDocs(items)
        setIndex(0)
        setOpen(true)
      } catch {
        // 网关未配置/不可达：静默忽略
      }
    }

    void load(false)

    // 设置页「再次弹出」：忽略禁用开关，强制显示
    const onReshow = () => void load(true)
    window.addEventListener(RESHOW_STARTUP_NOTICE_EVENT, onReshow)

    return () => {
      cancelled = true
      window.removeEventListener(RESHOW_STARTUP_NOTICE_EVENT, onReshow)
    }
  }, [])

  const closeOnly = useCallback(() => setOpen(false), [])

  /** 「不再显示」（VSCode 风格，单一按钮）：全局禁用启动弹窗，可在设置中恢复。 */
  const dismissForever = useCallback(() => {
    apiSaveSetting(DISABLED_KEY, 'true').catch(() => {})
    setOpen(false)
  }, [])

  if (!docs.length) return null

  const doc = docs[Math.min(index, docs.length - 1)]
  const total = docs.length

  return (
    <Dialog open={open} onOpenChange={(o) => (o ? setOpen(true) : closeOnly())}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{doc.title}</DialogTitle>
          <p className="text-xs text-muted-foreground">
            第 {index + 1} / {total} 页 · {doc.updated_at ? new Date(doc.updated_at).toLocaleString() : ''}
          </p>
        </DialogHeader>
        {doc.images && doc.images.length > 0 && (
          <div className="flex flex-col gap-2">
            {doc.images.map((img, i) => (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                key={i}
                src={img}
                alt={`${doc.title} 图 ${i + 1}`}
                className="w-full max-h-56 object-contain rounded-md border border-border/60"
              />
            ))}
          </div>
        )}
        {doc.body && <p className="text-sm whitespace-pre-wrap">{doc.body}</p>}

        <div className="flex items-center justify-between gap-2 pt-2">
          <div className="flex items-center gap-1">
            {total > 1 && (
              <>
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7 px-2"
                  disabled={index <= 0}
                  onClick={() => setIndex((i) => i - 1)}
                >
                  <ChevronLeft className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7 px-2"
                  disabled={index >= total - 1}
                  onClick={() => setIndex((i) => i + 1)}
                >
                  <ChevronRight className="h-3.5 w-3.5" />
                </Button>
              </>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={dismissForever}>
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
