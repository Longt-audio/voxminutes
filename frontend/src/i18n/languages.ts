/** 支持的语言。默认英语；用户选择由 languageStore 持久化到 localStorage。 */

export type Language = 'en' | 'zh' | 'ko' | 'ja'

export const LANGUAGE_OPTIONS: { value: Language; label: string }[] = [
  { value: 'en', label: 'English' },
  { value: 'zh', label: '中文' },
  { value: 'ko', label: '한국어' },
  { value: 'ja', label: '日本語' },
]

/** Language -> BCP 47 locale（用于 Intl 日期/数字格式化） */
export const LANGUAGE_LOCALE: Record<Language, string> = {
  en: 'en-US',
  zh: 'zh-CN',
  ko: 'ko-KR',
  ja: 'ja-JP',
}
