import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { Language } from '@/i18n/messages'

/** UI 语言：默认英语，用户切换后持久化（localStorage） */
interface LanguageState {
  language: Language
  setLanguage: (language: Language) => void
}

export const useLanguageStore = create<LanguageState>()(
  persist(
    (set) => ({
      language: 'en',
      setLanguage: (language) => set({ language }),
    }),
    { name: 'voxminutes-language' },
  ),
)
