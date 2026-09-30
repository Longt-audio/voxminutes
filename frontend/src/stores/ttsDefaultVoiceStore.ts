'use client'

import { create } from 'zustand'
import type { RemoteModelItem } from '@/services/ipc'

// ttsDefaultVoiceStore.ts
//
// 「每个语种的默认音色」（2026-09-23 新增）。
//
// 背景：TTS 目前只用在翻译页（朗读原文/译文）。用户希望**按语种**决定用哪个模型 + 哪个音色：
//   中文 → MiMo 的某个音色；韩文 → Supertonic 的某个 sid。
// 以前是按「远程 TTS 模型 id」记音色（ttsVoiceStore），表达不了「同一模型下不同语言不同音色」
// 这件事，也无法在语言之间切换模型。本 store 把它升级成「语言 → {模型, 音色, 指令}」。
//
// ⚠️ 为什么默认音色存在**本地**而不是网关：
//   TTS 是用户自己的听感偏好，跟账号无关；换设备重设一次可以接受，
//   而放网关要多一张表、一套接口和一次同步（收益不匹配）。参考实现也是本地存。

const STORAGE_KEY = 'voxminutes-tts-default-voice-by-lang'

/** 某语言的默认音色配置。 */
export interface LangVoiceConfig {
  /** 远程 TTS 模型 id（网关目录里的 id，如 mimo-v2.5-tts / supertonic-3） */
  model: string
  /** 音色取值（MiMo 是音色名，Supertonic 是 sid 字符串）；空 = 用模型默认 */
  voice: string
  /** 自然语言指令（仅 MiMo 支持：风格/情感/语速；Supertonic 会忽略） */
  instructions?: string
}

function loadFromStorage(): Record<string, LangVoiceConfig> {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return {}
    const parsed = JSON.parse(raw)
    if (!parsed || typeof parsed !== 'object') return {}
    // 清洗：只保留形状正确的条目（历史脏数据不能让整个页面崩）
    const out: Record<string, LangVoiceConfig> = {}
    for (const [lang, cfg] of Object.entries(parsed as Record<string, unknown>)) {
      const c = cfg as Partial<LangVoiceConfig>
      if (c && typeof c.model === 'string' && c.model) {
        out[lang] = {
          model: c.model,
          voice: typeof c.voice === 'string' ? c.voice : '',
          ...(typeof c.instructions === 'string' && c.instructions ? { instructions: c.instructions } : {}),
        }
      }
    }
    return out
  } catch {
    return {}
  }
}

function saveToStorage(data: Record<string, LangVoiceConfig>) {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
  } catch {
    // ignore
  }
}

// ── 语种表 ────────────────────────────────────────────────────────────────────
//
// 与参考实现（mig_extracted/ttsDefaultSpeakerStore）保持同一份语言清单与显示名，
// 但**引擎换成远程模型**：中文 → MiMo，其余 31 语种 → 自建 Supertonic。
// 清单顺序即界面顺序（中文最前，其余按常用度）。

/** Supertonic 3 支持的 31 个语言码（与网关 SUPERTONIC_LANGS 一致，**不含中文**）。
 *  ⚠️ 这份清单必须与 `gateway/src/upstream.ts` 的 SUPERTONIC_LANGS 和本地服务
 *  `supertonic_tts_service.mjs` 的 LANGS 三处一致；网关侧还有一次白名单校验兜底
 *  （不在其中会返回 400 而不是静默出错），所以这里多写/少写不会造成静默故障。 */
export const SUPERTONIC_LANGS = [
  'en', 'ja', 'ko', 'es', 'fr', 'de', 'pt', 'ru', 'ar', 'hi', 'vi', 'it',
  'nl', 'pl', 'tr', 'uk', 'id', 'sv', 'da', 'fi', 'el', 'hu', 'ro',
  'cs', 'sk', 'sl', 'bg', 'hr', 'lt', 'lv', 'et',
]

/** 中文系（含粤语等方言）—— 只能走 MiMo（Supertonic 不含中文）。 */
export const CHINESE_LANGS = ['zh', 'yue', 'wuu', 'cmn']

