'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import Link from 'next/link'
import { toast } from 'sonner'
import { Download, Loader2, RotateCcw, Sparkles, Star, Volume2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { listRemoteModels, ttsSynthesize, saveTtsAudio, type RemoteModelItem } from '@/services/ipc'
import { remoteModelDisplayName } from '@/lib/remoteModelChoice'
import { pickLangSegment } from '@/lib/langSegment'
import { catalogByKind } from '@/stores/remoteCatalogStore'
import { MIMO_TTS_VOICES, SUPERTONIC_FALLBACK_VOICES } from '@/lib/ttsVoices'
import {
  ALL_TTS_LANGS,
  getLangName,
  isChineseLang,
  useTtsDefaultVoiceStore,
  type LangVoiceConfig,
} from '@/stores/ttsDefaultVoiceStore'
import { cn } from '@/lib/utils'

/** 把 base64 音频解码为 Blob（供 <audio> 播放 / 下载） */
function base64ToBlob(base64: string, mime: string): Blob {
  const byteChars = atob(base64)
  const byteNums = new Array(byteChars.length)
  for (let i = 0; i < byteChars.length; i++) {
    byteNums[i] = byteChars.charCodeAt(i)
  }
  return new Blob([new Uint8Array(byteNums)], { type: mime })
}

interface TtsAudio {
  url: string
  base64: string
  mime: string
  label: string
  fileName: string
}

/** 每千字符的价格文案（网关给的是「积分/字符」）。 */
function pricePerK(m: RemoteModelItem | undefined, tpl: string, noPrice: string): string {
  if (!m || typeof m.price !== 'number' || m.price <= 0) return noPrice
  const per1000 = m.price * 1000
  // 0.001×1000 = 1 → "1"；0.00324×1000 = 3.24 → "3.24"
  const shown = Number.isInteger(per1000) ? String(per1000) : per1000.toFixed(2).replace(/0+$/, '').replace(/\.$/, '')
  return tpl.replace('{price}', shown)
}

type TabKey = 'mimo' | 'supertonic' | 'defaults'

// ── 2026-09-23 重做为三 tab ──────────────────────────────────────────────────
// 用户要求（原文）：「该页面上方显示对两个 tts 模型的简单介绍，比如支持语言等；
// 然后分两个 tab，分别是两个模型的具体功能；然后，再有一个 tab，是专门对默认音色的管理…
// 需要对每个语种设置默认音频，比如中文的默认音色是 mimo 的某种音色，韩文的默认音色是
// supertonic 的 xxx，并且需要在各自的 tts-tab 页面增加设置默认音色的按钮」。
//
// 2026-09-22 那版把它做成了「单页 + 只认 MiMo」，理由是"只有一个模型，按模型分 tab 已无意义"。
// 现在自建 Supertonic 3 上线（31 语种、0.001 积分/字符），两个模型并存 → 分 tab 重新有意义。
//
// 关键行为变化：默认音色从「按模型存」改成「**按语种**存」（见 ttsDefaultVoiceStore）。
// 翻译页据此按语种自动挑模型+音色 —— 中文走 MiMo、韩文走 Supertonic，用户不用手选模型。
export default function TtsPage() {
  const t = useMessages()
  const language = useLanguageStore((s) => s.language)
  const [models, setModels] = useState<RemoteModelItem[]>([])
  const [loading, setLoading] = useState(true)
  /** 模型列表加载失败的真实原因（空列表时展示 + 支持重试） */
  const [loadError, setLoadError] = useState<string | null>(null)
  const [tab, setTab] = useState<TabKey>('mimo')

  // ── MiMo tab：语种 + 音色 + 风格指令 ──
  const [mimoLang, setMimoLang] = useState('zh')
  const [voice, setVoice] = useState('')
  const [text, setText] = useState('')
  const [style, setStyle] = useState('')
  // 音色设计（官方 mimo-v2.5-tts-voicedesign）
  const [designDesc, setDesignDesc] = useState('')
  const [designText, setDesignText] = useState('')

  // ── Supertonic tab：语种 + 音色(sid) ──
  const [stLang, setStLang] = useState('en')
  const [stVoice, setStVoice] = useState('')
  const [stText, setStText] = useState('')

  // 播放器状态
  const [ttsAudio, setTtsAudio] = useState<TtsAudio | null>(null)
  const [generating, setGenerating] = useState<string | null>(null) // 'mimo' | 'design' | 'supertonic'
  const [savingAudio, setSavingAudio] = useState(false)
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const urlRef = useRef<string | null>(null)

  // 按语种的默认音色（用户设置 + 推导默认）
  const byLang = useTtsDefaultVoiceStore((s) => s.byLang)
  const setForLang = useTtsDefaultVoiceStore((s) => s.setForLang)
  const clearLang = useTtsDefaultVoiceStore((s) => s.clearLang)
  const clearAll = useTtsDefaultVoiceStore((s) => s.clearAll)

  const loadModels = useCallback(() => {
    setLoading(true)
    listRemoteModels()
      .then((list) => {
        setModels(catalogByKind(list, 'tts'))
        setLoadError(null)
      })
      .catch((e) => {
        setModels([])
        setLoadError(e instanceof Error ? e.message : String(e))
      })
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => {
    loadModels()
  }, [loadModels])

  const mimoModel = useMemo(() => models.find((m) => m.id === 'mimo-v2.5-tts'), [models])
  const designModel = useMemo(() => models.find((m) => m.id.includes('voicedesign')), [models])
  const superModel = useMemo(() => models.find((m) => m.id === 'supertonic-3'), [models])

  /** Supertonic 音色清单：优先用网关下发的 voices，缺失时回退到内置的 10 个 sid。
   *  （回退是必要的：客户端在网关升级前可能已缓存了不含 voices 的目录。） */
  const superVoices = useMemo(() => {
    const fromGw = (superModel?.voices ?? []).filter((v) => v && typeof v.id === 'string')
    return fromGw.length > 0 ? fromGw : SUPERTONIC_FALLBACK_VOICES
  }, [superModel])

  const resolved = useCallback(
    (lang: string): LangVoiceConfig => useTtsDefaultVoiceStore.getState().resolve(lang, models),
    [models],
  )

  // 切 tab / 切语种时，把音色恢复成「该语种的生效默认」，避免带着上一个语种的音色去合成
  useEffect(() => {
    if (tab !== 'mimo') return
    setVoice(resolved(mimoLang).voice || '')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, mimoLang, byLang])

  useEffect(() => {
    if (tab !== 'supertonic') return
    setStVoice(resolved(stLang).voice || '')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, stLang, byLang])

  // 合成完成后自动播放（被浏览器策略拒绝时明确提示，不再静默）
  useEffect(() => {
    if (ttsAudio && audioRef.current) {
      audioRef.current.play().catch(() => toast.error(t.ttsPlayFailed))
    }
  }, [ttsAudio, t])

  // 卸载时释放 Blob URL
  useEffect(
    () => () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current)
    },
    []
  )

  const playResult = useCallback((base64: string, mime: string, label: string, fileName: string) => {
    if (urlRef.current) URL.revokeObjectURL(urlRef.current)
    const url = URL.createObjectURL(base64ToBlob(base64, mime))
    urlRef.current = url
    setTtsAudio({ url, base64, mime, label, fileName })
  }, [])

  /** MiMo 预置音色合成 */
  const handleGenerateMimo = useCallback(async () => {
    if (!mimoModel || generating) return
    const content = text.trim()
    if (!content) return
    setGenerating('mimo')
    try {
      const v = voice.trim() || resolved(mimoLang).voice || undefined
      const result = await ttsSynthesize(content, v, mimoModel.id, style.trim() || undefined, mimoLang)
      playResult(
        result.audio_base64,
        result.content_type,
        `${remoteModelDisplayName(mimoModel)}${v ? ` · ${v}` : ''}`,
        `vox_tts_mimo_${Date.now()}`
      )
    } catch (e) {
      toast.error(t.ttsGenerateFailed, { description: e instanceof Error ? e.message : String(e) })
    } finally {
      setGenerating(null)
    }
  }, [mimoModel, generating, text, voice, style, mimoLang, resolved, playResult, t])

  /** Supertonic 合成（语种必传：它靠 language 决定发音） */
  const handleGenerateSuper = useCallback(async () => {
    if (!superModel || generating) return
    const content = stText.trim()
    if (!content) return
    setGenerating('supertonic')
    try {
      const v = stVoice.trim() || resolved(stLang).voice || undefined
      const result = await ttsSynthesize(content, v, superModel.id, undefined, stLang)
      playResult(
        result.audio_base64,
        result.content_type,
        `${remoteModelDisplayName(superModel)} · ${getLangName(stLang)}${v ? ` · ${v}` : ''}`,
        `vox_tts_supertonic_${Date.now()}`
      )
    } catch (e) {
      toast.error(t.ttsGenerateFailed, { description: e instanceof Error ? e.message : String(e) })
    } finally {
      setGenerating(null)
    }
  }, [superModel, generating, stText, stVoice, stLang, resolved, playResult, t])

  /** 音色设计：用一段自然语言描述生成全新音色并朗读给定文本 */
  const handleDesign = useCallback(async () => {
    if (!designModel || generating) return
    const desc = designDesc.trim()
    const content = designText.trim()
    if (!desc || !content) return
    setGenerating('design')
    try {
      const result = await ttsSynthesize(content, undefined, designModel.id, desc, mimoLang)
      playResult(result.audio_base64, result.content_type, remoteModelDisplayName(designModel), `vox_tts_design_${Date.now()}`)
    } catch (e) {
      toast.error(t.ttsGenerateFailed, { description: e instanceof Error ? e.message : String(e) })
    } finally {
      setGenerating(null)
    }
  }, [designModel, generating, designDesc, designText, mimoLang, playResult, t])

  /** 把当前 tab 选中的音色设为**该语种**的默认。 */
  const handleSetDefaultForLang = useCallback(
    (targetLang: string, cfg: LangVoiceConfig, label: string) => {
      setForLang(targetLang, cfg)
      toast.success(t.ttsSetDefaultDone.replace('{voice}', label), {
        description: t.ttsCurrentDefaultFor.replace('{lang}', getLangName(targetLang)),
      })
    },
    [setForLang, t]
  )

  const handleDownload = useCallback(async () => {
    if (!ttsAudio || savingAudio) return
    setSavingAudio(true)
    try {
      const path = await saveTtsAudio(ttsAudio.base64, ttsAudio.fileName, ttsAudio.mime)
      if (path) toast.success(t.ttsSaved.replace('{path}', path))
    } catch (e) {
      toast.error(t.ttsGenerateFailed, { description: e instanceof Error ? e.message : String(e) })
    } finally {
      setSavingAudio(false)
    }
  }, [ttsAudio, savingAudio, t])

  const highlights = [t.ttsHl1, t.ttsHl2, t.ttsHl3, t.ttsHl4, t.ttsHl5]

  /** 语种下拉的选项：中文系在前，其余按常用度。 */
  const langOptions = ALL_TTS_LANGS

  const modelById = useCallback(
    (id: string) => models.find((m) => m.id === id),
    [models]
  )

  return (
    <div className="h-full flex flex-col bg-background px-5 pt-6 pb-5 gap-4 overflow-y-auto custom-scrollbar">
      {/* 页头 */}
      <header className="shrink-0 flex items-baseline gap-2 flex-wrap">
        <h1 className="text-xl font-semibold">{t.ttsTitle}</h1>
        <p className="text-xs text-muted-foreground">{t.ttsSubtitle}</p>
      </header>

      {/* ── 顶部：两个模型的介绍卡（用户要求的第一块）────────────────────────── */}
      <section className="shrink-0 flex flex-col gap-2">
        <div className="flex items-baseline gap-2 flex-wrap">
          <span className="text-sm font-semibold">{t.ttsIntroCardTitle}</span>
          <span className="text-xs text-muted-foreground">{t.ttsIntroCardHint}</span>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {/* MiMo */}
          <div className="rounded-lg border bg-card px-4 py-3 flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <Sparkles className="h-4 w-4 text-primary shrink-0" />
              <span className="text-sm font-semibold">{t.ttsModelMimoName}</span>
              {mimoModel?.recommended ? (
                <span className="rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] text-primary">★ {t.recRecommended}</span>
              ) : null}
              {!mimoModel && <span className="text-[10px] text-amber-600">({t.ttsNoModelsHint})</span>}
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">{t.ttsModelMimoDesc}</p>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
              <span className="text-foreground/85">{t.ttsSupportedLangsValue}</span>
              <span className="text-foreground/70">{pricePerK(mimoModel, t.ttsPricePerK, t.ttsNoPrice)}</span>
            </div>
            <details className="text-xs">
              <summary className="cursor-pointer select-none text-muted-foreground hover:text-foreground">
                {t.ttsHighlightsTitle}
              </summary>
              <ul className="mt-1.5 ml-4 list-disc space-y-0.5 text-muted-foreground">
                {highlights.map((h, i) => (
                  <li key={i}>{h}</li>
                ))}
              </ul>
            </details>
          </div>

          {/* Supertonic */}
          <div className="rounded-lg border bg-card px-4 py-3 flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <Volume2 className="h-4 w-4 text-emerald-600 shrink-0" />
              <span className="text-sm font-semibold">{t.ttsModelSuperName}</span>
              {!superModel && <span className="text-[10px] text-amber-600">({t.ttsNoModelsHint})</span>}
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">{t.ttsModelSuperDesc}</p>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
              <span className="text-foreground/85">
                {(superModel?.languages ? pickLangSegment(superModel.languages, language).split('·')[0]?.trim() : '') || '31 languages'}
              </span>
              <span className="text-foreground/70">{pricePerK(superModel, t.ttsPricePerK, t.ttsNoPrice)}</span>
            </div>
            <p className="text-[11px] text-muted-foreground/80">{t.ttsSuperNoInstructions}</p>
          </div>
        </div>
      </section>

      {loading ? (
        <div className="text-sm text-muted-foreground py-6">{t.comLoading}</div>
      ) : models.length === 0 ? (
        <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800 flex flex-col gap-2">
          <span>{t.ttsNoModelsHint}</span>
          {loadError && (
            <span className="text-xs break-all text-amber-900/80">
              {t.ttsNoModelsReason}：{loadError}
            </span>
          )}
          <div className="flex gap-2 pt-0.5">
            <Button size="sm" variant="outline" className="h-7 px-3 text-xs" onClick={loadModels}>
              <RotateCcw className="mr-1 h-3 w-3" />
              {t.ttsRetry}
            </Button>
            <Button size="sm" variant="outline" className="h-7 px-3 text-xs" asChild>
              <Link href="/account">{t.ttsGoAccount}</Link>
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-3 min-h-0">
          {/* ── Tab 切换 ── */}
          <div className="flex items-center gap-1 border-b">
            {([
              ['mimo', t.ttsTabMimo],
              ['supertonic', t.ttsTabSupertonic],
              ['defaults', t.ttsTabDefaults],
            ] as Array<[TabKey, string]>).map(([k, label]) => (
              <button
                key={k}
                type="button"
                onClick={() => setTab(k)}
                className={cn(
                  '-mb-px border-b-2 px-3 py-2 text-xs transition-colors',
                  tab === k
                    ? 'border-primary font-medium text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                )}
              >
                {label}
              </button>
            ))}
          </div>

          {/* ── Tab 1：MiMo ── */}
          {tab === 'mimo' && (
            <section className="rounded-lg border bg-card p-4 flex flex-col gap-3">
              {!mimoModel ? (
                <p className="text-sm text-muted-foreground">{t.ttsNoModelsHint}</p>
              ) : (
                <>
                  <LangPicker value={mimoLang} onChange={setMimoLang} options={langOptions} label={t.ttsSpokenLangLabel} />

                  <div>
                    <p className="text-[11px] text-muted-foreground mb-1.5">{t.ttsVoiceGalleryHint}</p>
                    <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
                      {MIMO_TTS_VOICES.map((v) => {
                        const active = voice === v.id
                        const isDefaultForLang = resolved(mimoLang).model === 'mimo-v2.5-tts' && resolved(mimoLang).voice === v.id
                        return (
                          <button
                            key={v.id}
                            type="button"
                            onClick={() => setVoice(v.id)}
                            disabled={generating !== null}
                            className={cn(
                              'rounded-md border px-2.5 py-2 text-left transition-colors',
                              active ? 'border-primary bg-primary/5 ring-1 ring-primary/30' : 'border-border hover:border-primary/40 hover:bg-muted/40'
                            )}
                          >
                            <div className="flex items-center gap-1.5">
                              <span className="text-sm font-medium">{v.id === 'mimo_default' ? t.ttsProviderDefault : v.id}</span>
                              {isDefaultForLang && <Star className="h-3 w-3 text-primary shrink-0" />}
                            </div>
                            <div className="mt-0.5 text-[11px] text-muted-foreground">
                              {v.lang === 'zh' ? t.ttsVoiceLangZh : t.ttsVoiceLangEn}
                              {' · '}
                              {v.gender === 'female' ? t.ttsGenderFemale : t.ttsGenderMale}
                            </div>
                            <div className="text-[11px] text-muted-foreground/80">
                              {language === 'zh' ? v.styleZh : v.styleEn}
                            </div>
                          </button>
                        )
                      })}
                    </div>
                  </div>

                  <div className="flex items-center gap-2 -mt-1">
                    <span className="text-[11px] text-muted-foreground flex-1 truncate">
                      {t.ttsCurrentDefaultFor.replace('{lang}', getLangName(mimoLang))}
                      {' '}
                      {(() => {
                        const cfg = resolved(mimoLang)
                        return cfg.model === 'mimo-v2.5-tts' && cfg.voice ? cfg.voice : t.ttsNotSet
                      })()}
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 px-2 text-xs gap-1"
                      onClick={() =>
                        handleSetDefaultForLang(
                          mimoLang,
                          { model: 'mimo-v2.5-tts', voice: voice.trim(), ...(style.trim() ? { instructions: style.trim() } : {}) },
                          voice.trim() || t.ttsProviderDefault
                        )
                      }
                      disabled={!voice.trim() || generating !== null}
                      title={t.ttsSetAsDefault}
                    >
                      <Star className="h-3 w-3" />
                      {t.ttsSetDefaultForLang.replace('{lang}', getLangName(mimoLang))}
                    </Button>
                  </div>

                  <div className="flex flex-col gap-1">
                    <label className="text-xs text-muted-foreground">{t.ttsStyleLabel}</label>
                    <input
                      className="h-8 rounded-md border border-input bg-background px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                      value={style}
                      onChange={(e) => setStyle(e.target.value)}
                      placeholder={t.ttsStylePlaceholder}
                      disabled={generating !== null}
                    />
                    <span className="text-[11px] text-muted-foreground/80">{t.ttsStyleHint}</span>
                  </div>

                  <div className="flex flex-col gap-1">
                    <textarea
                      className="min-h-[64px] rounded-md border border-input bg-background px-2 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                      value={text}
                      onChange={(e) => setText(e.target.value)}
                      placeholder={t.ttsTextPlaceholder}
                      disabled={generating !== null}
                    />
                    <div className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 px-2 text-xs"
                        onClick={() => setText(t.ttsDemoText)}
                        disabled={generating !== null}
                      >
                        {t.ttsDemoText.slice(0, 6)}…
                      </Button>
                      <span className="flex-1" />
                      <Button size="sm" className="h-8 px-3 text-xs gap-1.5" onClick={handleGenerateMimo} disabled={!text.trim() || generating !== null}>
                        {generating === 'mimo' ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Volume2 className="h-3.5 w-3.5" />}
                        {generating === 'mimo' ? t.ttsGenerating : t.ttsGenerate}
                      </Button>
                    </div>
                  </div>

                  {/* 音色设计（仅 MiMo 有） */}
                  <div className="border-t pt-3 flex flex-col gap-2">
                    <div className="flex items-baseline gap-2">
                      <span className="text-xs font-semibold">{t.ttsDesignSection}</span>
                    </div>
                    {!designModel ? (
                      <p className="text-xs text-muted-foreground">{t.ttsDesignUnavailable}</p>
                    ) : (
                      <>
                        <p className="text-[11px] text-muted-foreground">{t.ttsDesignHint}</p>
                        <input
                          className="h-8 rounded-md border border-input bg-background px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                          value={designDesc}
                          onChange={(e) => setDesignDesc(e.target.value)}
                          placeholder={t.ttsDesignPlaceholder}
                          disabled={generating !== null}
                        />
                        <textarea
                          className="min-h-[56px] rounded-md border border-input bg-background px-2 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                          value={designText}
                          onChange={(e) => setDesignText(e.target.value)}
                          placeholder={t.ttsDesignTextPlaceholder}
                          disabled={generating !== null}
                        />
                        <div className="flex">
                          <span className="flex-1" />
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-8 px-3 text-xs gap-1.5"
                            onClick={handleDesign}
                            disabled={!designDesc.trim() || !designText.trim() || generating !== null}
                          >
                            {generating === 'design' ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Sparkles className="h-3.5 w-3.5" />}
                            {t.ttsDesignGenerate}
                          </Button>
                        </div>
                      </>
                    )}
                  </div>
                </>
              )}
            </section>
          )}

          {/* ── Tab 2：Supertonic 3 ── */}
          {tab === 'supertonic' && (
            <section className="rounded-lg border bg-card p-4 flex flex-col gap-3">
              {!superModel ? (
                <p className="text-sm text-muted-foreground">{t.ttsNoModelsHint}</p>
              ) : (
                <>
                  {/* 语种：只列 Supertonic 支持的（不含中文——它不支持，选了会 400） */}
                  <div className="flex items-center gap-3">
                    <label className="text-xs text-muted-foreground shrink-0">{t.ttsSpokenLangLabel}</label>
                    <select
                      className="h-8 rounded-md border border-input bg-background px-2 text-sm"
                      value={stLang}
                      onChange={(e) => setStLang(e.target.value)}
                      disabled={generating !== null}
                    >
                      {langOptions
                        .filter((l) => !isChineseLang(l))
                        .map((l) => (
                          <option key={l} value={l}>
                            {getLangName(l)}（{l}）
                          </option>
                        ))}
                    </select>
                    <span className="text-[11px] text-muted-foreground">{t.ttsSuperNoInstructions}</span>
                  </div>

                  <div>
                    <p className="text-[11px] text-muted-foreground mb-1.5">{t.ttsVoiceIdLabel}</p>
                    <div className="grid grid-cols-3 sm:grid-cols-5 gap-2">
                      {superVoices.map((v) => {
                        const active = stVoice === v.id
                        const isDefaultForLang = resolved(stLang).model === 'supertonic-3' && resolved(stLang).voice === v.id
                        return (
                          <button
                            key={v.id}
                            type="button"
                            onClick={() => setStVoice(v.id)}
                            disabled={generating !== null}
                            className={cn(
                              'rounded-md border px-2 py-2 text-center transition-colors',
                              active ? 'border-primary bg-primary/5 ring-1 ring-primary/30' : 'border-border hover:border-primary/40 hover:bg-muted/40'
                            )}
                          >
                            <div className="flex items-center justify-center gap-1">
                              <span className="text-sm font-medium">{v.id}</span>
                              {isDefaultForLang && <Star className="h-3 w-3 text-primary shrink-0" />}
                            </div>
                            <div className="text-[11px] text-muted-foreground">
                              {v.gender === 'male' ? t.ttsVoiceMale : v.gender === 'female' ? t.ttsVoiceFemale : ''}
                            </div>
                          </button>
                        )
                      })}
                    </div>
                  </div>

                  <div className="flex items-center gap-2 -mt-1">
                    <span className="text-[11px] text-muted-foreground flex-1 truncate">
                      {t.ttsCurrentDefaultFor.replace('{lang}', getLangName(stLang))}
                      {' '}
                      {(() => {
                        const cfg = resolved(stLang)
                        return cfg.model === 'supertonic-3' && cfg.voice ? cfg.voice : t.ttsNotSet
                      })()}
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 px-2 text-xs gap-1"
                      onClick={() =>
                        handleSetDefaultForLang(stLang, { model: 'supertonic-3', voice: stVoice.trim() }, stVoice.trim() || '-')
                      }
                      disabled={!stVoice.trim() || generating !== null}
                      title={t.ttsSetAsDefault}
                    >
                      <Star className="h-3 w-3" />
                      {t.ttsSetDefaultForLang.replace('{lang}', getLangName(stLang))}
                    </Button>
                  </div>

                  <div className="flex flex-col gap-1">
                    <textarea
                      className="min-h-[64px] rounded-md border border-input bg-background px-2 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                      value={stText}
                      onChange={(e) => setStText(e.target.value)}
                      placeholder={t.ttsTextPlaceholder}
                      disabled={generating !== null}
                    />
                    <div className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 px-2 text-xs"
                        onClick={() => setStText(t.ttsDemoText)}
                        disabled={generating !== null}
                      >
                        {t.ttsDemoText.slice(0, 6)}…
                      </Button>
                      <span className="flex-1" />
                      <Button size="sm" className="h-8 px-3 text-xs gap-1.5" onClick={handleGenerateSuper} disabled={!stText.trim() || generating !== null}>
                        {generating === 'supertonic' ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Volume2 className="h-3.5 w-3.5" />}
                        {generating === 'supertonic' ? t.ttsGenerating : t.ttsGenerate}
                      </Button>
                    </div>
                  </div>
                </>
              )}
            </section>
          )}

          {/* ── Tab 3：默认音色管理 ── */}
          {tab === 'defaults' && (
            <section className="rounded-lg border bg-card p-4 flex flex-col gap-3">
              <div className="flex items-start gap-3">
                <p className="text-[11px] text-muted-foreground flex-1 leading-relaxed">{t.ttsDefaultsTabHint}</p>
                <Button variant="outline" size="sm" className="h-7 px-2 text-xs shrink-0" onClick={() => { clearAll(); toast.success(t.ttsClearAllDone) }}>
                  <RotateCcw className="h-3 w-3 mr-1" />
                  {t.ttsClearAllDefaults}
                </Button>
              </div>
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b bg-muted/30">
                    <th className="text-left py-2 px-3 font-medium text-muted-foreground">{t.ttsDefaultsColLang}</th>
                    <th className="text-left py-2 px-3 font-medium text-muted-foreground">{t.ttsDefaultsColModel}</th>
                    <th className="text-left py-2 px-3 font-medium text-muted-foreground">{t.ttsDefaultsColVoice}</th>
                    <th className="text-left py-2 px-3 font-medium text-muted-foreground">{t.ttsDefaultsColStatus}</th>
                    <th className="text-right py-2 px-3 font-medium text-muted-foreground">{t.ttsDefaultsColActions}</th>
                  </tr>
                </thead>
                <tbody>
                  {ALL_TTS_LANGS.map((lang) => {
                    const cfg = resolved(lang)
                    const custom = !!byLang[lang]
                    const m = modelById(cfg.model)
                    return (
                      <tr key={lang} className="border-b border-border/40 hover:bg-muted/20">
                        <td className="py-1.5 px-3 font-medium">{getLangName(lang)}</td>
                        <td className="py-1.5 px-3 text-muted-foreground">
                          {m ? remoteModelDisplayName(m) : cfg.model || '—'}
                        </td>
                        <td className="py-1.5 px-3">{cfg.voice || t.ttsProviderDefault}</td>
                        <td className="py-1.5 px-3">
                          <span
                            className={cn(
                              'text-[10px] px-1.5 py-0.5 rounded-full',
                              custom ? 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400' : 'bg-muted text-muted-foreground'
                            )}
                          >
                            {custom ? t.ttsStatusCustom : t.ttsStatusDefault}
                          </span>
                        </td>
                        <td className="py-1.5 px-3 text-right">
                          {custom && (
                            <button
                              type="button"
                              className="text-[11px] text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
                              onClick={() => clearLang(lang)}
                            >
                              {t.ttsRestoreDefault}
                            </button>
                          )}
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </section>
          )}
        </div>
      )}

      {/* 播放器：合成完成后显示，可播放 / 下载保存音频 */}
      {ttsAudio && (
        <div className="shrink-0 rounded-md border bg-card p-3 flex items-center gap-3">
          <span className="text-xs font-medium text-muted-foreground shrink-0 max-w-[220px] truncate">{ttsAudio.label}</span>
          <audio ref={audioRef} controls src={ttsAudio.url} className="flex-1 min-w-0 h-9" />
          <Button variant="outline" size="sm" className="gap-1.5 shrink-0" onClick={handleDownload} disabled={savingAudio}>
            {savingAudio ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />}
            {savingAudio ? t.ttsSaving : t.comDownload}
          </Button>
        </div>
      )}
    </div>
  )
}

/** 语种选择行（两个模型 tab 共用）。 */
function LangPicker({
  value,
  onChange,
  options,
  label,
}: {
  value: string
  onChange: (v: string) => void
  options: string[]
  label: string
}) {
  return (
    <div className="flex items-center gap-3">
      <label className="text-xs text-muted-foreground shrink-0">{label}</label>
      <select
        className="h-8 rounded-md border border-input bg-background px-2 text-sm"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      >
        {options.map((l) => (
          <option key={l} value={l}>
            {getLangName(l)}（{l}）
          </option>
        ))}
      </select>
    </div>
  )
}
