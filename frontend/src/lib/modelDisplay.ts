import type { Messages } from '@/i18n/messages'
import type { ModelDownloadProgress } from '@/types'

/** 字节数 → 人类可读体积（GB / MB）；0 或负数返回空串 */
export function formatSize(bytes: number): string {
  if (!bytes || bytes <= 0) return ''
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  return `${Math.round(bytes / (1024 * 1024))} MB`
}

/** 下载/导入进度事件的阶段文案（done/error/cancelled 返回空串，由 toast 反馈） */
export function stageText(p: ModelDownloadProgress, t: Messages): string {
  if (p.stage === 'downloading') return t.setStageDownloading.replace('{percent}', String(Math.floor(p.percent)))
  if (p.stage === 'extracting') return t.setStageExtracting
  if (p.stage === 'verifying') return t.setStageVerifying
  return ''
}

/** 模型 id → 类别（ASR / 翻译 / 总结），用于分组展示 */
export type ModelGroup = 'asr' | 'translate' | 'summary'

export function modelGroup(id: string): ModelGroup {
  if (id === 'sense-voice' || id === 'x-asr-480ms') return 'asr'
  if (id.startsWith('opus-mt-') || id.startsWith('hy-mt2-')) return 'translate'
  return 'summary'
}

/** 模型 id → 一句话描述（i18n）；未知模型返回 null */
export function modelDesc(id: string, t: Messages): string | null {
  switch (id) {
    case 'sense-voice':
      return t.setModelDescSenseVoice
    case 'x-asr-480ms':
      return t.setModelDescXAsr
    case 'opus-mt-zh-en':
    case 'opus-mt-en-zh':
      return t.setModelDescOpusMt
    case 'hy-mt2-1.8b-q4_k_m':
      return t.setModelDescHymt2
    case 'qwen2.5-3b-instruct-q4_k_m':
      return t.setModelDescQwen25
    case 'qwen3-4b-instruct-2507-q4_k_m':
      return t.setModelDescQwen3
    case 'gemma-3-4b-it-q4_k_m':
      return t.setModelDescGemma
    default:
      return null
  }
}

/**
 * 模型 id → 本地化显示名；未知模型回退到后端 display_name。
 * 后端注册表（model_download.rs）的 display_name 是硬编码中文，
 * 前端按 UI 语言展示，统一从这里取。
 */
export function modelDisplayName(id: string, t: Messages, fallback?: string): string {
  switch (id) {
    case 'sense-voice':
      return t.setModelNameSenseVoice
    case 'x-asr-480ms':
      return t.setModelNameXAsr
    case 'opus-mt-zh-en':
      return t.setModelNameOpusZhEn
    case 'opus-mt-en-zh':
      return t.setModelNameOpusEnZh
    case 'hy-mt2-1.8b-q4_k_m':
      return t.setModelNameHymt2
    case 'qwen2.5-3b-instruct-q4_k_m':
      return t.setModelNameQwen25
    case 'qwen3-4b-instruct-2507-q4_k_m':
      return t.setModelNameQwen3
    case 'gemma-3-4b-it-q4_k_m':
      return t.setModelNameGemma
    default:
      return fallback ?? id
  }
}