const LANG_NAMES: Record<string, string> = {
  zh: '中文', yue: '粤语', en: 'English', ja: '日本語', ko: '한국어',
  fr: 'Français', de: 'Deutsch', es: 'Español', ru: 'Русский', ar: 'العربية',
  pt: 'Português', it: 'Italiano', tr: 'Türkçe', vi: 'Tiếng Việt',
  id: 'Bahasa Indonesia', hi: 'हिन्दी', pl: 'Polski', cs: 'Čeština',
  nl: 'Nederlands', sv: 'Svenska', da: 'Dansk', fi: 'Suomi', el: 'Ελληνικά',
  hu: 'Magyar', ro: 'Română', sk: 'Slovenčina', sl: 'Slovenščina',
  bg: 'Български', hr: 'Hrvatski', lt: 'Lietuvių', lv: 'Latviešu',
  et: 'Eesti', uk: 'Українська', th: 'ไทย', ms: 'Bahasa Melayu', tl: 'Filipino',
}

export function getLangName(code: string): string {
  return LANG_NAMES[code] || code
}

/** 界面要展示的全部语种（中文系在前，其余按上表顺序）。 */
export const ALL_TTS_LANGS: string[] = [...CHINESE_LANGS.slice(0, 1), ...SUPERTONIC_LANGS]

/** 该语种是否只能用 MiMo（中文系）。 */
export function isChineseLang(lang: string): boolean {
  return CHINESE_LANGS.includes(lang) || lang.startsWith('zh')
}

/** 该语种能否用 Supertonic（非中文且在其 31 语种内）。 */
export function canUseSupertonic(lang: string): boolean {
  return !isChineseLang(lang) && SUPERTONIC_LANGS.includes(lang)
}

// ── 默认配置推导 ──────────────────────────────────────────────────────────────
//
// 未设置时不再"随机音色"（参考实现是本地多音色所以随机有意义），
// 而是给一个**确定的**默认：同一个 sid 每台机器都一样，便于复现问题。

/** Supertonic 的默认 sid：0（女声）。 */
export const SUPERTONIC_DEFAULT_SID = '0'
/** MiMo 的中文默认音色：官方预置里风格最中性的「冰糖」。 */
export const MIMO_DEFAULT_ZH_VOICE = '冰糖'

/** 某语种在未手动设置时的默认配置（依据可用模型列表）。 */
export function defaultConfigForLang(lang: string, models: RemoteModelItem[]): LangVoiceConfig {
  const has = (id: string) => models.some((m) => m.id === id)
  if (isChineseLang(lang)) {
    // 中文只能 MiMo；它没上架就留空 model（调用方会回退到网关默认）
    return has('mimo-v2.5-tts')
      ? { model: 'mimo-v2.5-tts', voice: MIMO_DEFAULT_ZH_VOICE }
      : { model: '', voice: '' }
  }
  if (canUseSupertonic(lang) && has('supertonic-3')) {
    return { model: 'supertonic-3', voice: SUPERTONIC_DEFAULT_SID }
  }
  // 兜底：Supertonic 不支持该语种（如泰语）或它没上架 → MiMo + 英文音色
  return has('mimo-v2.5-tts') ? { model: 'mimo-v2.5-tts', voice: 'Mia' } : { model: '', voice: '' }
}

interface TtsDefaultVoiceStore {
  /** 语言码 → 用户手动设置的默认音色（只存用户设过的；未设的不在这里） */
  byLang: Record<string, LangVoiceConfig>
  /** 取某语言生效的配置：用户设置优先，否则按可用模型推导默认 */
  resolve: (lang: string, models: RemoteModelItem[]) => LangVoiceConfig
  /** 用户是否手动设置过 */
  isCustom: (lang: string) => boolean
  /** 设置某语言的默认音色 */
  setForLang: (lang: string, cfg: LangVoiceConfig) => void
  /** 恢复某语言的默认（删掉用户设置） */
  clearLang: (lang: string) => void
  /** 清空全部（管理页的"全部恢复默认"） */
  clearAll: () => void
}

export const useTtsDefaultVoiceStore = create<TtsDefaultVoiceStore>((set, get) => ({
  byLang: loadFromStorage(),

  resolve: (lang, models) => {
    const custom = get().byLang[lang]
    if (custom) return custom
    return defaultConfigForLang(lang, models)
  },

  isCustom: (lang) => !!get().byLang[lang],

  setForLang: (lang, cfg) => {
    set((state) => {
      const next = { ...state.byLang, [lang]: cfg }
      saveToStorage(next)
      return { byLang: next }
    })
  },

  clearLang: (lang) => {
    set((state) => {
      const next = { ...state.byLang }
      delete next[lang]
      saveToStorage(next)
      return { byLang: next }
    })
  },

  clearAll: () => {
    saveToStorage({})
    set({ byLang: {} })
  },
}))
