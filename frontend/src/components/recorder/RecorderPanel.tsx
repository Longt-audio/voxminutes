'use client'

import Link from 'next/link'
import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { toast } from 'sonner'
import { Mic, Square, Pause, Play, MicOff, Speaker, Languages, Captions, Info } from 'lucide-react'
import { useAppStore } from '@/state'
import { useRecorder, REMOTE_ASR_PLACEHOLDER } from '@/hooks/useRecorder'
import { useAudioLevel } from '@/hooks/useAudioLevel'
import {
  sherpaOnnxGetModels,
  onModelDownloadProgress,
  getDownloadableModels,
  getDefaultAudioDevices,
  apiGetTranscriptConfig,
  apiSaveTranscriptConfig,
  switchAsrModel,
  setMicMute as ipcSetMicMute,
  openSystemSoundSettings,
  getTranslationEnabled,
  getTranslationTargetLang,
  getTranslationEngine,
  setTranslationEnabled as ipcSetTranslationEnabled,
  setTranslationTargetLang as ipcSetTranslationTargetLang,
  setTranslationEngine as ipcSetTranslationEngine,
  setTranslationHomeLang,
  toggleSubtitleWindow,
  getRemoteEnabled,
} from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { AudioSpectrumBars } from './AudioSpectrumBars'
import { RecordingSetupDialog, type RecordingSetup } from './RecordingSetupDialog'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { getTranslateTargetLangs, translateTargetLangLabel, defaultTargetLang } from '@/lib/translateTargetLangs'
import { availableTranslationEngines, fetchCustomApiConfigured } from '@/lib/translationEngines'
import { useRemoteModelChoice, remoteModelDisplayName, formatModelPrice } from '@/lib/remoteModelChoice'
import { pickLangSegment } from '@/lib/langSegment'
import { cn } from '@/lib/utils'
import type { ModelInfo, TranslationEngine, DownloadableModelInfo } from '@/types'

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
          if (cfg?.model) {
            // 远程占位名（qwen3-asr-remote*）不在本地模型列表里，需单独识别——
            // 否则会被下面的 firstAvailable 兜底成 X-ASR，导致信息卡显示与实际引擎不符
            const isRemoteCfg =
              cfg.provider === 'remote-qwen3-asr' || cfg.model.startsWith('qwen3-asr-remote')
            if (isRemoteCfg || list.some((m) => m.name === cfg.model)) {
              setSelectedModel(cfg.model)
              return
            }
          }
        } catch {}
        // ── 默认选中谁（用户 2026-09-30 明确的四档规则）────────────────────
        //   1. 上次选过且仍然可用        → 用它（上面的 cfg 分支已 return）
        //   2. 远程服务已开启            → 选推荐的远程流式模型
        //   3. 否则                      → 第一个【已安装】的本地模型
        //   4. 都没有                    → 不选（此前会回退到 DEFAULT_ASR_MODEL，
        //      而那是个**没下载**的模型名，新机器上等于默认选了个用不了的模型）
        const remoteOn = await getRemoteEnabled().catch(() => false)
        if (remoteOn) {
          setSelectedModel(REMOTE_ASR_PLACEHOLDER)
          return
        }
        const firstAvailable = list.find((m) => !m.hidden && !m.is_remote && m.status !== 'Missing')
        setSelectedModel(firstAvailable?.name ?? '')
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

  // 模型下载完成后刷新模型列表（修复：引导下载完模型仍显示「未下载」/ 不自动选中）
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    onModelDownloadProgress((p) => {
      if (p.stage === 'done') {
        sherpaOnnxGetModels()
          .then((list) => {
            setModels(list)
            // 若当前未选中任何可用模型，自动选中第一个已下载的
            // （远程占位名不在本地列表里但同样是有效选择，不能因此被改回本地模型）
            const cur = useAppStore.getState().selectedModel
            const isRemoteSel = cur === 'remote' || cur.startsWith('qwen3-asr-remote')
            const stillAvailable = isRemoteSel || list.some((m) => m.name === cur && m.status !== 'Missing')
            if (!stillAvailable) {
              const first = list.find((m) => !m.hidden && !m.is_remote && m.status !== 'Missing')
              if (first) setSelectedModel(first.name)
            }
          })
          .catch(() => {})
      }
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [setModels, setSelectedModel])
}

