import type { DownloadableModelInfo } from '@/types'
import { summaryGetConfig } from '@/services/ipc'

/** 根据已下载模型，返回可用的翻译引擎（未下载的引擎不显示）。 */
export function availableTranslationEngines(
  models: DownloadableModelInfo[],
): ('opus' | 'hymt2')[] {
  const opusInstalled = ['opus-mt-zh-en', 'opus-mt-en-zh'].every(
    (id) => models.find((m) => m.id === id)?.installed,
  )
  const hymt2Installed = models.some((m) => m.id.startsWith('hy-mt2-') && m.installed)
  const engines: ('opus' | 'hymt2')[] = []
  if (opusInstalled) engines.push('opus')
  if (hymt2Installed) engines.push('hymt2')
  return engines
}

/** 若当前引擎不可用，回退到第一个可用引擎；都没有则返回 null。 */
export function fallbackTranslationEngine(
  current: string,
  models: DownloadableModelInfo[],
): 'opus' | 'hymt2' | null {
  const available = availableTranslationEngines(models)
  if (available.length === 0) return null
  if (available.includes(current as 'opus' | 'hymt2')) return current as 'opus' | 'hymt2'
  return available[0]
}

/**
 * custom-api 引擎是否可用：summary.api_config 已配置 openai/anthropic 协议且
 * endpoint 非空（与 Rust 侧 current_engine 的回落规则一致）。
 * 引擎下拉据此决定是否显示 custom-api 选项。
 */
export async function fetchCustomApiConfigured(): Promise<boolean> {
  try {
    const config = await summaryGetConfig()
    return (
      !!config &&
      !!config.endpoint.trim() &&
      (config.protocol === 'openai' || config.protocol === 'anthropic')
    )
  } catch {
    return false
  }
}
