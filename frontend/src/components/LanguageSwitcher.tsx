'use client'

import * as Select from '@radix-ui/react-select'
import { Check, ChevronDown, Languages } from 'lucide-react'
import { useEffect, useState } from 'react'
import { LANGUAGE_OPTIONS, type Language } from '@/i18n/messages'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { setTranslationHomeLang } from '@/services/ipc'

/** 标题栏右侧的语言切换入口（英/中/韩/日），选择会被记住 */
export function LanguageSwitcher() {
  const language = useLanguageStore((s) => s.language)
  const setLanguage = useLanguageStore((s) => s.setLanguage)
  const t = useMessages()
  // 与 useMessages 同理：首次渲染按默认语言（en）展示，避免水合不一致
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])

  const current = mounted ? language : 'en'
  const currentLabel = LANGUAGE_OPTIONS.find((o) => o.value === current)?.label ?? 'English'

  const handleChange = (v: Language) => {
    setLanguage(v)
    // UI 语言即翻译 home：同步后端（后端仅据此决定默认目标语言，不重置当前 target）
    setTranslationHomeLang(v).catch(() => {})
  }

  return (
    <Select.Root value={current} onValueChange={(v) => handleChange(v as Language)}>
      <Select.Trigger
        aria-label={t.languageLabel}
        title={t.languageLabel}
        className="flex items-center gap-1.5 h-8 px-2 rounded-md text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground outline-none"
      >
        <Languages className="h-3.5 w-3.5" />
        <span>{currentLabel}</span>
        <ChevronDown className="h-3 w-3 opacity-60" />
      </Select.Trigger>
      <Select.Portal>
        <Select.Content
          position="popper"
          sideOffset={4}
          align="end"
          className="z-50 min-w-[6.5rem] overflow-hidden rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
        >
          <Select.Viewport>
            {LANGUAGE_OPTIONS.map((opt) => (
              <Select.Item
                key={opt.value}
                value={opt.value}
                className="relative flex cursor-pointer select-none items-center rounded-sm py-1.5 pl-2 pr-7 text-xs outline-none transition-colors data-[highlighted]:bg-muted data-[highlighted]:text-foreground"
              >
                <Select.ItemText>{opt.label}</Select.ItemText>
                <Select.ItemIndicator className="absolute right-2">
                  <Check className="h-3.5 w-3.5" />
                </Select.ItemIndicator>
              </Select.Item>
            ))}
          </Select.Viewport>
        </Select.Content>
      </Select.Portal>
    </Select.Root>
  )
}