/** 左栏：开始/停止按钮（开始前仅此一个）+ 录音中的暂停/静音 + 当前配置信息 */
export function RecorderControls() {
  const { isRecording, isPaused, isProcessing, startRecording, stopRecording, togglePause } = useRecorder()
  useRecorderInit()
  const t = useMessages()

  const selectedModel = useAppStore((s) => s.selectedModel)
  const localModels = useAppStore((s) => s.models)
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
  const [subtitleVisible, setSubtitleVisible] = useState(false)
  const [infoOpen, setInfoOpen] = useState(false)
  // 信息卡 fixed 定位锚点（打开时按按钮位置计算一次）
  const infoBtnRef = useRef<HTMLButtonElement | null>(null)
  const [infoPos, setInfoPos] = useState<{ left: number; top: number } | null>(null)
  const [translationModels, setTranslationModels] = useState<DownloadableModelInfo[] | null>(null)
  const [remoteEnabled, setRemoteEnabled] = useState(false)
  const [customApiConfigured, setCustomApiConfigured] = useState(false)
  const home = useLanguageStore((s) => s.language)

  // 当前远程语音识别模型选择（录音信息区显示用）
  const remoteAsr = useRemoteModelChoice('asr')
  // 录音中切换翻译模型：远程部分列出全部具体翻译模型（排除推理模型，与录音前设置弹窗同一口径）
  const remoteTranslate = useRemoteModelChoice('translate', { excludeReasoning: true, usage: 'translate' })

  // ── 录音中热切换流式 ASR 模型（流式↔流式：X-ASR / 远程流式）──
  const [asrSwitching, setAsrSwitching] = useState(false)
  const isRemoteAsrSelected = selectedModel === 'remote' || selectedModel.startsWith('qwen3-asr-remote')
  const currentAsrIsStreaming = isRemoteAsrSelected || selectedModel.startsWith('x-asr-')
  // 下拉选项：本地仅流式 X-ASR（SenseVoice 是 VAD 伪实时，不支持录音中切换）+ 远程流式
  const localStreamingModels = localModels.filter(
    (m) => m.name.startsWith('x-asr-') && m.status !== 'Missing'
  )
  const asrSwitchValue = isRemoteAsrSelected ? `remote:${remoteAsr.value}` : selectedModel

  const handleAsrSwitch = async (v: string) => {
    if (asrSwitching) return
    const prevModel = selectedModel
    const prevRemoteId = remoteAsr.value
    setAsrSwitching(true)
    try {
      // 先持久化新选择（后端切换命令读的就是这份配置）。
      // remoteAsr.set 内部已带 asr_mode 持久化（远端模型选择走全局 store），
      // 这里不再重复调 IPC，避免两份写入互相覆盖。
      // 但真实远程模型 id 仍随 apiSaveTranscriptConfig/switchAsrModel 显式传给后端
      // （2026-09-28 竞态修复：set 的持久化是异步的，切换命令不能赌它已落盘）。
      if (v.startsWith('remote:')) {
        const id = v.slice('remote:'.length)
        remoteAsr.set(id)
        await apiSaveTranscriptConfig('remote-qwen3-asr', 'qwen3-asr-remote', null, id)
      } else {
        await apiSaveTranscriptConfig('x-asr', v, null)
      }
      // 再触发后端热切换（录音管线不动，只重启转写子系统）
      await switchAsrModel(v.startsWith('remote:') ? v.slice('remote:'.length) : null)
      useAppStore.getState().setSelectedModel(v.startsWith('remote:') ? 'qwen3-asr-remote' : v)
      toast.success(t.recAsrSwitched)
    } catch (e) {
      // 切换失败：回滚持久化配置与界面选择，仍在跑的旧引擎不受影响
      try {
        if (prevModel === 'remote' || prevModel.startsWith('qwen3-asr-remote')) {
          if (prevRemoteId) remoteAsr.set(prevRemoteId)
          await apiSaveTranscriptConfig('remote-qwen3-asr', 'qwen3-asr-remote', null, prevRemoteId || null)
        } else {
          await apiSaveTranscriptConfig(prevModel.startsWith('x-asr-') ? 'x-asr' : 'sherpaonnx', prevModel, null)
        }
      } catch {}
      useAppStore.getState().setSelectedModel(prevModel)
      toast.error(t.recAsrSwitchFailed.replace('{error}', e instanceof Error ? e.message : String(e)))
    } finally {
      setAsrSwitching(false)
    }
  }

  useEffect(() => {
    getDownloadableModels()
      .then(setTranslationModels)
      .catch(() => setTranslationModels([]))
    getRemoteEnabled()
      .then(setRemoteEnabled)
      .catch(() => setRemoteEnabled(false))
  }, [])

  // 自定义 API 引擎是否在引擎下拉中显示（开启翻译时刷新一次，设置页改动后可见）
  useEffect(() => {
    fetchCustomApiConfigured()
      .then(setCustomApiConfigured)
      .catch(() => setCustomApiConfigured(false))
  }, [translateEnabled])

  // 未加载完成时先显示全部引擎，加载后按已下载过滤
  const translationEngines = translationModels === null
    ? (['opus', 'hymt2'] as const)
    : availableTranslationEngines(translationModels)

  const handleSubtitleToggle = async () => {
    const next = !subtitleVisible
    setSubtitleVisible(next)
    try {
      setSubtitleVisible(await toggleSubtitleWindow())
    } catch {
      setSubtitleVisible(!next)
    }
  }

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

  // 翻译模型下拉：本地引擎（opus/hymt2）与自定义 API 原样；远程展开为具体模型（remote:<id>）。
  // 选远程项时先持久化远程子模型选择（全局 store，与其他使用处同步），再切引擎到 remote。
  const handleTranslateModelSelect = (v: string) => {
    if (v.startsWith('remote:')) {
      remoteTranslate.set(v.slice('remote:'.length))
      if (translationEngine !== 'remote') handleEngineChange('remote')
    } else {
      handleEngineChange(v as TranslationEngine)
    }
  }
  const translateModelValue =
    translationEngine === 'remote' ? `remote:${remoteTranslate.value}` : translationEngine
  const translateModelOptionValues = [
    ...(translationEngines.includes('opus') ? ['opus'] : []),
    ...(translationEngines.includes('hymt2') ? ['hymt2'] : []),
    ...(remoteEnabled ? remoteTranslate.models.map((m) => `remote:${m.id}`) : []),
    ...(customApiConfigured ? ['custom-api'] : []),
  ]

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
    // 记住这次选的识别模型 —— 此前只在 useRecorder.startRecording 里落库，
    // 结果是「在确认弹窗里换了模型但没真按开始录音」= 选择丢失（用户 2026-09-30 反馈）。
    // 这里改成确认时就写，做到「没开始录音也记住」。
    if (setup.modelName) {
      const isRemote = setup.modelName.startsWith(REMOTE_ASR_PLACEHOLDER)
      apiSaveTranscriptConfig(
        isRemote ? 'remote-qwen3-asr' : setup.modelName.startsWith('x-asr-') ? 'x-asr' : 'sherpaonnx',
        setup.modelName,
        null,
        isRemote ? remoteAsr.value || null : null
      ).catch(() => {})
    }
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

  // 切换录音状态时收起信息展示卡
  useEffect(() => {
    setInfoOpen(false)
  }, [isRecording])

  // 打开信息卡时按按钮当前位置计算 fixed 坐标（贴按钮下方，右对齐到按钮左缘）
  useEffect(() => {
    if (!infoOpen) {
      setInfoPos(null)
      return
    }
    const rect = infoBtnRef.current?.getBoundingClientRect()
    if (rect) setInfoPos({ left: rect.left, top: rect.bottom + 4 })
  }, [infoOpen])

  // 信息展示卡数据（selectedModel 在 startRecording 时已回写为实际使用的模型）
  const isRemoteModel = selectedModel === 'remote' || selectedModel.startsWith('qwen3-asr-remote')
  const selectedRemoteAsr = remoteAsr.models.find((mm) => mm.id === remoteAsr.value)
  const infoModelDisplay = isRemoteModel
    ? selectedRemoteAsr
      ? remoteModelDisplayName(selectedRemoteAsr)
      : t.recRemoteModel
    : selectedModel === 'x-asr-480ms'
      ? t.recModelXAsr + t.mdLocalSuffix
      : selectedModel === 'sense-voice'
        ? t.recModelSenseVoice + t.mdLocalSuffix
        : selectedModel || t.recNoModel
  const infoModeLabel = isRemoteModel
    ? selectedRemoteAsr?.mode === 'streaming'
      ? t.recBadgeStreaming
      : t.recBadgeBatch
    : selectedModel.startsWith('x-asr-')
      ? t.recBadgeStreaming
      : t.recBadgeBatch
  // 支持语言：远程取网关目录下发的 languages（四段式按界面语言取段）；本地用既有文案
  const infoModelLangs = isRemoteModel
    ? pickLangSegment(selectedRemoteAsr?.languages?.trim() || '', home)
    : selectedModel.startsWith('x-asr-')
      ? t.recLangsXAsr
      : selectedModel === 'sense-voice'
        ? t.recLangsSenseVoice
        : ''
  // 模型简介：远程用网关显示名+计费说明；本地用既有介绍文案
  const infoModelDesc = isRemoteModel
    ? selectedRemoteAsr
      ? `${t.recBadgeRemote} · ${formatModelPrice(selectedRemoteAsr, t)}`
      : ''
    : selectedModel.startsWith('x-asr-')
      ? t.recXAsrDesc
      : selectedModel === 'sense-voice'
        ? t.recSenseVoiceDesc
        : ''
  const infoEngineLabel =
    translationEngine === 'opus'
      ? t.recEngineOpus
      : translationEngine === 'hymt2'
        ? t.recEngineHymt2
        : translationEngine === 'remote'
          ? t.recEngineRemote
          : t.recEngineCustomApi
  const infoTargetLang =
    targetLangOptions.find((o) => o.code === translateTargetLang)?.name || translateTargetLang
  const infoRemoteStatus = !remoteEnabled
    ? t.recInfoRemoteDisabled
    : remoteAsr.models.length > 0
      ? t.recInfoRemoteOnline
      : t.recInfoRemoteOffline

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

          {/* 录音中热切换识别模型（仅流式模型可切；非流式录音中禁用并提示） */}
          <div className="flex flex-col gap-1">
            <label className="text-xs text-muted-foreground px-0.5">{t.recAsrSwitchLabel}</label>
            <select
              className="h-8 w-[120px] rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none disabled:opacity-60"
              value={asrSwitchValue}
              disabled={asrSwitching || !currentAsrIsStreaming}
              title={currentAsrIsStreaming ? undefined : t.recAsrSwitchStreamingOnly}
              onChange={(e) => {
                const v = e.target.value
                if (v && v !== asrSwitchValue) void handleAsrSwitch(v)
              }}
            >
              {localStreamingModels.map((m) => (
                <option key={m.name} value={m.name}>
                  {m.name.startsWith('x-asr-') ? 'X-ASR' : m.name}
                  {t.mdLocalSuffix}
                </option>
              ))}
              {remoteEnabled &&
                remoteAsr.models.map((m) => (
                  <option key={m.id} value={`remote:${m.id}`}>
                    {remoteModelDisplayName(m)}
                  </option>
                ))}
              {/* 兜底：当前值不在选项里（如远程目录未加载完）时显示占位，避免 select 空白 */}
              {![...localStreamingModels.map((m) => m.name), ...remoteAsr.models.map((m) => `remote:${m.id}`)].includes(asrSwitchValue) && (
                <option value={asrSwitchValue}>{asrSwitchValue}</option>
              )}
            </select>
            {asrSwitching && <span className="text-xs text-muted-foreground px-0.5">{t.recAsrSwitching}</span>}
          </div>
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
          <Button
            variant={subtitleVisible ? 'default' : 'outline'}
            size="sm"
            className="gap-2 w-[120px] font-medium px-3"
            onClick={handleSubtitleToggle}
            title={t.recSubtitleTitle}
          >
            <Captions className="h-4 w-4" />
            {t.recSubtitle}
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
              <div className="flex flex-col gap-1">
                <label className="text-xs text-muted-foreground px-0.5">{t.recTranslateModelLabel}</label>
                <select
                  className="h-8 w-[120px] rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none"
                  value={translateModelValue}
                  onChange={(e) => handleTranslateModelSelect(e.target.value)}
                  title={t.recTranslateModelLabel}
                >
                  {translationEngines.includes('opus') && <option value="opus">{t.recEngineOpus}</option>}
                  {translationEngines.includes('hymt2') && <option value="hymt2">{t.recEngineHymt2}</option>}
                  {/* 远程翻译模型逐个列出（display_name），不再是笼统的一个「远程模型」 */}
                  {remoteEnabled && remoteTranslate.models.length > 0 && (
                    <optgroup label={t.recAsrGroupRemote}>
                      {remoteTranslate.models.map((m) => (
                        <option key={m.id} value={`remote:${m.id}`}>
                          {remoteModelDisplayName(m)}
                        </option>
                      ))}
                    </optgroup>
                  )}
                  {customApiConfigured && <option value="custom-api">{t.recEngineCustomApi}</option>}
                  {/* 兜底：当前值不在选项里（如远程目录未加载完）时显示占位，避免 select 空白 */}
                  {!translateModelOptionValues.includes(translateModelValue) && (
                    <option value={translateModelValue}>
                      {translationEngine === 'remote' ? t.recEngineRemote : translateModelValue}
                    </option>
                  )}
                </select>
              </div>
            </>
          )}

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

      {/* 信息展示：录音开始后才出现，固定排在所有控制按钮最下面；悬浮卡显示当前会话详情。
          用 Portal 挂到 body（fixed 定位）——左栏容器是 overflow-y-auto，
          旧 absolute 方案超出栏宽的部分会被整体裁剪（2026-09-17 用户反馈「展示区域不全」的根因）。 */}
      {isRecording && (
        <div className="relative">
          <Button
            ref={infoBtnRef}
            variant="ghost"
            size="sm"
            className={cn(
              'gap-1.5 w-[120px] justify-start px-3 text-xs text-muted-foreground',
              infoOpen && 'bg-accent text-foreground'
            )}
            onClick={() => setInfoOpen((v) => !v)}
          >
            <Info className="h-3.5 w-3.5" />
            {t.recInfoToggle}
          </Button>
          {infoOpen && infoPos && createPortal(
            <div
              className="fixed z-[100] w-[280px] space-y-2 rounded-md border bg-popover p-3 text-[11px] text-muted-foreground shadow-xl"
              style={{ left: infoPos.left, top: infoPos.top }}
            >
              {/* 语音识别：模型名 + 流式/非流式 + 支持语言 + 模型简介 */}
              <div className="space-y-0.5">
                <p>
                  <span className="font-medium text-foreground">{t.recLabelAsr}</span>
                  {infoModelDisplay} · {infoModeLabel}
                </p>
                {infoModelLangs && (
                  <p className="text-muted-foreground/80">
                    <span className="font-medium text-foreground">{t.recLabelLangs}</span>
                    {infoModelLangs}
                  </p>
                )}
                {infoModelDesc && <p className="leading-relaxed text-muted-foreground/70">{infoModelDesc}</p>}
              </div>
              <div className="border-t border-border/50" />
              <p>
                <span className="font-medium text-foreground">{t.recTranslate}: </span>
                {translateEnabled ? `${infoEngineLabel} · ${infoTargetLang}` : t.recInfoTranslateOff}
              </p>
              <p className="break-all" title={defaultDevices.microphone || t.recNoDevice}>
                <span className="font-medium text-foreground">{t.recLabelMic}</span>
                {defaultDevices.microphone || t.recNoDevice}
              </p>
              <p className="break-all" title={defaultDevices.speaker || t.recNoDevice}>
                <span className="font-medium text-foreground">{t.recLabelSpeaker}</span>
                {defaultDevices.speaker || t.recNoDevice}
              </p>
              <p>
                <span className="font-medium text-foreground">{t.recInfoRemoteLabel}: </span>
                {infoRemoteStatus}
              </p>
            </div>,
            document.body
          )}
        </div>
      )}

      <RecordingSetupDialog open={setupOpen} onOpenChange={setSetupOpen} onConfirm={handleConfirmSetup} />
    </div>
  )
}

