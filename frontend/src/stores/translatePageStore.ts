import { create } from 'zustand'

/** 翻译页内容（切换页面后返回仍保留） */
interface TranslatePageState {
  input: string
  output: string
  /** 目标语言代码（合法取值由当前引擎决定；home 仅决定默认值，不参与过滤） */
  targetLang: string
  setInput: (v: string) => void
  setOutput: (v: string) => void
  setTargetLang: (v: string) => void
}

export const useTranslatePageStore = create<TranslatePageState>()((set) => ({
  input: '',
  output: '',
  targetLang: 'zh', // 与默认 UI 语言 en 对应的默认目标；mount 后会与后端同步
  setInput: (v) => set({ input: v }),
  setOutput: (v) => set({ output: v }),
  setTargetLang: (v) => set({ targetLang: v }),
}))
