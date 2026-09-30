import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { Language } from '@/i18n/messages'

/** 按系统语言推断默认界面语言（navigator.language 在 macOS/Windows webview 均可用）。
 *  仅在用户从未手动选过语言时生效：persist 里已有值时水合会覆盖这里的初始值，
 *  之后用户手动切换的结果持久化到 localStorage，优先于系统语言。 */
function detectSystemLanguage(): Language {
  if (typeof navigator === 'undefined') return 'en' // SSR / 静态导出环境
  const lang = (navigator.language || '').toLowerCase()
  if (lang.startsWith('zh')) return 'zh'
  if (lang.startsWith('ko')) return 'ko'
  if (lang.startsWith('ja')) return 'ja'
  return 'en'
}

/** UI 语言：默认跟随系统语言，用户切换后持久化（localStorage）并优先于系统语言 */
interface LanguageState {
  language: Language
  setLanguage: (language: Language) => void
}

export const useLanguageStore = create<LanguageState>()(
  persist(
    (set) => ({
      language: detectSystemLanguage(),
      setLanguage: (language) => set({ language }),
    }),
    { name: 'voxminutes-language' },
  ),
)
