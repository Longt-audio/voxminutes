'use client'

import { useEffect, useState } from 'react'
import {
  listRemoteModels,
  getRemoteModelChoice,
  setRemoteModelChoice,
  type RemoteModelItem,
} from '@/services/ipc'

/** 远程模型选择（在各功能使用处选择，用户中心只读展示）。
 *  读取网关模型列表 + 当前选择，返回 [列表, 当前值, 设置函数]。 */
export function useRemoteModelChoice(kind: 'asr' | 'translate' | 'tts') {
  const [models, setModels] = useState<RemoteModelItem[]>([])
  const [value, setValue] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    Promise.all([
      listRemoteModels().catch(() => [] as RemoteModelItem[]),
      getRemoteModelChoice().catch(() => ({ asr: '', translate: '', tts: '' })),
    ]).then(([list, choice]) => {
      if (cancelled) return
      const filtered = list.filter((m) => m.kind === kind)
      setModels(filtered)
      const cur = choice[kind]
      // 当前值必须落在可用列表里，否则取第一个
      setValue(filtered.some((m) => m.id === cur) ? cur : filtered[0]?.id || '')
      setLoading(false)
    })
    return () => {
      cancelled = true
    }
  }, [kind])

  const set = (v: string) => {
    setValue(v)
    const m = models.find((mm) => mm.id === v)
    // ASR 需要把网关下发的 mode 一并透传，后端据此决定流式/非流式（不再依赖模型名后缀）
    setRemoteModelChoice(
      kind === 'asr'
        ? { asr: v, asr_mode: m?.mode }
        : kind === 'translate'
          ? { translate: v }
          : { tts: v },
    ).catch(() => {})
  }

  return { models, value, set, loading }
}

/** 格式化模型单价为可读积分消耗文案：ASR 显示「积分/小时」，LLM「积分/千token」，TTS「积分/千字符」。 */
export function formatModelPrice(m: RemoteModelItem): string {
  const price = m.price ?? 0
  if (price <= 0) return '免费'
  if (m.price_unit === 'second') return `${(price * 3600).toFixed(2)} 积分/小时`
  if (m.price_unit === 'char') return `${(price * 1000).toFixed(2)} 积分/千字符`
  return `${(price * 1000).toFixed(2)} 积分/千token`
}
