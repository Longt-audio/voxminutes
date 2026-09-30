import type { Language } from '@/i18n/languages'

/** 四段式分段顺序：中文|English|한국어|日本語 */
const SEGMENT_LANG_ORDER: Language[] = ['zh', 'en', 'ko', 'ja']

/**
 * 网关四段式短文本「中文|English|한국어|日本語」→ 按界面语言取对应段。
 * 缺段回退：英文段（parts[1]）→ 第一段；无 `|` 时原样返回（未本地化的旧内容）。
 * 仅用于短文本（模型 tags / languages / 版本 notes / 底部短消息）；
 * 长文档（欢迎词、公告）走接口的 ?lang= 参数，不要用本函数切（正文可能含表格等 `|`）。
 */
export function pickLangSegment(text: string, lang: Language): string {
  const trimmed = text.trim()
  if (!trimmed.includes('|')) return trimmed
  const parts = trimmed.split('|').map((s) => s.trim())
  const idx = SEGMENT_LANG_ORDER.indexOf(lang)
  return parts[idx] || parts[1] || parts[0] || trimmed
}