/** 单路迷你电平条（原始 RMS，开方缩放便于观察） */
/**
 * 单路电平条。muted=true 时进入「已静音」外观：
 *   整体压暗 + 灰白 45° 斜纹铺满 + 红色 MicOff 图标。
 * 为什么用斜纹而不是单纯变色：变色在深色主题下容易被误读成「音量小」，
 * 斜纹是通用的「已屏蔽」符号语言，12px 宽度下也能一眼区分（2026-09-30 用户要求）。
 */
function LevelMeter({
  label,
  value,
  title,
  muted = false,
}: {
  label: string
  value: number
  title: string
  muted?: boolean
}) {
  const h = Math.min(1, Math.sqrt(Math.max(0, value)) * 1.5)
  return (
    <div className={cn('flex items-center gap-1', muted && 'opacity-60')} title={title}>
      <span className="text-[10px] text-muted-foreground">{label}</span>
      <div className="h-5 w-1.5 rounded-sm bg-muted overflow-hidden flex flex-col-reverse">
        {muted ? (
          // 满格斜纹：表示「这条路被切断了」，与音量大小无关
          <div
            className="w-full"
            style={{
              height: '100%',
              backgroundImage:
                'repeating-linear-gradient(45deg, rgba(115,115,125,0.75) 0 2px, rgba(0,0,0,0) 2px 4px)',
            }}
          />
        ) : (
          <div className="w-full bg-primary transition-[height] duration-75" style={{ height: `${h * 100}%` }} />
        )}
      </div>
      {muted && <MicOff className="h-3 w-3 shrink-0 text-destructive" />}
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
  const isMicMuted = useAppStore((s) => s.isMicMuted)
  const setMicMuted = useAppStore((s) => s.setMicMuted)
  const t = useMessages()

  useAudioLevel()

  /** 点提示语直接取消静音（省得用户去找按钮）。失败则回滚，避免状态与后端不一致。 */
  const handleUnmuteMic = async () => {
    setMicMuted(false)
    try {
      await ipcSetMicMute(false)
    } catch {
      setMicMuted(true)
    }
  }

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
    <div className="flex gap-3 items-center h-8 w-full">
      <span className="sh-rec-dot text-primary shrink-0" />
      <span className="font-mono text-2xl font-semibold tracking-wider leading-none tabular-nums">
        {formatDuration(recordingDuration)}
      </span>
      {isPaused && <Badge variant="warning">{t.recPaused}</Badge>}
      {/* 顺序：系统音频在左、麦克风在右 —— 这样静音提示紧挨着麦克风条（2026-09-30 用户要求） */}
      <div className="flex items-center gap-2 shrink-0">
        <LevelMeter label={t.recSysShort} value={audioLevels.system} title={t.recSysLevelTitle} />
        <LevelMeter
          label={t.recMicShort}
          value={audioLevels.mic}
          title={isMicMuted ? t.recMicMutedHint : t.recMicLevelTitle}
          muted={isMicMuted}
        />
      </div>
      {/* 麦克风默认静音时，在能量条与频谱之间的空位给出可点击的提示。
          用 -webkit-box + line-clamp 两行截断（Tailwind 3.4 的 line-clamp-2 也等价），
          完整文案挂在 title 上，窗口很窄时也能看到全文。 */}
      {isMicMuted && (
        <button
          type="button"
          onClick={handleUnmuteMic}
          title={t.recMicMutedHint}
          className="min-w-0 flex-1 basis-24 max-w-[320px] overflow-hidden text-left text-[10px] leading-[1.15] text-amber-600 hover:underline [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical] dark:text-amber-500"
        >
          {t.recMicMutedHint}
        </button>
      )}
      {/* 能量条：ml-auto 整体靠右与计时/指示灯拉开间距；flex-1 加宽、max-w 限宽、min-w 防窄窗口溢出 */}
      <div className="ml-auto flex-1 min-w-[100px] max-w-[380px] h-full rounded-md border bg-muted/40 px-1.5 py-0.5">
        <AudioSpectrumBars />
      </div>
    </div>
  )
}
