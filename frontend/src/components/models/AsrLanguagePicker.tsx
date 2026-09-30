'use client'

// 识别语言选择器（与 ASR 模型选择器配套）。
//
// 关键设计：**语言列表按所选模型过滤**，且客户端只发**规范码**（ISO-639-1 + yue）。
// 各上游真正需要的代码并不统一（豆包要 zh-CN/en-US，Deepgram 说粤语要 zh-HK，
// 千问/Deepgram/MiMo 用 ISO 码……），这份映射由网关按协议完成——见
// gateway/src/asrLanguages.ts 与 docs。这样新增模型/上游只改网关一张表。

import { Languages } from 'lucide-react'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { languageLabel, sortLanguages } from '@/lib/asrLanguages'
import { cn } from '@/lib/utils'

export interface AsrLanguagePickerProps {
  /** 当前语言（规范码）或 'auto' */
  value: string
  onChange: (code: string) => void
  /** 当前 ASR 模型支持的语言（规范码）。空数组 = 该模型未声明 → 只能自动检测。 */
  supported: string[]
  disabled?: boolean
  /** 显示「支持：xx / yy」小字（默认 true） */
  showSupported?: boolean
  className?: string
}

export function AsrLanguagePicker({
  value,
  onChange,
  supported,
  disabled,
  showSupported = true,
  className,
}: AsrLanguagePickerProps) {
  const t = useMessages()
  const locale = useLanguageStore((s) => s.language)
  const codes = sortLanguages(supported)

  return (
    <div className={cn('flex flex-col gap-1', className)}>
      <div className="flex items-center gap-1.5">
        <Languages className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        <span className="text-xs text-muted-foreground shrink-0">{t.recRecogLang}</span>
        <select
          className="h-7 min-w-[130px] rounded-md border border-input bg-background px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50"
          value={value || 'auto'}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled || codes.length === 0}
          title={codes.length === 0 ? t.recLangAutoOnly : undefined}
        >
          <option value="auto">{t.recLangAuto}</option>
          {codes.map((c) => (
            <option key={c} value={c}>
              {languageLabel(c, locale)}
            </option>
          ))}
        </select>
      </div>
      {showSupported && (
        <span className="text-[11px] text-muted-foreground/80 leading-snug">
          {codes.length === 0
            ? t.recLangAutoOnly
            : t.recLangSupported.replace('{list}', codes.map((c) => languageLabel(c, locale)).join(' / '))}
        </span>
      )}
    </div>
  )
}
