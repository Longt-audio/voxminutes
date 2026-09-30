'use client'

import { create } from 'zustand'
import { getRemoteModelChoice, setRemoteModelChoice, warmRemoteStreaming } from '@/services/ipc'

/** 远程模型选择的五种用途（实时 ASR / 离线重识别 ASR / 翻译 / 会议总结 / TTS）。
 *  除 TTS 外各自是**独立持久化字段**（后端 REMOTE_ASR_MODEL / REMOTE_ASR_OFFLINE_MODEL /
 *  REMOTE_TRANSLATE_MODEL / REMOTE_SUMMARY_MODEL）。
 *
 *  为什么「会议总结」要独立于「翻译」（2026-09-23）：网关后台能给每个 LLM 标用途
 *  （both / translate / summary），而翻译与总结的合法模型集**不相交** —— 豆包机器翻译
 *  只翻译、deepseek-flash 只总结。共用一个字段时，在总结里选模型会把翻译的选择改掉
 *  （两侧的「失配自动纠正」还会互相打架）。与 asr / asr_offline 拆分同一先例。 */
export type RemoteChoiceKind = 'asr' | 'asr_offline' | 'translate' | 'summary' | 'tts'

type ChoiceValues = Record<RemoteChoiceKind, string>

const EMPTY: ChoiceValues = { asr: '', asr_offline: '', translate: '', summary: '', tts: '' }

interface RemoteModelChoiceState {
  values: ChoiceValues
  /** 已从后端成功读回一次。全局只读一次，所有 hook 实例共享同一份值。 */
  loaded: boolean
  loading: boolean
  /** 从后端读回选择（幂等；已读回或正在读时直接返回）。 */
  load: () => void
  /** 强制从后端重读（设置页保存后、或怀疑外部改动时用）。 */
  reload: () => void
  /** 只更新内存（后端写入失败时的本地回显，不落盘）。 */
  setLocal: (kind: RemoteChoiceKind, value: string) => void
  /** 更新内存 + 持久化到后端（asrMode 由调用方从网关模型目录透传）。 */
  persist: (kind: RemoteChoiceKind, value: string, asrMode?: string) => Promise<void>
}

/**
 * 远程模型选择的**唯一权威内存副本**。
 *
 * 此前 `useRemoteModelChoice` 把值放在各 hook 实例自己的 useState 里，
 * 每个组件（历史页外层 = 按钮文案、AsrModelPicker 内层 = 卡片高亮、设置页…）
 * 各存一份、只在挂载时读一次后端 → 在一个地方选了别的模型，别处永远显示
 * 挂载时的旧值（2026-09-20 用户实测：「离线识别模型选了 mimo，显示一直是 Qwen-ASR，
 * 选其他的也还是它」）。改为 zustand 全局 store 后所有实例实时同步。
 */
export const useRemoteModelChoiceStore = create<RemoteModelChoiceState>((set, get) => ({
  values: { ...EMPTY },
  loaded: false,
  loading: false,

  load: () => {
    const { loaded, loading } = get()
    if (loaded || loading) return
    set({ loading: true })
    getRemoteModelChoice()
      .catch(() => EMPTY)
      .then((choice) => {
        set({
          values: {
            asr: choice?.asr ?? '',
            asr_offline: choice?.asr_offline ?? '',
            translate: choice?.translate ?? '',
            summary: choice?.summary ?? '',
            tts: choice?.tts ?? '',
          },
          loaded: true,
          loading: false,
        })
      })
  },

  reload: () => {
    set({ loaded: false, loading: false })
    get().load()
  },

  setLocal: (kind, value) => {
    set((s) => (s.values[kind] === value ? s : { values: { ...s.values, [kind]: value } }))
  },

  persist: async (kind, value, asrMode) => {
    // 先本地生效（UI 立即响应），失败再回滚为后端当前值
    set((s) => ({ values: { ...s.values, [kind]: value } }))
    try {
      await setRemoteModelChoice(
        kind === 'asr'
          ? { asr: value, asr_mode: asrMode }
          : kind === 'asr_offline'
            ? { asr_offline: value }
            : kind === 'translate'
              ? { translate: value }
              : kind === 'summary'
                ? { summary: value }
                : { tts: value },
      )
      // 实时 ASR 模型切换后立即按新模型预热流式通道（预检缓存 key 含模型，
      // 不换 key 旧缓存不会误命中；fire-and-forget，失败不影响选择本身）。
      if (kind === 'asr') warmRemoteStreaming()
    } catch (e) {
      get().reload()
      throw e
    }
  },
}))
