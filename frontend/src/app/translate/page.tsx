'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import Link from 'next/link'
import { ArrowLeftRight, Copy, Check, Loader2, X, Volume2, Download } from 'lucide-react'
import { toast } from 'sonner'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import { translateText, getTranslationEngine, setTranslationEngine as ipcSetTranslationEngine, getTranslationTargetLang, setTranslationTargetLang as ipcSetTranslationTargetLang, setTranslationHomeLang, onTranslateTextStream, getDownloadableModels, getRemoteEnabled, ttsSynthesize, saveTtsAudio } from '@/services/ipc'
import { useTranslatePageStore } from '@/stores/translatePageStore'
import { useLanguageStore } from '@/stores/languageStore'
import { useTtsVoiceStore } from '@/stores/ttsVoiceStore'
import { useMessages } from '@/i18n/useMessages'
import { getTranslateTargetLangs, translateTargetLangLabel, defaultTargetLang } from '@/lib/translateTargetLangs'
import { availableTranslationEngines } from '@/lib/translationEngines'
import { useRemoteModelChoice, formatModelPrice } from '@/lib/remoteModelChoice'
import type { TranslationEngine, DownloadableModelInfo } from '@/types'

/** 与后端一致的 CJK 启发式：非空白字符中 CJK 占比 > 30% 视为中文 */
function detectIsZh(text: string): boolean {
  const chars = [...text.replace(/\s/g, '')]
  if (chars.length === 0) return false
  const cjk = chars.filter((c) => {
    const u = c.codePointAt(0) ?? 0
    return (u >= 0x4e00 && u <= 0x9fff) || (u >= 0x3400 && u <= 0x4dbf)
  }).length
  return cjk * 10 > chars.length * 3
}

/** 把 base64 音频解码为 Blob URL（供 <audio> 播放） */
function base64ToBlobUrl(base64: string, mime: string): { url: string; blob: Blob } {
  const byteChars = atob(base64)
  const byteNums = new Array(byteChars.length)
  for (let i = 0; i < byteChars.length; i++) {
    byteNums[i] = byteChars.charCodeAt(i)
  }
  const blob = new Blob([new Uint8Array(byteNums)], { type: mime })
  return { url: URL.createObjectURL(blob), blob }
}

