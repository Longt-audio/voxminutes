// asrLanguages.ts
//
// 「识别语言」的客户端侧知识：规范语言码列表、本地模型的支持范围、以及语言名的本地化显示。
//
// 分工（与网关 `asrLanguages.ts` 对应）：
//   · **客户端**只使用/发送**规范码**（ISO-639-1 + `yue`），并只展示该模型支持的语言；
//     支持列表来自网关模型目录的 `language_codes`（远程模型）或本文件的本地表。
//   · **网关**负责把规范码翻成各上游真正的 API 代码（豆包 `zh-CN`、Deepgram 粤语 `zh-HK`、
//     千问/Deepgram/MiMo 的 ISO 码等）——各上游代码不统一，这份映射只应存在一处。

/** 规范码的展示顺序（覆盖网关所有 ASR 协议支持语言的并集，与网关表保持一致）。 */
export const LANGUAGE_ORDER = [
  'zh', 'en', 'yue',
  'ja', 'ko', 'vi', 'th', 'id', 'ms', 'tl',
  'hi', 'ar',
  'fr', 'de', 'es', 'pt', 'it', 'nl',
  'ru', 'uk', 'pl', 'cs', 'sk', 'hu', 'ro', 'bg', 'hr',
  'sv', 'da', 'fi', 'no', 'el',
  'tr',
]

/** 本地 ASR 模型支持的语言（远程模型用网关下发的 language_codes）。
 *  · SenseVoice：zh / en / ja / ko / yue（模型官方支持 5 语种）
 *  · X-ASR：流式 transducer，zh / en（含标点模型） */
export const LOCAL_ASR_LANGUAGE_SUPPORT: Record<string, string[]> = {
  'sense-voice': ['zh', 'en', 'ja', 'ko', 'yue'],
  'x-asr': ['zh', 'en'],
}

/** 按展示顺序排序 + 去重。 */
export function sortLanguages(codes: string[]): string[] {
  const set = new Set(codes)
  const ordered = LANGUAGE_ORDER.filter((c) => set.has(c))
  // 顺序表里没有的（未来新增）追加在后面，保证不会丢
  const extra = [...set].filter((c) => !LANGUAGE_ORDER.includes(c)).sort()
  return [...ordered, ...extra]
}

/**
 * 语言码 → 本地化显示名。优先用 Intl.DisplayNames（CLDR 全量、免维护 i18n 词条），
 * 不支持时回退到内置的中/英名表。
 * @param code 规范码（zh / en / yue …）
 * @param locale BCP-47 界面语言（zh / en / ko / ja）
 */
const FALLBACK_NAMES: Record<string, { zh: string; en: string }> = {
  zh: { zh: '中文', en: 'Chinese' },
  en: { zh: '英语', en: 'English' },
  yue: { zh: '粤语', en: 'Cantonese' },
  ja: { zh: '日语', en: 'Japanese' },
  ko: { zh: '韩语', en: 'Korean' },
  vi: { zh: '越南语', en: 'Vietnamese' },
  th: { zh: '泰语', en: 'Thai' },
  id: { zh: '印尼语', en: 'Indonesian' },
  ms: { zh: '马来语', en: 'Malay' },
  tl: { zh: '菲律宾语', en: 'Filipino' },
  hi: { zh: '印地语', en: 'Hindi' },
  ar: { zh: '阿拉伯语', en: 'Arabic' },
  fr: { zh: '法语', en: 'French' },
  de: { zh: '德语', en: 'German' },
  es: { zh: '西班牙语', en: 'Spanish' },
  pt: { zh: '葡萄牙语', en: 'Portuguese' },
  it: { zh: '意大利语', en: 'Italian' },
  nl: { zh: '荷兰语', en: 'Dutch' },
  ru: { zh: '俄语', en: 'Russian' },
  uk: { zh: '乌克兰语', en: 'Ukrainian' },
  pl: { zh: '波兰语', en: 'Polish' },
  cs: { zh: '捷克语', en: 'Czech' },
  sk: { zh: '斯洛伐克语', en: 'Slovak' },
  hu: { zh: '匈牙利语', en: 'Hungarian' },
  ro: { zh: '罗马尼亚语', en: 'Romanian' },
  bg: { zh: '保加利亚语', en: 'Bulgarian' },
  hr: { zh: '克罗地亚语', en: 'Croatian' },
  sv: { zh: '瑞典语', en: 'Swedish' },
  da: { zh: '丹麦语', en: 'Danish' },
  fi: { zh: '芬兰语', en: 'Finnish' },
  no: { zh: '挪威语', en: 'Norwegian' },
  el: { zh: '希腊语', en: 'Greek' },
  tr: { zh: '土耳其语', en: 'Turkish' },
}

export function languageLabel(code: string, locale: string): string {
  try {
    const dn = new Intl.DisplayNames([locale], { type: 'language' })
    const name = dn.of(code)
    if (name && name !== code) return name
  } catch {
    // Intl.DisplayNames 不可用 → 回退
  }
  const fb = FALLBACK_NAMES[code]
  if (!fb) return code
  return locale.startsWith('zh') ? fb.zh : fb.en
}
