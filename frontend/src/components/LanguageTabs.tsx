'use client'

import { useEffect, useState } from 'react'
import { LANGUAGE_OPTIONS, type Language } from '@/i18n/messages'
import { useLanguageStore } from '@/stores/languageStore'
import { setTranslationHomeLang } from '@/services/ipc'
import { cn } from '@/lib/utils'

/** 四语言直选标签（欢迎弹窗 P1 用）：一眼可见全部支持语言，点击即切换并记住。
 *  与 LanguageSwitcher（下拉）同一套 store/副作用。 */
export function LanguageTabs() {
  const language = useLanguageStore((s) => s.language)
  const setLanguage = useLanguageStore((s) => s.setLanguage)
  // 与 LanguageSwitcher 同理：首次渲染按默认语言（en）展示，避免水合不一致
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])

  const current = mounted ? language : 'en'

  const handleChange = (v: Language) => {
    setLanguage(v)
    // UI 语言即翻译 home：同步后端（后端仅据此决定默认目标语言，不重置当前 target）
    setTranslationHomeLang(v).catch(() => {})
  }

  return (
    <div className="inline-flex items-center rounded-full border border-border/60 bg-muted/60 p-0.5">
      {LANGUAGE_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          type="button"
          onClick={() => handleChange(opt.value)}
          className={cn(
            'rounded-full px-3.5 py-1.5 text-sm transition-colors',
            opt.value === current
              ? 'font-medium bg-primary text-primary-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          )}
        >
          {opt.label}
        </button>
      ))}
    </div>
  )
}
