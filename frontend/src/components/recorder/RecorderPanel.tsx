'use client'

import Link from 'next/link'
import { useEffect, useState } from 'react'
import { Mic, Square, Pause, Play, MicOff, Speaker, Languages } from 'lucide-react'
import { useAppStore } from '@/state'
import { useRecorder, useRecordingTimer, DEFAULT_ASR_MODEL } from '@/hooks/useRecorder'
import { useAudioLevel } from '@/hooks/useAudioLevel'
import {
  sherpaOnnxGetModels,
  getDefaultAudioDevices,
  apiGetTranscriptConfig,
  setMicMute as ipcSetMicMute,
  openSystemSoundSettings,
  getTranslationEnabled,
  getTranslationTargetLang,
  getTranslationEngine,
  setTranslationEnabled as ipcSetTranslationEnabled,
  setTranslationTargetLang as ipcSetTranslationTargetLang,
  setTranslationEngine as ipcSetTranslationEngine,
  setTranslationHomeLang,
} from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { AudioSpectrumBars } from './AudioSpectrumBars'
import { RecordingSetupDialog, type RecordingSetup } from './RecordingSetupDialog'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { getTranslateTargetLangs, translateTargetLangLabel, defaultTargetLang } from '@/lib/translateTargetLangs'
import type { ModelInfo, TranslationEngine } from '@/types'

function formatDuration(totalSeconds: number): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  return `${pad(h)}:${pad(m)}:${pad(s)}`
}

/** 初始化模型列表并回填已保存的模型选择 */
function useRecorderInit() {
  const setModels = useAppStore((s) => s.setModels)
  const setSelectedModel = useAppStore((s) => s.setSelectedModel)

  useEffect(() => {
    sherpaOnnxGetModels()
      .then((list) => {
        setModels(list)
        return list
      })
      .catch(() => [] as ModelInfo[])
      .then(async (list) => {
        try {
          const cfg = await apiGetTranscriptConfig()
          if (cfg?.model && list.some((m) => m.name === cfg.model)) {
            setSelectedModel(cfg.model)
            return
          }
        } catch {}
        const firstAvailable = list.find((m) => !m.hidden && !m.is_remote && m.status !== 'Missing')
        setSelectedModel(firstAvailable?.name || DEFAULT_ASR_MODEL)
      })

    getDefaultAudioDevices()
      .then((d) => useAppStore.getState().setDefaultDevices(d))
      .catch(() => {})

    // 同步后端翻译开关状态
    getTranslationEnabled()
      .then((v) => useAppStore.getState().setTranslateEnabled(v))
      .catch(() => {})
    // 先按当前 UI 语言设置翻译 home（后端据此决定默认目标语言，不重置当前 target），再读 target
    const home = useLanguageStore.getState().language
    Promise.all([
      getTranslationEngine().catch(() => null),
      setTranslationHomeLang(home).catch(() => {}),
    ])
      .then(([engine]) => {
        if (engine) useAppStore.getState().setTranslationEngine(engine)
        return getTranslationTargetLang().catch(() => null)
      })
      .then((l) => {
        if (l === null) return
        const engine = useAppStore.getState().translationEngine
        // 后端同步来的值若已不在可选项内（引擎变化），回退默认目标并写回后端
        const target = getTranslateTargetLangs(engine).includes(l) ? l : defaultTargetLang(home)
        useAppStore.getState().setTranslateTargetLang(target)
        if (target !== l) ipcSetTranslationTargetLang(target).catch(() => {})
      })
  }, [setModels, setSelectedModel])
}

