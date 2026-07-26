'use client'

// 历史记录页共用的时间格式化工具

import { useEffect, useState } from 'react'
import { LANGUAGE_LOCALE } from '@/i18n/languages'
import { useLanguageStore } from '@/stores/languageStore'

const pad = (n: number) => String(n).padStart(2, '0')

function parseDate(iso: string): Date | null {
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? null : d
}

/** created_at → 本地化短日期时间（列表用），如 en「Jul 20, 02:35 PM」、zh「7月20日 14:35」 */
export function formatCreatedAt(iso: string, locale: string): string {
  const d = parseDate(iso)
  if (!d) return iso.slice(0, 16)
  return new Intl.DateTimeFormat(locale, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(d)
}

/** created_at → 本地化完整日期时间（详情元信息用），如 en「Jul 20, 2026, 2:35 PM」、zh「2026年7月20日 14:35」 */
export function formatCreatedAtFull(iso: string, locale: string): string {
  const d = parseDate(iso)
  if (!d) return iso
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(d)
}

/** duration_ms → mm:ss */
export function formatDurationMs(ms?: number | null): string {
  if (ms == null || ms < 0) return '--:--'
  const total = Math.floor(ms / 1000)
  return `${pad(Math.floor(total / 60))}:${pad(total % 60)}`
}

/** 片段 start_ms → mm:ss */
export function formatSegmentTime(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  return `${pad(Math.floor(total / 60))}:${pad(total % 60)}`
}

/** 按当前 UI 语言输出日期的格式化器。与 useMessages 一致：首渲染固定英语，挂载后再切换，避免水合不一致。 */
export function useHistoryFormat() {
  const language = useLanguageStore((s) => s.language)
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])
  const locale = LANGUAGE_LOCALE[mounted ? language : 'en']
  return {
    formatCreatedAt: (iso: string) => formatCreatedAt(iso, locale),
    formatCreatedAtFull: (iso: string) => formatCreatedAtFull(iso, locale),
    formatDurationMs,
    formatSegmentTime,
  }
}
