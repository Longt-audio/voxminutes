'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { Download, Loader2, RotateCcw, Star, Volume2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { listRemoteModels, ttsSynthesize, saveTtsAudio, type RemoteModelItem } from '@/services/ipc'
import { useTtsVoiceStore } from '@/stores/ttsVoiceStore'
import { formatModelPrice } from '@/lib/remoteModelChoice'
import { cn } from '@/lib/utils'

/** 各供应商常用音色建议（datalist 提供建议，允许自由输入；留空用供应商默认音色）。
 *  mimo 列表来自上游报错实测；dashscope 提供实测可用的默认音色，可自由输入其他。 */
const VOICE_SUGGESTIONS: Record<string, string[]> = {
  openai: ['alloy', 'echo', 'fable', 'onyx', 'nova', 'shimmer'],
  dashscope: ['longanhuan_v3.6'],
  mimo: ['mimo_default', '冰糖', '茉莉', '苏打', '白桦', 'Mia', 'Chloe', 'Milo', 'Dean'],
}

function voiceSuggestions(provider: string): string[] {
  return VOICE_SUGGESTIONS[provider] ?? []
}

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

export default function TtsPage() {
  const t = useMessages()
  const [models, setModels] = useState<RemoteModelItem[]>([])
  const [loading, setLoading] = useState(true)
  // 当前 tab：某个模型 id，或 'default'（默认语音概览）
  const [tab, setTab] = useState<string>('')
  // 每个模型的音色输入与文本输入
  const [voiceInputs, setVoiceInputs] = useState<Record<string, string>>({})
  const [texts, setTexts] = useState<Record<string, string>>({})
  // 播放器状态
  const [ttsAudio, setTtsAudio] = useState<TtsAudio | null>(null)
  const [generating, setGenerating] = useState<string | null>(null) // 正在合成的模型 id
  const [savingAudio, setSavingAudio] = useState(false)
  const audioRef = useRef<HTMLAudioElement | null>(null)
  const urlRef = useRef<string | null>(null)

  const voices = useTtsVoiceStore((s) => s.voices)
  const setVoice = useTtsVoiceStore((s) => s.setVoice)
  const clearVoice = useTtsVoiceStore((s) => s.clearVoice)

  // 加载远程 TTS 模型列表；初始化每个模型的音色输入（用已存的默认音色）
  useEffect(() => {
    let cancelled = false
    listRemoteModels()
      .then((list) => {
        if (cancelled) return
        const ttsModels = list.filter((m) => m.kind === 'tts')
        setModels(ttsModels)
        const init: Record<string, string> = {}
        ttsModels.forEach((m) => {
          const v = useTtsVoiceStore.getState().getVoice(m.id)
          if (v) init[m.id] = v
        })
        setVoiceInputs(init)
        if (ttsModels.length > 0) setTab(ttsModels[0].id)
        else setTab('default')
        setLoading(false)
      })
      .catch(() => {
        if (!cancelled) {
          setModels([])
          setTab('default')
          setLoading(false)
        }
      })
    return () => {
      cancelled = true
    }
  }, [])

  // 合成完成后自动播放
  useEffect(() => {
    if (ttsAudio) {
      audioRef.current?.play().catch(() => {})
    }
  }, [ttsAudio])

  // 卸载时释放 Blob URL
  useEffect(
    () => () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current)
    },
    []
  )

  const currentModel = models.find((m) => m.id === tab)

  const handleGenerate = useCallback(
    async (model: RemoteModelItem) => {
      const text = (texts[model.id] ?? '').trim()
      if (!text || generating) return
      setGenerating(model.id)
      try {
        // 音色：输入值 > 该模型已存的默认音色 > 供应商默认（留空）
        const voice = (voiceInputs[model.id] ?? voices[model.id] ?? '').trim() || undefined
        const result = await ttsSynthesize(text, voice, model.id)
        if (urlRef.current) URL.revokeObjectURL(urlRef.current)
        const blob = base64ToBlob(result.audio_base64, result.content_type)
        const url = URL.createObjectURL(blob)
        urlRef.current = url
        setTtsAudio({
          url,
          base64: result.audio_base64,
          mime: result.content_type,
          label: `${model.owned_by} / ${model.id}`,
          fileName: `vox_tts_${model.id}_${Date.now()}`,
        })
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        toast.error(t.ttsGenerateFailed, { description: msg })
      } finally {
        setGenerating(null)
      }
    },
    [texts, voiceInputs, voices, generating, t]
  )

  const handleSetDefault = useCallback(
    (model: RemoteModelItem) => {
      const voice = (voiceInputs[model.id] ?? '').trim()
      if (!voice) return
      setVoice(model.id, voice)
      toast.success(t.ttsSetDefaultDone.replace('{voice}', voice))
    },
    [voiceInputs, setVoice, t]
  )

  const handleResetDefault = useCallback(
    (model: RemoteModelItem) => {
      clearVoice(model.id)
      setVoiceInputs((prev) => ({ ...prev, [model.id]: '' }))
      toast.success(t.ttsResetDefaultDone)
    },
    [clearVoice, t]
  )

  const handleDownload = useCallback(async () => {
    if (!ttsAudio || savingAudio) return
    setSavingAudio(true)
    try {
      const path = await saveTtsAudio(ttsAudio.base64, ttsAudio.fileName, ttsAudio.mime)
      if (path) toast.success(t.ttsSaved.replace('{path}', path))
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      toast.error(t.ttsGenerateFailed, { description: msg })
    } finally {
      setSavingAudio(false)
    }
  }, [ttsAudio, savingAudio, t])

  return (
    <div className="h-full flex flex-col bg-background px-5 pt-8 pb-5 gap-4 overflow-y-auto custom-scrollbar">
      {/* 页头 */}
      <header className="shrink-0 flex items-baseline gap-2">
        <h1 className="text-xl font-semibold">{t.ttsTitle}</h1>
        <p className="text-xs text-muted-foreground">{t.ttsSubtitle}</p>
      </header>

      {loading ? (
        <div className="text-sm text-muted-foreground py-6">{t.comLoading}</div>
      ) : models.length === 0 ? (
        <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
          {t.ttsNoModelsHint}
        </div>
      ) : (
        <>
          {/* tab 栏：每个 TTS 模型一个 tab + 默认语音 tab（下划线风格，与设置页一致） */}
          <div className="shrink-0 flex gap-4 border-b border-border/60 overflow-x-auto">
            {models.map((m) => (
              <button
                key={m.id}
                className={cn(
                  'px-1 pb-2 text-sm font-medium border-b-2 -mb-px transition-colors whitespace-nowrap',
                  tab === m.id
                    ? 'border-primary text-primary'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                )}
                onClick={() => setTab(m.id)}
              >
                {m.owned_by} / {m.id}
              </button>
            ))}
            <button
              className={cn(
                'px-1 pb-2 text-sm font-medium border-b-2 -mb-px transition-colors whitespace-nowrap',
                tab === 'default'
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
              onClick={() => setTab('default')}
            >
              {t.ttsDefaultTab}
            </button>
          </div>

          {/* 内容区 */}
          <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar">
            {tab !== 'default' && currentModel ? (
              <ModelSynthesisPanel
                model={currentModel}
                voiceInput={voiceInputs[currentModel.id] ?? ''}
                setVoiceInput={(v) => setVoiceInputs((prev) => ({ ...prev, [currentModel.id]: v }))}
                text={texts[currentModel.id] ?? ''}
                setText={(v) => setTexts((prev) => ({ ...prev, [currentModel.id]: v }))}
                generating={generating === currentModel.id}
                onGenerate={() => handleGenerate(currentModel)}
                onSetDefault={() => handleSetDefault(currentModel)}
                defaultVoice={voices[currentModel.id]}
              />
            ) : (
              <DefaultVoicePanel models={models} voices={voices} onReset={handleResetDefault} />
            )}
          </div>
        </>
      )}

      {/* 播放器：合成完成后显示，可播放 / 下载保存音频 */}
      {ttsAudio && (
        <div className="shrink-0 rounded-md border bg-card p-3 flex items-center gap-3">
          <span className="text-xs font-medium text-muted-foreground shrink-0">{ttsAudio.label}</span>
          <audio ref={audioRef} controls src={ttsAudio.url} className="flex-1 min-w-0 h-9" />
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5 shrink-0"
            onClick={handleDownload}
            disabled={savingAudio}
          >
            {savingAudio ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Download className="h-3.5 w-3.5" />
            )}
            {savingAudio ? t.ttsSaving : t.comDownload}
          </Button>
        </div>
      )}
    </div>
  )
}

