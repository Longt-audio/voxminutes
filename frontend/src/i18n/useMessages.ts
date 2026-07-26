'use client'

import { useEffect, useState } from 'react'
import { MESSAGES, type Messages } from './messages'
import { useLanguageStore } from '@/stores/languageStore'

/**
 * 读取当前 UI 语言的文案。
 * 首次渲染固定返回英语，避免 Next.js 静态导出时的水合不一致；
 * 挂载后再切换到用户持久化的语言。
 */
export function useMessages(): Messages {
  const language = useLanguageStore((s) => s.language)
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])
  return MESSAGES[mounted ? language : 'en']
}
