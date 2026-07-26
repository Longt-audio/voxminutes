import type { Messages } from '@/i18n/messages'
import type { Language } from '@/i18n/languages'
import type { TranslationEngine } from '@/types'

/** 各翻译引擎支持的目标语言代码（不再有 auto；源语言==目标语言时后端跳过/返回原文） */
export const OPUS_TARGET_LANGS = ['zh', 'en'] as const
export const HYMT2_TARGET_LANGS = [
  'zh',
  'en',
  'ja',
  'ko',
  'fr',
  'de',
  'es',
  'ru',
  'pt',
  'zh-Hant',
  'yue',
  'th',
  'vi',
] as const

/** 按引擎返回可选目标语言代码列表（全量，不排除 home） */
export function getTranslateTargetLangs(engine: TranslationEngine): string[] {
  return engine === 'hymt2' ? [...HYMT2_TARGET_LANGS] : [...OPUS_TARGET_LANGS]
}

/** home 对应的默认目标语言（仅用于初始值/非法值回退）：home 非英语 → 英语；home 是英语 → 中文 */
export function defaultTargetLang(home: Language): string {
  return home === 'en' ? 'zh' : 'en'
}

/** 目标语言代码 → 展示名（语言名本身做 i18n） */
export function translateTargetLangLabel(code: string, t: Messages): string {
  switch (code) {
    case 'zh':
      return t.trLangZh
    case 'en':
      return t.trLangEn
    case 'ja':
      return t.trLangJa
    case 'ko':
      return t.trLangKo
    case 'fr':
      return t.trLangFr
    case 'de':
      return t.trLangDe
    case 'es':
      return t.trLangEs
    case 'ru':
      return t.trLangRu
    case 'pt':
      return t.trLangPt
    case 'zh-Hant':
      return t.trLangZhHant
    case 'yue':
      return t.trLangYue
    case 'th':
      return t.trLangTh
    case 'vi':
      return t.trLangVi
    default:
      return code
  }
}
