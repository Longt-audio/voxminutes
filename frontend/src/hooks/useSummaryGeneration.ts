'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import {
  onSummaryStream,
  summaryCancel,
  summaryGenerate,
  summaryLocalGenerate,
  summarySave,
} from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import type { SummaryApiConfig } from '@/types'

export type SummaryGenerateMethod = 'api' | 'local'

/** 发起一次总结生成所需的全部参数（prompt 为已拼接转写内容的完整 prompt） */
export interface SummaryGenerateParams {
  method: SummaryGenerateMethod
  prompt: string
  apiConfig?: SummaryApiConfig | null
  localModelId?: string
}

/**
 * 总结流式生成共享逻辑：订阅 summary-stream（token 追加 / done 自动保存 / error 提示），
 * 供总结结果面板等使用。requestId 过滤保证多次生成互不串扰。
 */
export function useSummaryGeneration(
  recordingId: string,
  source: 'realtime' | 'offline',
  onSaved?: () => void
) {
  const t = useMessages()
  const [result, setResult] = useState('')
  const [streaming, setStreaming] = useState(false)
  const requestIdRef = useRef<string | null>(null)
  const resultRef = useRef('')
  const onSavedRef = useRef(onSaved)
  useEffect(() => {
    onSavedRef.current = onSaved
  })

  // 订阅 summary-stream：按 requestId 过滤，token 追加，done 自动保存，error 提示
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    onSummaryStream((e) => {
      if (e.requestId !== requestIdRef.current) return
      if (e.kind === 'token') {
        resultRef.current += e.text
        setResult(resultRef.current)
        return
      }
      requestIdRef.current = null
      setStreaming(false)
      if (e.kind === 'error') {
        toast.error(t.sumGenFailed.replace('{error}', e.text))
      } else if (resultRef.current.trim()) {
        summarySave(recordingId, source, resultRef.current)
          .then(() => onSavedRef.current?.())
          .catch(() => {})
      }
    })
      .then((u) => {
        if (disposed) u()
        else unlisten = u
      })
      .catch(() => {})
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [recordingId, source, t])

  /** 开始一次生成；返回是否成功发起（invoke 失败会提示并复位状态） */
  const start = useCallback(
    async (params: SummaryGenerateParams) => {
      const requestId = crypto.randomUUID()
      const invokePromise =
        params.method === 'api'
          ? summaryGenerate(requestId, params.apiConfig!, params.prompt, { maxTokens: 4096 })
          : summaryLocalGenerate(requestId, params.prompt, {
              maxTokens: 4096,
              modelId: params.localModelId || undefined,
            })
      requestIdRef.current = requestId
      resultRef.current = ''
      setResult('')
      setStreaming(true)
      try {
        await invokePromise
      } catch (e) {
        if (requestIdRef.current === requestId) requestIdRef.current = null
        setStreaming(false)
        toast.error(t.sumGenFailed.replace('{error}', String(e)))
      }
    },
    [t]
  )

  const stop = useCallback(() => {
    const requestId = requestIdRef.current
    requestIdRef.current = null
    setStreaming(false)
    if (requestId) summaryCancel(requestId).catch(() => {})
  }, [])

  /** 清空结果（重新生成前调用） */
  const reset = useCallback(() => {
    resultRef.current = ''
    setResult('')
  }, [])

  /** 直接回填内容（只读模式加载已保存总结） */
  const setContent = useCallback((text: string) => {
    resultRef.current = text
    setResult(text)
  }, [])

  return { result, streaming, start, stop, reset, setContent }
}
