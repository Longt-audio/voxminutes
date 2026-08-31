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
    setRemoteModelChoice({ [kind]: v }).catch(() => {})
  }

  return { models, value, set, loading }
}

/** 格式化模型单价为可读积分消耗文案。 */
export function formatModelPrice(m: RemoteModelItem): string {
  const price = m.price ?? 0
  if (price <= 0) return '免费'
  const unit = m.price_unit === 'second' ? '秒' : m.price_unit === 'char' ? '字符' : 'token'
  const priceStr = price >= 0.01 ? price.toFixed(2) : price.toFixed(4)
  return `${priceStr} 积分/${unit}`
}
