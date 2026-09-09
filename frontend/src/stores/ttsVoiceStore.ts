'use client'

import { create } from 'zustand'

// 远程 TTS 默认语音（音色）偏好：按「远程 TTS 模型 id」记忆，持久化到 localStorage。
// 翻译页播放原文/译文、语音合成页生成时都会用到。

const STORAGE_KEY = 'voxminutes-tts-default-voice'

function loadFromStorage(): Record<string, string> {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw)
  } catch {
    // ignore
  }
  return {}
}

function saveToStorage(data: Record<string, string>) {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
  } catch {
    // ignore
  }
}

interface TtsVoiceStore {
  /** 每个远程 TTS 模型当前选择的默认音色（modelId → voice） */
  voices: Record<string, string>
  /** 读取某模型的默认音色（未设置返回 undefined，走供应商默认） */
  getVoice: (modelId: string) => string | undefined
  /** 设置某模型的默认音色 */
  setVoice: (modelId: string, voice: string) => void
  /** 清除某模型的默认音色（恢复供应商默认） */
  clearVoice: (modelId: string) => void
}

export const useTtsVoiceStore = create<TtsVoiceStore>((set, get) => ({
  voices: loadFromStorage(),

  getVoice: (modelId) => {
    const v = get().voices[modelId]
    return v && v.trim() ? v : undefined
  },

  setVoice: (modelId, voice) => {
    set((state) => {
      const next = { ...state.voices, [modelId]: voice }
      saveToStorage(next)
      return { voices: next }
    })
  },

  clearVoice: (modelId) => {
    set((state) => {
      const next = { ...state.voices }
      delete next[modelId]
      saveToStorage(next)
      return { voices: next }
    })
  },
}))