/** 单个 TTS 模型的合成面板：模型信息 + 音色 + 文本 + 生成 / 设为默认 */
function ModelSynthesisPanel({
  model,
  voiceInput,
  setVoiceInput,
  text,
  setText,
  generating,
  onGenerate,
  onSetDefault,
  defaultVoice,
}: {
  model: RemoteModelItem
  voiceInput: string
  setVoiceInput: (v: string) => void
  text: string
  setText: (v: string) => void
  generating: boolean
  onGenerate: () => void
  onSetDefault: () => void
  defaultVoice?: string
}) {
  const t = useMessages()
  const suggestions = voiceSuggestions(model.owned_by)
  const datalistId = `voice-list-${model.id.replace(/[^a-zA-Z0-9]/g, '-')}`

  return (
    <div className="max-w-[720px] pt-4 pb-6 flex flex-col gap-4">
      {/* 模型信息 */}
      <div className="flex items-center gap-2">
        <span className="text-sm font-semibold">{model.id}</span>
        <span className="text-[11px] text-muted-foreground">
          {model.owned_by} · {formatModelPrice(model)}
        </span>
        {defaultVoice && (
          <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-primary/10 text-primary flex items-center gap-1">
            <Star className="h-2.5 w-2.5" />
            {defaultVoice}
          </span>
        )}
      </div>

      {/* 音色 */}
      <div className="flex items-center gap-3">
        <label className="text-xs text-muted-foreground shrink-0 w-12">{t.ttsVoice}</label>
        <input
          list={datalistId}
          className="h-8 flex-1 rounded-md border border-input bg-background px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          value={voiceInput}
          onChange={(e) => setVoiceInput(e.target.value)}
          placeholder={t.ttsVoicePlaceholder}
          disabled={generating}
        />
        {suggestions.length > 0 && (
          <datalist id={datalistId}>
            {suggestions.map((v) => (
              <option key={v} value={v} />
            ))}
          </datalist>
        )}
      </div>

      {/* 文本 */}
      <textarea
        className="min-h-[140px] resize-y rounded-md border border-input bg-transparent p-3 text-sm leading-relaxed focus:outline-none custom-scrollbar"
        placeholder={t.ttsTextPlaceholder}
        value={text}
        onChange={(e) => setText(e.target.value)}
        disabled={generating}
      />

      {/* 操作 */}
      <div className="flex items-center gap-2">
        <Button className="gap-1.5" onClick={onGenerate} disabled={generating || !text.trim()}>
          {generating ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Volume2 className="h-4 w-4" />
          )}
          {generating ? t.ttsGenerating : t.ttsGenerate}
        </Button>
        <Button
          variant="outline"
          className="gap-1.5"
          onClick={onSetDefault}
          disabled={generating || !voiceInput.trim()}
          title={t.ttsSetAsDefault}
        >
          <Star className="h-3.5 w-3.5" />
          {t.ttsSetAsDefault}
        </Button>
      </div>
    </div>
  )
}