export default function TranslatePage() {
  const t = useMessages()
  // 输入/输出放在全局 store，切换页面后返回不丢失
  const input = useTranslatePageStore((s) => s.input)
  const output = useTranslatePageStore((s) => s.output)
  const targetLang = useTranslatePageStore((s) => s.targetLang)
  const setInput = useTranslatePageStore((s) => s.setInput)
  const setOutput = useTranslatePageStore((s) => s.setOutput)
  const setTargetLang = useTranslatePageStore((s) => s.setTargetLang)

  const [translating, setTranslating] = useState(false)
  const [copied, setCopied] = useState(false)
  const [modelMissing, setModelMissing] = useState(false)
  const [engine, setEngine] = useState<TranslationEngine>('opus')
  const [translationModels, setTranslationModels] = useState<DownloadableModelInfo[] | null>(null)
  const [remoteEnabled, setRemoteEnabled] = useState(false)

  // ── TTS 播放（远程语音合成，走网关 /v1/audio/speech） ──────────────────────────
  const [ttsLoading, setTtsLoading] = useState<'source' | 'target' | null>(null)
  const [savingAudio, setSavingAudio] = useState(false)
  const [ttsAudio, setTtsAudio] = useState<{
    url: string
    base64: string
    mime: string
    label: string
    fileName: string
  } | null>(null)
  const audioRef = useRef<HTMLAudioElement | null>(null)
  // 当前 Blob URL：新建前先 revoke，避免内存泄漏
  const ttsUrlRef = useRef<string | null>(null)

  useEffect(() => {
    getDownloadableModels()
      .then(setTranslationModels)
      .catch(() => setTranslationModels([]))
    getRemoteEnabled()
      .then(setRemoteEnabled)
      .catch(() => setRemoteEnabled(false))
  }, [])

  const translationEngines = translationModels === null
    ? (['opus', 'hymt2'] as const)
    : availableTranslationEngines(translationModels)
  // 远程翻译模型（engine 为 remote 时在翻译页直接选择，与后端远程模型同步）
  const remoteTranslate = useRemoteModelChoice('translate')
  // 远程 TTS 模型（语音合成使用，与后端远程模型同步）
  const remoteTts = useRemoteModelChoice('tts')
  // 请求代际：取消时 +1，迟到结果比对不一致则丢弃
  const requestIdRef = useRef(0)
  // 当前流式请求的 request_id：匹配才接受 delta，取消/结束后置空
  const streamRequestIdRef = useRef<string | null>(null)
  // 翻译前的旧输出：取消/失败时恢复
  const prevOutputRef = useRef('')

  // 同步后端翻译引擎与目标语言（内存态，不做持久化）：
  // 先按当前 UI 语言设置 home，再读 target；返回值非法时回退默认目标并写回后端
  useEffect(() => {
    let cancelled = false
    ;(async () => {
      const home = useLanguageStore.getState().language
      const engine = await getTranslationEngine().catch(() => 'opus' as TranslationEngine)
      if (cancelled) return
      setEngine(engine)
      await setTranslationHomeLang(home).catch(() => {})
      const l = await getTranslationTargetLang().catch(() => null)
      if (cancelled || l === null) return
      const target = getTranslateTargetLangs(engine).includes(l) ? l : defaultTargetLang(home)
      setTargetLang(target)
      if (target !== l) ipcSetTranslationTargetLang(target).catch(() => {})
    })()
    return () => {
      cancelled = true
    }
  }, [setTargetLang])

  // 订阅 translate-text-stream：匹配的 delta 追加到输出区（opus 引擎不发此事件）
  useEffect(() => {
    let unlisten: UnlistenFn | null = null
    let cancelled = false
    onTranslateTextStream((e) => {
      if (e.request_id !== streamRequestIdRef.current) return
      setOutput(useTranslatePageStore.getState().output + e.delta)
    }).then((fn) => {
      if (cancelled) fn()
      else unlisten = fn
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [setOutput])

  const handleEngineChange = useCallback((next: TranslationEngine) => {
    setEngine((prev) => {
      ipcSetTranslationEngine(next).catch(() => setEngine(prev))
      return next
    })
    // 切换引擎后若当前目标语言不再可用（如 hymt2 的日语切到 opus），回退默认目标
    const home = useLanguageStore.getState().language
    if (!getTranslateTargetLangs(next).includes(useTranslatePageStore.getState().targetLang)) {
      const fallback = defaultTargetLang(home)
      setTargetLang(fallback)
      ipcSetTranslationTargetLang(fallback).catch(() => {})
    }
  }, [setTargetLang])

  // 目标语言切换：同时写 store 与后端全局值，失败回滚
  const handleTargetLangChange = useCallback((lang: string) => {
    const prev = useTranslatePageStore.getState().targetLang
    setTargetLang(lang)
    ipcSetTranslationTargetLang(lang).catch(() => setTargetLang(prev))
  }, [setTargetLang])

  // 目标语言选项按引擎动态生成（全量，不排除 home；源语言==目标语言时后端返回原文）
  const targetLangOptions = useMemo(
    () =>
      getTranslateTargetLangs(engine).map((code) => ({
        code,
        name: translateTargetLangLabel(code, t),
      })),
    [engine, t]
  )

  // 自动识别输入语言（实时显示，翻译时后端同样自动定向）
  const inputIsZh = useMemo(() => detectIsZh(input), [input])
  const sourceLabel = input.trim() ? (inputIsZh ? t.trLangZh : t.trLangEn) : t.trAutoDetect
  const targetLabel = translateTargetLangLabel(targetLang, t)
  // 交换按钮只在 target 为中/英且输入检测为另一者时有意义
  const showSwap =
    !!input.trim() && ((targetLang === 'en' && inputIsZh) || (targetLang === 'zh' && !inputIsZh))

  const handleSwap = useCallback(() => {
    setInput(output)
    setOutput(input)
  }, [input, output, setInput, setOutput])

  const handleTranslate = useCallback(async () => {
    const text = input.trim()
    if (!text || translating) return
    const requestId = ++requestIdRef.current
    const streamId = crypto.randomUUID()
    streamRequestIdRef.current = streamId
    const prevOutput = useTranslatePageStore.getState().output
    prevOutputRef.current = prevOutput
    setTranslating(true)
    setModelMissing(false)
    setOutput('') // 流式期间从空开始累积 delta
    try {
      const result = await translateText(text, 'auto', targetLang, streamId)
      if (requestId === requestIdRef.current) {
        setOutput(result) // 以 invoke 返回的完整译文为准收尾
      }
    } catch (e) {
      if (requestId !== requestIdRef.current) return // 已取消，静默丢弃
      setOutput(prevOutput) // 失败时丢弃已流入的部分译文，恢复原输出
      const msg = e instanceof Error ? e.message : String(e)
      if (msg.includes('未安装') || msg.includes('缺失') || msg.includes('not installed')) {
        setModelMissing(true)
      }
      toast.error(t.trTranslateFailed, { description: msg })
    } finally {
      if (streamRequestIdRef.current === streamId) {
        streamRequestIdRef.current = null
      }
      if (requestId === requestIdRef.current) {
        setTranslating(false)
      }
    }
  }, [input, translating, targetLang, setOutput, t])

  const handleCancel = useCallback(() => {
    // 前端取消：作废当前请求，迟到的翻译结果和流式 delta 直接忽略
    requestIdRef.current++
    streamRequestIdRef.current = null
    setOutput(prevOutputRef.current) // 丢弃已流入的部分译文，恢复翻译前输出
    setTranslating(false)
  }, [setOutput])

  const handleCopy = useCallback(async () => {
    if (!output) return
    try {
      await navigator.clipboard.writeText(output)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      toast.error(t.trCopyFailed)
    }
  }, [output, t])

  // ── TTS：播放原文 / 译文 ─────────────────────────────────────────────────────
  const handleTtsPlay = useCallback(
    async (kind: 'source' | 'target') => {
      const text = kind === 'source' ? input : output
      if (!text.trim() || ttsLoading) return
      setTtsLoading(kind)
      try {
        // 用当前远程 TTS 模型的默认音色（可在「语音合成」页设置）；未设置走供应商默认
        const voice = useTtsVoiceStore.getState().getVoice(remoteTts.value)
        const result = await ttsSynthesize(text.trim(), voice, undefined)
        if (ttsUrlRef.current) URL.revokeObjectURL(ttsUrlRef.current)
        const { url } = base64ToBlobUrl(result.audio_base64, result.content_type)
        ttsUrlRef.current = url
        const label = kind === 'source' ? sourceLabel : targetLabel
        const stamp = Date.now()
        setTtsAudio({
          url,
          base64: result.audio_base64,
          mime: result.content_type,
          label,
          fileName: `vox_tts_${kind}_${stamp}`,
        })
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        toast.error(t.trTtsFailed, { description: msg })
      } finally {
        setTtsLoading(null)
      }
    },
    [input, output, ttsLoading, remoteTts.value, sourceLabel, targetLabel, t]
  )

  const handleTtsDownload = useCallback(async () => {
    if (!ttsAudio || savingAudio) return
    setSavingAudio(true)
    try {
      const path = await saveTtsAudio(ttsAudio.base64, ttsAudio.fileName, ttsAudio.mime)
      if (path) toast.success(t.trTtsSaved.replace('{path}', path))
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      toast.error(t.trTtsFailed, { description: msg })
    } finally {
      setSavingAudio(false)
    }
  }, [ttsAudio, savingAudio, t])

  // 合成完成后自动播放；卸载时释放 Blob URL
  useEffect(() => {
    if (ttsAudio) {
      audioRef.current?.play().catch(() => {})
    }
  }, [ttsAudio])
  useEffect(
    () => () => {
      if (ttsUrlRef.current) URL.revokeObjectURL(ttsUrlRef.current)
    },
    []
  )

  return (
    <div className="h-full flex flex-col bg-background px-5 pt-8 pb-5 gap-4 overflow-y-auto custom-scrollbar">
      {/* 页头：标题 + 描述（同行） */}
      <header className="shrink-0 flex items-baseline gap-2">
        <h1 className="text-xl font-semibold">{t.trTitle}</h1>
        <p className="text-xs text-muted-foreground">{t.trSubtitle}</p>
      </header>

      {/* 语言指示 + 交换 + 翻译按钮（同一行） */}
      <div className="flex items-center gap-3 shrink-0">
        <span className="text-sm font-medium min-w-[64px] text-center rounded-md bg-muted px-3 py-1.5">
          {sourceLabel}
        </span>
        {showSwap && (
          <Button variant="outline" size="sm" className="gap-1.5" onClick={handleSwap} title={t.trSwapTitle}>
            <ArrowLeftRight className="h-3.5 w-3.5" />
            {t.trSwap}
          </Button>
        )}
        <span className="text-sm font-medium min-w-[64px] text-center rounded-md bg-muted px-3 py-1.5">
          {targetLabel}
        </span>

        <select
          className="h-8 rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none"
          value={targetLang}
          onChange={(e) => handleTargetLangChange(e.target.value)}
          title={t.trTargetLang}
        >
          {targetLangOptions.map((o) => (
            <option key={o.code} value={o.code}>
              {o.name}
            </option>
          ))}
        </select>

        <select
          className="h-8 rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none"
          value={engine}
          onChange={(e) => handleEngineChange(e.target.value as TranslationEngine)}
          title={t.trEngine}
        >
          {translationEngines.includes('opus') && <option value="opus">{t.trEngineOpus}</option>}
          {translationEngines.includes('hymt2') && <option value="hymt2">{t.trEngineHymt2}</option>}
          {remoteEnabled && <option value="remote">{t.recEngineRemote}</option>}
        </select>

        {/* 远程翻译模型（engine 为 remote 时直接在此选择） */}
        {engine === 'remote' && remoteTranslate.models.length > 0 && (
          <select
            className="h-8 rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none max-w-[220px]"
            value={remoteTranslate.value}
            onChange={(e) => remoteTranslate.set(e.target.value)}
            title={t.trRemoteModel}
          >
            {remoteTranslate.models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.owned_by} / {m.id} · {formatModelPrice(m)}
              </option>
            ))}
          </select>
        )}

        {/* 远程 TTS 模型（语音合成使用，与用户中心选择同步） */}
        {remoteEnabled && remoteTts.models.length > 0 && (
          <select
            className="h-8 rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none max-w-[200px]"
            value={remoteTts.value}
            onChange={(e) => remoteTts.set(e.target.value)}
            title={`${t.trRemoteModel} (TTS)`}
          >
            {remoteTts.models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.owned_by} / {m.id}
              </option>
            ))}
          </select>
        )}

        {/* 默认语音入口：指引用户到语音合成页设置默认音色 */}
        {remoteEnabled && (
          <Link
            href="/tts"
            className="text-xs text-muted-foreground hover:text-foreground hover:underline shrink-0"
          >
            {t.trSetDefaultVoice}
          </Link>
        )}

        <div className="flex-1" />

        {translating ? (
          <>
            <Button disabled className="min-w-[96px]">
              <Loader2 className="h-4 w-4 animate-spin" /> {t.trTranslating}
            </Button>
            <Button variant="outline" onClick={handleCancel} className="gap-1">
              <X className="h-4 w-4" />
              {t.comCancel}
            </Button>
          </>
        ) : (
          <>
            <Button onClick={handleTranslate} disabled={!input.trim()} className="min-w-[96px]">
              {t.trTranslate}
            </Button>
            <span className="text-xs text-muted-foreground">{t.trShortcutHint}</span>
          </>
        )}
      </div>

      {/* 模型缺失提示 */}
      {modelMissing && (
        <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 shrink-0">
          <div className="text-sm font-medium text-amber-800">{t.trModelMissingTitle}</div>
          <div className="mt-0.5 text-sm text-amber-700">
            {t.trModelMissingPre}
            <Link href="/settings" className="text-primary underline">
              {t.trModelMissingLink}
            </Link>
            {t.trModelMissingPost}
          </div>
        </div>
      )}

      {/* 输入 / 输出 */}
      <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="flex flex-col min-h-0 rounded-md border bg-card">
          <div className="flex items-center justify-between px-3 py-2 border-b">
            <span className="text-xs font-medium text-muted-foreground">{sourceLabel}</span>
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground/70">
                {t.trCharCount.replace('{count}', String(input.length))}
              </span>
              <button
                className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 disabled:opacity-40"
                onClick={() => handleTtsPlay('source')}
                disabled={!input.trim() || ttsLoading !== null}
                title={t.trPlaySource}
              >
                {ttsLoading === 'source' ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Volume2 className="h-3 w-3" />
                )}
                {ttsLoading === 'source' ? t.trTtsLoading : t.trPlaySource}
              </button>
            </div>
          </div>
          <textarea
            className="flex-1 min-h-[160px] resize-none bg-transparent p-3 text-sm leading-relaxed focus:outline-none custom-scrollbar"
            placeholder={t.trInputPlaceholder}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                e.preventDefault()
                handleTranslate()
              }
            }}
          />
        </div>

        <div className="flex flex-col min-h-0 rounded-md border bg-card">
          <div className="flex items-center justify-between px-3 py-2 border-b">
            <span className="text-xs font-medium text-muted-foreground">{targetLabel}</span>
            <div className="flex items-center gap-3">
              <button
                className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 disabled:opacity-40"
                onClick={() => handleTtsPlay('target')}
                disabled={!output || ttsLoading !== null}
                title={t.trPlayTarget}
              >
                {ttsLoading === 'target' ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Volume2 className="h-3 w-3" />
                )}
                {ttsLoading === 'target' ? t.trTtsLoading : t.trPlayTarget}
              </button>
              <button
                className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 disabled:opacity-40"
                onClick={handleCopy}
                disabled={!output}
              >
                {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                {copied ? t.comCopied : t.comCopy}
              </button>
            </div>
          </div>
          <div className="flex-1 min-h-[160px] p-3 text-sm leading-relaxed overflow-y-auto custom-scrollbar whitespace-pre-wrap">
            {output ? (
              // 流式期间 delta 已写入 output，直接可读
              output
            ) : translating ? (
              <span className="text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                {t.trTranslating}
              </span>
            ) : (
              <span className="text-muted-foreground/50">{t.trOutputPlaceholder}</span>
            )}
          </div>
        </div>
      </div>

      {/* TTS 播放器：合成完成后显示，可播放 / 下载保存音频 */}
      {ttsAudio && (
        <div className="shrink-0 rounded-md border bg-card p-3 flex items-center gap-3">
          <span className="text-xs font-medium text-muted-foreground shrink-0">{ttsAudio.label}</span>
          <audio ref={audioRef} controls src={ttsAudio.url} className="flex-1 min-w-0 h-9" />
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5 shrink-0"
            onClick={handleTtsDownload}
            disabled={savingAudio}
          >
            {savingAudio ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Download className="h-3.5 w-3.5" />
            )}
            {savingAudio ? t.trTtsSaving : t.comDownload}
          </Button>
        </div>
      )}
    </div>
  )
}