/** 左栏：开始/停止按钮（开始前仅此一个）+ 录音中的暂停/静音 + 当前配置信息 */
export function RecorderControls() {
  const { isRecording, isPaused, isProcessing, startRecording, stopRecording, togglePause } = useRecorder()
  useRecordingTimer()
  useRecorderInit()
  const t = useMessages()

  const selectedModel = useAppStore((s) => s.selectedModel)
  const isMicMuted = useAppStore((s) => s.isMicMuted)
  const setMicMuted = useAppStore((s) => s.setMicMuted)
  const defaultDevices = useAppStore((s) => s.defaultDevices)
  const translateEnabled = useAppStore((s) => s.translateEnabled)
  const translateTargetLang = useAppStore((s) => s.translateTargetLang)
  const translationEngine = useAppStore((s) => s.translationEngine)
  const setTranslateEnabled = useAppStore((s) => s.setTranslateEnabled)
  const setTranslateTargetLang = useAppStore((s) => s.setTranslateTargetLang)
  const setTranslationEngine = useAppStore((s) => s.setTranslationEngine)

  const [setupOpen, setSetupOpen] = useState(false)
  const home = useLanguageStore((s) => s.language)

  const handleTranslateToggle = () => {
    const next = !translateEnabled
    setTranslateEnabled(next)
    ipcSetTranslationEnabled(next).catch(() => setTranslateEnabled(!next))
  }

  const handleTargetLangChange = (lang: string) => {
    const prev = translateTargetLang
    setTranslateTargetLang(lang)
    ipcSetTranslationTargetLang(lang).catch(() => setTranslateTargetLang(prev))
  }

  const handleEngineChange = (engine: TranslationEngine) => {
    const prev = translationEngine
    setTranslationEngine(engine)
    ipcSetTranslationEngine(engine).catch(() => setTranslationEngine(prev))
    // 切换引擎后若当前目标语言不再可用（如 hymt2 的日语切到 opus），回退默认目标
    if (!getTranslateTargetLangs(engine).includes(translateTargetLang)) {
      const fallback = defaultTargetLang(home)
      setTranslateTargetLang(fallback)
      ipcSetTranslationTargetLang(fallback).catch(() => {})
    }
  }

  // 目标语言选项按引擎动态生成（全量，不排除 home）；zh/en 沿用既有文案，其余用语言名
  const targetLangOptions = getTranslateTargetLangs(translationEngine).map((code) => ({
    code,
    name:
      code === 'en'
        ? t.recTranslateToEn
        : code === 'zh'
          ? t.recTranslateToZh
          : translateTargetLangLabel(code, t),
  }))

  const handleConfirmSetup = (setup: RecordingSetup) => {
    // 只更新 store：set_mic_mute 需要活动录音（开始前调用会失败），
    // 录音开始后由 useRecorder 的 onRecordingStarted 把该状态下发到音频管线
    setMicMuted(setup.micMuted)
    startRecording({
      modelName: setup.modelName,
      micDeviceName: setup.micDeviceName,
      systemDeviceName: setup.systemDeviceName,
      language: setup.language,
    })
  }

  const handleToggleMicMute = async () => {
    const next = !isMicMuted
    setMicMuted(next)
    try {
      await ipcSetMicMute(next)
    } catch {
      setMicMuted(!next)
    }
  }

  return (
    <div className="flex flex-col gap-3 shrink-0">
      {/* 主按钮 */}
      {isRecording ? (
        <Button variant="destructive" size="sm" className="gap-2 w-[120px] font-medium px-3" onClick={stopRecording} disabled={isProcessing}>
          <Square className="h-4 w-4 fill-current" />
          {t.recStop}
        </Button>
      ) : (
        <Button size="sm" className="gap-2 w-[120px] font-medium px-3" onClick={() => setSetupOpen(true)} disabled={isProcessing}>
          <Mic className="h-4 w-4" />
          {isProcessing ? t.recPreparing : t.recStart}
        </Button>
      )}

      {/* 录音中的附加控制与当前配置 */}
      {isRecording && (
        <>
          <Button variant="outline" size="sm" className="gap-2 w-[120px] font-medium px-3" onClick={togglePause}>
            {isPaused ? <Play className="h-4 w-4" /> : <Pause className="h-4 w-4" />}
            {isPaused ? t.recResume : t.recPause}
          </Button>
          <Button
            variant={isMicMuted ? 'destructive' : 'outline'}
            size="sm"
            className="gap-2 w-[120px] font-medium px-3"
            onClick={handleToggleMicMute}
          >
            {isMicMuted ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
            {isMicMuted ? t.recMuted : t.recMute}
          </Button>
          <Button
            variant={translateEnabled ? 'default' : 'outline'}
            size="sm"
            className="gap-2 w-[120px] font-medium px-3"
            onClick={handleTranslateToggle}
            title={t.recTranslateTitle}
          >
            <Languages className="h-4 w-4" />
            {translateEnabled ? t.recTranslating : t.recTranslate}
          </Button>
          {translateEnabled && (
            <>
              <select
                className="h-8 w-[120px] rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none"
                value={translateTargetLang}
                onChange={(e) => handleTargetLangChange(e.target.value)}
                title={t.recTargetLang}
              >
                {targetLangOptions.map((o) => (
                  <option key={o.code} value={o.code}>
                    {o.name}
                  </option>
                ))}
              </select>
              <select
                className="h-8 w-[120px] rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none"
                value={translationEngine}
                onChange={(e) => handleEngineChange(e.target.value as TranslationEngine)}
                title={t.recTranslateEngine}
              >
                <option value="opus">{t.recEngineOpus}</option>
                <option value="hymt2">{t.recEngineHymt2}</option>
              </select>
            </>
          )}

          <div className="w-[168px] rounded-md bg-muted/50 p-3 space-y-1.5 text-[11px] text-muted-foreground mt-1">
            <p className="truncate" title={selectedModel}>
              <span className="font-medium text-foreground">{t.recLabelAsr}</span>
              {selectedModel === 'x-asr-480ms' ? t.recModelXAsr : selectedModel === 'sense-voice' ? t.recModelSenseVoice : selectedModel || t.recNoModel}
            </p>
            {(selectedModel === 'x-asr-480ms' || selectedModel === 'sense-voice') && (
              <p className="truncate" title={selectedModel === 'x-asr-480ms' ? t.recLangsXAsr : t.recLangsSenseVoice}>
                <span className="font-medium text-foreground">{t.recLabelLangs}</span>
                {selectedModel === 'x-asr-480ms' ? t.recLangsXAsr : t.recLangsSenseVoice}
              </p>
            )}
            <p className="truncate" title={defaultDevices.microphone || t.recNoDevice}>
              <span className="font-medium text-foreground">{t.recLabelMic}</span>
              {defaultDevices.microphone || t.recNoDevice}
            </p>
            <p className="truncate" title={defaultDevices.speaker || t.recNoDevice}>
              <span className="font-medium text-foreground">{t.recLabelSpeaker}</span>
              {defaultDevices.speaker || t.recNoDevice}
            </p>
          </div>

          {/* 打开 Windows 音频设备设置页（仅录音中显示） */}
          <Button
            variant="outline"
            size="sm"
            className="gap-2 w-[120px] font-medium px-3"
            title={t.recOpenSoundTitle}
            onClick={() => openSystemSoundSettings().catch(() => {})}
          >
            <Speaker className="h-4 w-4" />
            {t.recAudioDevices}
          </Button>
        </>
      )}

      <RecordingSetupDialog open={setupOpen} onOpenChange={setSetupOpen} onConfirm={handleConfirmSetup} />
    </div>
  )
}

/** 单路迷你电平条（原始 RMS，开方缩放便于观察） */
function LevelMeter({ label, value, title }: { label: string; value: number; title: string }) {
  const h = Math.min(1, Math.sqrt(Math.max(0, value)) * 1.5)
  return (
    <div className="flex items-center gap-1" title={title}>
      <span className="text-[10px] text-muted-foreground">{label}</span>
      <div className="h-5 w-1.5 rounded-sm bg-muted overflow-hidden flex flex-col-reverse">
        <div className="w-full bg-primary transition-[height] duration-75" style={{ height: `${h * 100}%` }} />
      </div>
    </div>
  )
}

/** 右栏上方信息行：空闲时显示录制来源提示；录音时显示计时 + 双路电平 + 真实频谱 */
export function RecorderInfo() {
  const isRecording = useAppStore((s) => s.isRecording)
  const isPaused = useAppStore((s) => s.isPaused)
  const recordingDuration = useAppStore((s) => s.recordingDuration)
  const audioLevels = useAppStore((s) => s.audioLevels)
  const models = useAppStore((s) => s.models)
  const t = useMessages()

  useAudioLevel()

  const anyModelInstalled = models.some((m) => !m.hidden && !m.is_remote && m.status !== 'Missing')

  if (!isRecording) {
    // 空闲时在开始按钮旁提示录制来源（开始录音后此区域被计时器替换，提示自然消失）；
    // 未安装模型时同时显示警告
    return (
      <div className="flex flex-col gap-2">
        <p className="flex h-8 items-center text-xs text-muted-foreground">
          {t.recSourceHint}
        </p>
        {!anyModelInstalled && (
          <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3">
            <div className="text-sm font-medium text-amber-800">{t.recNoModelTitle}</div>
            <div className="mt-0.5 text-sm text-amber-700">
              {t.recNoModelPre}{' '}
              <Link href="/settings" className="text-primary underline">
                {t.recNoModelLink}
              </Link>{' '}
              {t.recNoModelPost}
            </div>
          </div>
        )}
      </div>
    )
  }

  return (
    <div className="flex gap-3 items-center h-8">
      <span className="sh-rec-dot text-primary shrink-0" />
      <span className="font-mono text-2xl font-semibold tracking-wider leading-none tabular-nums">
        {formatDuration(recordingDuration)}
      </span>
      {isPaused && <Badge variant="warning">{t.recPaused}</Badge>}
      <div className="flex items-center gap-2 shrink-0">
        <LevelMeter label={t.recMicShort} value={audioLevels.mic} title={t.recMicLevelTitle} />
        <LevelMeter label={t.recSysShort} value={audioLevels.system} title={t.recSysLevelTitle} />
      </div>
      <div className="flex-1 min-w-0 h-full rounded-md border bg-muted/40 px-1.5 py-0.5">
        <AudioSpectrumBars />
      </div>
    </div>
  )
}
