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

export type SummaryGenerateMethod = 'api' | 'local' | 'remote'

/** 发起一次总结生成所需的全部参数（prompt 为已拼接转写内容的完整 prompt） */
export interface SummaryGenerateParams {
  method: SummaryGenerateMethod
  prompt: string
  apiConfig?: SummaryApiConfig | null
  localModelId?: string
  /** method === 'remote' 时走网关的远程 LLM 模型 id（kind=translate 的远程模型目录） */
  remoteModelId?: string
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
  // 事件回调里读文案走 ref：订阅 effect 的依赖里不放对象（见下方订阅处注释）
  const tRef = useRef(t)
  useEffect(() => {
    tRef.current = t
  })
  const [result, setResult] = useState('')
  const [streaming, setStreaming] = useState(false)
  /** done 事件上报的截断标记：模型输出顶到 max_tokens（finish_reason=length），
   *  正文不完整 —— 面板据此提示用户可重新生成。截断内容仍照常自动保存。 */
  const [truncated, setTruncated] = useState(false)
  /** 已收到的思维链字数（> 0 且正文还没开始时，界面显示"正在思考…"）。
   *  只用于进度提示，绝不并入 result —— 否则思考过程会被当成纪要写进去。 */
  const [thinkingChars, setThinkingChars] = useState(0)
  /** 思维链全文（2026-09-24）：面板里可折叠展示「AI 思考过程」，参考 mig 的
   *  AiSummaryResultPanel（amber 折叠块）。同样**绝不并入 result**。 */
  const [thinking, setThinking] = useState('')
  const requestIdRef = useRef<string | null>(null)
  const resultRef = useRef('')
  const thinkRef = useRef('')
  const onSavedRef = useRef(onSaved)
  useEffect(() => {
    onSavedRef.current = onSaved
  })

  // 订阅 summary-stream：按 requestId 过滤，token 追加，done 自动保存，error 提示。
  //
  // ⚠️ 依赖数组只放原始类型（recordingId/source），**不放 t**（2026-09-27 排查
  //  「Maximum update depth exceeded」后的加固）：t 今天是模块级常量 MESSAGES[lang]、
  //  引用稳定，但它是对象——一旦 i18n 实现改成每次渲染返回新对象，这里就会每渲染
  //  退订/重订阅一次，流式期间每秒数十次重注册监听器。事件回调要用的文案一律走
  //  tRef（与 useModelDownload 同一先例），行为不变、依赖闭死。
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    onSummaryStream((e) => {
      if (e.requestId !== requestIdRef.current) return
      if (e.kind === 'thinking') {
        // 思维链：只用来显示「AI 思考过程」+ 提示正在思考，**不进正文**
        thinkRef.current += e.text
        setThinkingChars(thinkRef.current.length)
        setThinking(thinkRef.current)
        return
      }
      if (e.kind === 'token') {
        resultRef.current += e.text
        setThinkingChars(0) // 正文开始了，思考态结束
        setResult(resultRef.current)
        return
      }
      requestIdRef.current = null
      setStreaming(false)
      // done 且 truncated=true：输出顶到 max_tokens 被截断（半截也照常保存，面板加警告）
      setTruncated(e.kind === 'done' && e.truncated === true)
      if (e.kind === 'error') {
        toast.error(tRef.current.sumGenFailed.replace('{error}', e.text))
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
  }, [recordingId, source])

  /** 开始一次生成；返回是否成功发起（invoke 失败会提示并复位状态） */
  const start = useCallback(
    async (params: SummaryGenerateParams) => {
      const requestId = crypto.randomUUID()
      // remote：走网关（protocol='gateway'，endpoint/apiKey 由后端从远程服务配置补全）
      const apiConfig: SummaryApiConfig | null =
        params.method === 'remote'
          ? { protocol: 'gateway', endpoint: '', apiKey: '', model: params.remoteModelId ?? '' }
          : (params.apiConfig ?? null)
      // 思考模式的思维链**算在 completion 配额里**（2026-09-24 实测：4096 被推理吃光→正文 0 字），
      // 所以总结要留足正文余量。网关侧对 summary 另有 8192 兜底，这里保持一致。
      const invokePromise =
        params.method === 'local'
          ? summaryLocalGenerate(requestId, params.prompt, {
              maxTokens: 8192,
              modelId: params.localModelId || undefined,
            })
          : summaryGenerate(requestId, apiConfig!, params.prompt, { maxTokens: 8192 })
      requestIdRef.current = requestId
      resultRef.current = ''
      thinkRef.current = ''
      setThinkingChars(0)
      setThinking('')
      setResult('')
      setTruncated(false)
      setStreaming(true)
      try {
        await invokePromise
      } catch (e) {
        if (requestIdRef.current === requestId) requestIdRef.current = null
        setStreaming(false)
        toast.error(tRef.current.sumGenFailed.replace('{error}', String(e)))
      }
    },
    []
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
    thinkRef.current = ''
    setThinkingChars(0)
    setThinking('')
    setResult('')
    setTruncated(false)
  }, [])

  /** 直接回填内容（只读模式加载已保存总结） */
  const setContent = useCallback((text: string) => {
    resultRef.current = text
    setResult(text)
  }, [])

  return { result, streaming, thinkingChars, thinking, truncated, start, stop, reset, setContent }
}
