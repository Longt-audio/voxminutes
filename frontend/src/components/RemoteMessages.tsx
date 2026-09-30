'use client'

import { useEffect, useMemo, useState } from 'react'
import { X } from 'lucide-react'
import { fetchRemoteMessages, openExternalUrl, type RemoteMessage } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { pickLangSegment } from '@/lib/langSegment'

interface FooterItem {
  id: string
  title: string
  body: string
  type: string
  url?: string
}

/** type → 颜色：tip 蓝 / announcement 绿 / update 黄（三类推送消息的语义区分）。 */
function typeColor(type: string): string {
  if (type === 'announcement') return 'text-emerald-600'
  if (type === 'update') return 'text-amber-600'
  return 'text-blue-600' // tip
}

function typeLabel(type: string, t: ReturnType<typeof useMessages>): string {
  if (type === 'announcement') return t.msgTypeAnnouncement
  if (type === 'update') return t.msgTypeUpdate
  return t.msgTypeTip
}

/** 底部推送条：只显示网关推送的「底部短信息」（channel=footer），无任何内置默认文案——
 *  未联网 / 网关无推送时渲染空占位（保持 footer justify-between 布局不抖动，版本号常驻右侧）。
 *  启动公告文档已并入欢迎弹窗 WelcomeDialog；更新提醒由顶部 UpdateBanner 承担。 */
export function RemoteMessages() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const [messages, setMessages] = useState<RemoteMessage[]>([])
  const [dismissed, setDismissed] = useState<Set<string>>(new Set())
  const [index, setIndex] = useState(0)

  // 按界面语言拉取（?lang=）；语言切换后重新拉取
  useEffect(() => {
    fetchRemoteMessages(lang)
      .then((r) => setMessages(r.announcements || []))
      .catch(() => {
        // 网关未配置/不可达：静默忽略
      })
  }, [lang])

  const items = useMemo<FooterItem[]>(() => {
    // 只显示「底部短信息」渠道（channel 缺省视为 footer，兼容旧数据）；
    // 短消息 title/body 支持四段式「中文|English|한국어|日本語」，按界面语言取段
    return messages
      .filter((m) => (m.channel || 'footer') === 'footer')
      .filter((m) => !dismissed.has(m.id))
      .map((m) => ({
        id: m.id,
        title: pickLangSegment(m.title, lang),
        body: pickLangSegment(m.body, lang),
        type: m.type,
      }))
  }, [messages, dismissed, lang])

  // 轮播：多则每 5 秒切一条
  useEffect(() => {
    if (items.length <= 1) {
      setIndex(0)
      return
    }
    const timer = setInterval(() => setIndex((i) => (i + 1) % items.length), 5000)
    return () => clearInterval(timer)
  }, [items.length])

  const item = items.length > 0 ? items[Math.min(index, items.length - 1)] : null

  if (!item) {
    // 无推送：不占内容但保留占位元素，footer 为 justify-between，
    // 返回 null 会让右侧版本号/录音指示跳到左侧
    return <span className="min-w-0 flex-1" />
  }

  const dismiss = () => setDismissed((prev) => new Set(prev).add(item.id))

  return (
    <span className="flex min-w-0 items-center gap-2 text-[10px] text-muted-foreground/80">
      <span className={`shrink-0 font-medium ${typeColor(item.type)}`}>[{typeLabel(item.type, t)}]</span>
      <span className="truncate">
        <span className="font-medium">{item.title}</span>
        {item.body && <span> · {item.body}</span>}
        {item.url && (
          // 同 UpdateBanner：`<a target="_blank">` 在 Tauri 里会被静默取消，改走系统打开
          <button
            type="button"
            className="ml-1 underline underline-offset-2 hover:text-foreground"
            onClick={() => void openExternalUrl(String(item.url)).catch(() => {})}
          >
            {t.comDownload}
          </button>
        )}
      </span>
      <button className="shrink-0 opacity-50 hover:opacity-100" onClick={dismiss} title={t.comClose}>
        <X className="h-3 w-3" />
      </button>
    </span>
  )
}
