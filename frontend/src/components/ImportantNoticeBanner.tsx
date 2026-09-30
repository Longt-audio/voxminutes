'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { Megaphone, X } from 'lucide-react'
import { fetchImportantNotice, type ImportantNotice } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { pickLangSegment } from '@/lib/langSegment'

/** 轮询间隔（毫秒）：与 UpdateBanner 同一节奏。 */
const POLL_INTERVAL_MS = 60_000

/** localStorage 键：已关闭通知的指纹（updated_at 优先，否则文本哈希）。
 *  同一条内容关闭后不再弹出；后台改文本（updated_at 变化）后重新显示。 */
const DISMISSED_KEY = 'vox:importantNotice.dismissed'

/** 简单字符串哈希（updated_at 缺失时的文本指纹兜底）。 */
function hashText(s: string): string {
  let h = 0
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0
  }
  return `h${h}`
}

/** 重要信息推送横幅：客户端定时轮询网关 important-notice，text 非空才显示在页面顶部。
 *  关闭 X：按 updated_at（或文本指纹）持久记住，同一条内容不再弹出；
 *  空 text / 拉取失败 / 网关未配置 → 完全不渲染（不占位）。 */
export function ImportantNoticeBanner() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const [notice, setNotice] = useState<ImportantNotice | null>(null)
  const [dismissed, setDismissed] = useState<string | null>(null)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    try {
      setDismissed(localStorage.getItem(DISMISSED_KEY))
    } catch {
      // localStorage 不可用时按未关闭处理
    }
  }, [])

  const poll = useCallback(async () => {
    try {
      const res = await fetchImportantNotice(lang)
      // text 为空串 = 后台撤下，不再显示
      setNotice(res.text.trim() ? res : null)
    } catch {
      // 网关未配置/不可达：保持现状，静默忽略
    }
  }, [lang])

  // 启动时先拉一次 + 定时轮询（语言切换时以新语言重拉）
  useEffect(() => {
    void poll()
    timerRef.current = setInterval(() => void poll(), POLL_INTERVAL_MS)
    return () => {
      if (timerRef.current) clearInterval(timerRef.current)
    }
  }, [poll])

  if (!notice) return null

  // 不带 lang 时网关可能返回四段式「中文|English|한국어|日本語」，本地兜底切分
  const text = pickLangSegment(notice.text, lang)
  if (!text) return null

  // 指纹：updated_at 优先（语言切换不会因译文不同而重复弹出）；缺失时用文本哈希
  const fingerprint = notice.updated_at || hashText(notice.text)
  if (dismissed === fingerprint) return null

  const handleDismiss = () => {
    try {
      localStorage.setItem(DISMISSED_KEY, fingerprint)
    } catch {
      // 写入失败也至少在本次会话内关闭
    }
    setDismissed(fingerprint)
  }

  return (
    <div className="flex shrink-0 items-start gap-2 border-b border-amber-300 bg-amber-50 px-4 py-2 text-sm text-amber-900">
      <Megaphone className="mt-0.5 h-4 w-4 shrink-0" />
      <span className="flex-1 min-w-0 whitespace-pre-line">
        <span className="font-medium">{t.noticeLabel}</span>
        <span className="text-amber-800/90"> · {text}</span>
      </span>
      <button
        className="shrink-0 opacity-60 hover:opacity-100"
        onClick={handleDismiss}
        title={t.comClose}
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  )
}