/** 默认语音概览 tab：列出每个模型的默认音色（自定义 / 供应商默认），可恢复默认 */
function DefaultVoicePanel({
  models,
  voices,
  onReset,
}: {
  models: RemoteModelItem[]
  voices: Record<string, string>
  onReset: (model: RemoteModelItem) => void
}) {
  const t = useMessages()
  return (
    <div className="max-w-[720px] pt-4 pb-6">
      <p className="text-xs text-muted-foreground leading-relaxed mb-3">
        {t.ttsSubtitle}
      </p>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border/60 bg-muted/30 text-left">
            <th className="py-2 px-3 font-medium text-muted-foreground">{t.ttsColModel}</th>
            <th className="py-2 px-3 font-medium text-muted-foreground">{t.ttsColVoice}</th>
            <th className="py-2 px-3 font-medium text-muted-foreground"></th>
          </tr>
        </thead>
        <tbody>
          {models.map((m) => {
            const custom = voices[m.id]
            return (
              <tr key={m.id} className="border-b border-border/40">
                <td className="py-2 px-3">
                  <span className="font-medium">{m.id}</span>
                  <span className="ml-1.5 text-xs text-muted-foreground">{m.owned_by}</span>
                </td>
                <td className="py-2 px-3">
                  {custom ? (
                    <span className="text-primary">{custom}</span>
                  ) : (
                    <span className="text-muted-foreground">{t.ttsProviderDefault}</span>
                  )}
                </td>
                <td className="py-2 px-3 text-right">
                  {custom && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="gap-1 text-xs"
                      onClick={() => onReset(m)}
                    >
                      <RotateCcw className="h-3 w-3" />
                      {t.ttsResetDefault}
                    </Button>
                  )}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
