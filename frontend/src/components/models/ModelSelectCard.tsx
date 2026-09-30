'use client'

import { useState } from 'react'
import { Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { pickLangSegment } from '@/lib/langSegment'
import type { Language } from '@/i18n/languages'

/**
 * 模型选择卡片（录音前确认弹窗 / 翻译模型子页面共用）。
 *
 * 名称独占一行完整显示（允许换行）；标签（badges）排在名称下方，约定：
 *  - local       离线免费（本地模型）
 *  - remote      在线远程（远程模型）
 *  - streaming   流式（边说边出字）
 *  - batch       非流式（说完一句话出字）
 *  - recommended 推荐（网关后台勾选）
 *
 * 网关下发的 `tags`（后台可配，最多渲染 4 个）非空时**替代** badges 渲染；
 * 缺字段或空数组回退 badges 逻辑，保证旧网关也能用。
 * 单个 tag 支持 `中文|English|한국어|日本語` 四段式写法，按界面语言取对应段
 * （缺段回退第一段）；「推荐」/ Recommended 自动走 i18n；超过 14 字符截断。
 *
 * 「支持语言」传 collapsibleLanguages 且内容过长时收起为一行，点击展开/收起。
 * 选中态：边框/底色高亮 + 右上角对勾圆点。
 */
export type ModelCardBadgeTone = 'local' | 'remote' | 'streaming' | 'batch' | 'recommended'

export interface ModelCardBadge {
  text: string
  tone: ModelCardBadgeTone
}

const toneVariant: Record<ModelCardBadgeTone, 'success' | 'secondary' | 'default' | 'outline' | 'warning'> = {
  local: 'success',
  remote: 'secondary',
  streaming: 'default',
  batch: 'outline',
  recommended: 'warning',
}

const TAG_MAX_CHARS = 14

/**
 * 网关 tag 本地化：
 * - `中文|English|한국어|日本語` 四段式 → pickLangSegment 按界面语言取段；
 * - 「推荐」/【推荐】/ Recommended（大小写不敏感）→ 走 i18n；
 * - 其他自由文本原样；超过 14 字符截断（防英文长词撑破卡片），全文放 title。
 */
export function localizedModelTag(tag: string, lang: Language, recommendedLabel: string): string {
  let text = tag.trim()
  if (text.includes('|')) {
    text = pickLangSegment(text, lang)
  } else {
    const norm = text.replace(/[【】\[\]\s]/g, '').toLowerCase()
    if (norm === '推荐' || norm === 'recommended') text = recommendedLabel
  }
  return text.length > TAG_MAX_CHARS ? `${text.slice(0, TAG_MAX_CHARS)}…` : text
}

export interface ModelSelectCardProps {
  title: string
  description?: string
  badges?: ModelCardBadge[]
  /** 网关下发的模型标签：非空时替代 badges 渲染（最多 4 个），空/缺省回退 badges */
  tags?: string[]
  /** 支持语言（调用方已格式化，如「支持语言：中 / 英 / 日 / 韩 / 粤」） */
  languages?: string
  /** 积分消耗（远程模型，调用方用 formatModelPrice 格式化） */
  priceLabel?: string
  /** 「支持语言」内容长时收起为一行，点击可展开/收起（默认 false，保持原有平铺） */
  collapsibleLanguages?: boolean
  active: boolean
  disabled?: boolean
  onClick?: () => void
}

export function ModelSelectCard({
  title,
  description,
  badges,
  tags,
  languages,
  priceLabel,
  collapsibleLanguages = false,
  active,
  disabled,
  onClick,
}: ModelSelectCardProps) {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const recommendedLabel = t.recRecommended.replace(/[【】\[\]]/g, '').trim()
  const [langsExpanded, setLangsExpanded] = useState(false)
  const langsLong = !!languages && languages.length > 28
  // 网关 tags 优先（最多 4 个）；缺字段/空数组回退内置 badges
  const gatewayTags = tags && tags.length > 0 ? tags.slice(0, 4) : null
  // recommended 是结构化开关（还控制置顶排序），不能被自由文本 tags 覆盖：
  // tags 存在时也始终渲染推荐徽标，除非运营已在 tags 里手写了「推荐」。
  const recommendedBadge = badges?.find((b) => b.tone === 'recommended')
  const tagsMentionRecommended =
    gatewayTags?.some((tag) => /推荐|recommended/i.test(tag.replace(/[【】\[\]]/g, ''))) ?? false
  const showRecommendedBadge = !!recommendedBadge && !!gatewayTags && !tagsMentionRecommended
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={cn(
        'relative w-full rounded-lg border px-2.5 py-2 text-left transition-all',
        active
          ? 'border-primary bg-primary/5 ring-1 ring-primary'
          : 'border-border/60 bg-card/50 hover:border-primary/30',
        disabled && 'cursor-not-allowed opacity-50'
      )}
    >
      {active && (
        <span className="absolute -top-1.5 -right-1.5 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-primary-foreground shadow">
          <Check className="h-2.5 w-2.5" />
        </span>
      )}
      {/* 名称独占一行：完整显示（允许换行），标签移到名称下方，避免同行挤压截断 */}
      <div className="min-w-0 text-sm font-semibold leading-snug break-words">{title}</div>
      {gatewayTags ? (
        <div className="mt-1 flex flex-wrap gap-1">
          {showRecommendedBadge && (
            <Badge variant={toneVariant.recommended} className="max-w-full px-1.5 py-0 text-[10px] whitespace-normal break-words">
              {recommendedBadge.text.trim()}
            </Badge>
          )}
          {gatewayTags.map((tag, i) => {
            const text = localizedModelTag(tag, lang, recommendedLabel)
            return (
              <Badge
                key={`tag-${i}`}
                variant="secondary"
                className="max-w-full px-1.5 py-0 text-[10px] whitespace-normal break-words"
                title={tag.includes('|') ? tag : undefined}
              >
                {text}
              </Badge>
            )
          })}
        </div>
      ) : (
        badges &&
        badges.length > 0 && (
          <div className="mt-1 flex flex-wrap gap-1">
            {badges.map((b, i) => (
              <Badge
                key={`${b.tone}-${i}`}
                variant={toneVariant[b.tone]}
                className="max-w-full px-1.5 py-0 text-[10px] whitespace-normal break-words"
              >
                {b.text}
              </Badge>
            ))}
          </div>
        )
      )}
      {description && <div className="mt-0.5 text-[11px] leading-tight text-muted-foreground">{description}</div>}
      {(languages || priceLabel) && (
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] leading-tight text-muted-foreground/80">
          {languages && (
            <span className="flex min-w-0 items-center gap-1">
              <span
                className={cn('min-w-0', collapsibleLanguages && langsLong && !langsExpanded ? 'truncate' : 'break-words')}
                title={collapsibleLanguages && langsLong && !langsExpanded ? languages : undefined}
              >
                {languages}
              </span>
              {collapsibleLanguages && langsLong && (
                <span
                  role="button"
                  tabIndex={-1}
                  className="shrink-0 cursor-pointer text-primary hover:underline"
                  onClick={(e) => {
                    // 卡片本体是 button，展开/收起不能触发选中
                    e.stopPropagation()
                    setLangsExpanded((v) => !v)
                  }}
                >
                  {langsExpanded ? t.recLangsCollapse : t.recLangsExpand}
                </span>
              )}
            </span>
          )}
          {priceLabel && <span className="tabular-nums">{priceLabel}</span>}
        </div>
      )}
    </button>
  )
}
