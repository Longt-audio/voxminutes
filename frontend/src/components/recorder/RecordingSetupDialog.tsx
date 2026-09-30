'use client'

import { useEffect, useMemo, useRef, useState } from 'react'
import { useRouter } from 'next/navigation'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useAppStore } from '@/state'
import { listAudioDevices, getDownloadableModels, getDefaultAudioDevices, openSystemSoundSettings, setTranslationEnabled as ipcSetTranslationEnabled, setTranslationTargetLang as ipcSetTranslationTargetLang, setTranslationEngine as ipcSetTranslationEngine, getRemoteEnabled, summaryGetConfig, warmRemoteStreaming } from '@/services/ipc'
import { ExternalLink, Mic, MonitorSpeaker, Activity, Cloud, HardDrive } from 'lucide-react'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { getTranslateTargetLangs, translateTargetLangLabel, defaultTargetLang } from '@/lib/translateTargetLangs'
import { availableTranslationEngines, fetchCustomApiConfigured } from '@/lib/translationEngines'
import { useRemoteModelChoice, remoteModelDisplayName, remoteModelOptionLabel, formatModelPrice } from '@/lib/remoteModelChoice'
import { pickLangSegment } from '@/lib/langSegment'
import { ModelSelectCard } from '@/components/models/ModelSelectCard'
import { AsrModelPicker, useAsrModelOptions, normalizeAsrModelName } from '@/components/models/AsrModelPicker'
import { AsrLanguagePicker } from '@/components/models/AsrLanguagePicker'
import { languageLabel, sortLanguages } from '@/lib/asrLanguages'
import { AudioTestDialog } from '@/components/recorder/AudioTestDialog'
import { cn } from '@/lib/utils'
import type { AudioDevice, DefaultDevicesInfo, TranslationEngine, DownloadableModelInfo } from '@/types'

export interface RecordingSetup {
  modelName: string
  micDeviceName: string | null
  systemDeviceName: string | null
  language: string
  micMuted: boolean
}

interface RecordingSetupDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (setup: RecordingSetup) => void
}

const selectCls =
  'h-8 w-full rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50'

/** 上次选择的 ASR 模型（localStorage），打开弹窗时优先恢复 */
const LAST_ASR_MODEL_KEY = 'vox.lastAsrModel'
/** 上次选择的识别语言（localStorage；规范码或 'auto'） */
const LAST_ASR_LANGUAGE_KEY = 'vox.lastAsrLanguage'

type SetupTab = 'asr' | 'translate' | 'audio'

/** 开始录音前的设置对话框：语音识别模型 / 翻译模型 / 音频通路 三个 TAB */
export function RecordingSetupDialog({ open, onOpenChange, onConfirm }: RecordingSetupDialogProps) {
  const selectedModel = useAppStore((s) => s.selectedModel)
  // 静音状态与主窗口静音按钮共享同一 store 状态（双向互通）
  const isMicMuted = useAppStore((s) => s.isMicMuted)
  const setMicMuted = useAppStore((s) => s.setMicMuted)
  // 实时翻译状态同样共享（与录音控制面板互通）
  const translateEnabled = useAppStore((s) => s.translateEnabled)
  const setTranslateEnabled = useAppStore((s) => s.setTranslateEnabled)
  const translateTargetLang = useAppStore((s) => s.translateTargetLang)
  const setTranslateTargetLang = useAppStore((s) => s.setTranslateTargetLang)
  const translationEngine = useAppStore((s) => s.translationEngine)
  const setTranslationEngine = useAppStore((s) => s.setTranslationEngine)
  const t = useMessages()
  const home = useLanguageStore((s) => s.language)

  const [tab, setTab] = useState<SetupTab>('asr')
  const [modelName, setModelName] = useState('')
  // 用户本次打开后是否手动选过模型（未手动选时跟随「恢复上次/默认」解析结果）
  const userPickedRef = useRef(false)
  const [micDevice, setMicDevice] = useState('')
  const [systemDevice, setSystemDevice] = useState('')
  // 识别语言：恢复上次选择（localStorage）——各模型支持语言不同，模型切换时
  // AsrModelPicker 会自动把不再受支持的语言回落为 auto。
  const [language, setLanguage] = useState<string>(() => {
    if (typeof window === 'undefined') return 'auto'
    return window.localStorage.getItem(LAST_ASR_LANGUAGE_KEY) || 'auto'
  })
  const handleLanguageChange = (code: string) => {
    setLanguage(code)
    try {
      window.localStorage.setItem(LAST_ASR_LANGUAGE_KEY, code)
    } catch {
      /* localStorage 不可用：忽略，仅本次生效 */
    }
  }
  const [devices, setDevices] = useState<AudioDevice[]>([])
  const [defaults, setDefaults] = useState<DefaultDevicesInfo>({ microphone: null, speaker: null })
  const [translationModels, setTranslationModels] = useState<DownloadableModelInfo[] | null>(null)
  const [remoteEnabled, setRemoteEnabled] = useState(false)
  const [customApiConfigured, setCustomApiConfigured] = useState(false)
  const [customApiInfo, setCustomApiInfo] = useState<{ endpoint: string; model: string } | null>(null)
  const [audioTestOpen, setAudioTestOpen] = useState(false)
  const router = useRouter()

  useEffect(() => {
    getDownloadableModels()
      .then(setTranslationModels)
      .catch(() => setTranslationModels([]))
  }, [])

  useEffect(() => {
    getRemoteEnabled()
      .then(setRemoteEnabled)
      .catch(() => setRemoteEnabled(false))
  }, [])

  // 打开弹窗即提前预热流式识别通道（fire-and-forget，命中 5 分钟缓存时后端是 no-op）：
  // 用户挑模型的这几秒握手已完成，点「开始」不再白等 ~3.5s。
  useEffect(() => {
    if (open) warmRemoteStreaming()
  }, [open])

  const translationEngines = translationModels === null
    ? (['opus', 'hymt2'] as const)
    : availableTranslationEngines(translationModels)

  // ASR 模型选项（TAB1 卡片与「记住上次选择」解析共用）
  const asrOptions = useAsrModelOptions()
  // 远程翻译模型（实时翻译引擎选「远程」时使用）
  const remoteTranslate = useRemoteModelChoice('translate', { excludeReasoning: true, usage: 'translate' })
  const selectedRemoteTranslate = remoteTranslate.models.find((m) => m.id === remoteTranslate.value)

  const micOptions = useMemo(() => devices.filter((d) => d.device_type === 'Input'), [devices])
  const systemOptions = useMemo(() => devices.filter((d) => d.device_type === 'Output'), [devices])

  // 初始模型解析：优先上次选择（localStorage > store selectedModel，需校验仍可选），
  // 否则按 X-ASR > SenseVoice > 远程 的顺序取默认；都不满足则不选。
  const resolvedInitialModel = useMemo(() => {
    let last = ''
    try {
      last = window.localStorage.getItem(LAST_ASR_MODEL_KEY) || ''
    } catch {}
    for (const candidate of [last, selectedModel]) {
      const n = normalizeAsrModelName(candidate)
      if (n && asrOptions.isValid(n)) return n
    }
    return asrOptions.defaultChoice()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [asrOptions.localModels, asrOptions.remoteAvailable, selectedModel])

  // 用户未手动选择前，跟随解析结果（远程目录/模型列表异步就绪后会自动收敛）
  const effectiveModelName = userPickedRef.current ? modelName : resolvedInitialModel
  const handleModelChange = (name: string) => {
    userPickedRef.current = true
    setModelName(name)
  }

  const isXAsr = effectiveModelName.startsWith('x-asr-')
  const isRemote = normalizeAsrModelName(effectiveModelName) === 'remote'
  // 远程 ASR 不依赖本地模型下载，只要开关开启且已选远程模型即可开始
  const canStart = effectiveModelName
    ? isRemote
      ? asrOptions.remoteAvailable && !!asrOptions.remoteAsr.value
      : asrOptions.localModels.some((m) => m.name === effectiveModelName && m.status !== 'Missing')
    : false

  // 识别语言候选：**按当前 ASR 模型过滤**（各模型支持语言不同）。
  // 客户端只发规范码；到豆包 zh-CN / Deepgram 粤语 zh-HK 等上游代码的转换在网关完成。
  const supportedAsrLangs = asrOptions.supportedLanguages(effectiveModelName)
  const languageOptions = [
    { code: 'auto', name: t.recLangAuto },
    ...sortLanguages(supportedAsrLangs).map((code) => ({
      code,
      name: languageLabel(code, home),
    })),
  ]

  // 目标语言选项按引擎动态生成（全量，不排除 home）；zh/en 沿用录音面板既有文案，其余用语言名
  const targetLangOptions = getTranslateTargetLangs(translationEngine).map((code) => ({
    code,
    name:
      code === 'en'
        ? t.recTranslateToEn
        : code === 'zh'
          ? t.recTranslateToZh
          : translateTargetLangLabel(code, t),
  }))

  // 切换引擎后若当前目标语言不再可用（如 hymt2 的日语切到 opus），回退默认目标
  const handleEngineChange = (engine: TranslationEngine) => {
    const prev = translationEngine
    setTranslationEngine(engine)
    ipcSetTranslationEngine(engine).catch(() => setTranslationEngine(prev))
    if (!getTranslateTargetLangs(engine).includes(translateTargetLang)) {
      const fallback = defaultTargetLang(home)
      setTranslateTargetLang(fallback)
      ipcSetTranslationTargetLang(fallback).catch(() => {})
    }
  }

  // 打开时回到第一个 tab、重新走「恢复上次选择」并刷新设备列表；麦克风默认静音（与主窗口状态互通）
  useEffect(() => {
    if (!open) return
    setTab('asr')
    userPickedRef.current = false
    setMicDevice('')
    setSystemDevice('')
    setLanguage('auto')
    setMicMuted(true)
    listAudioDevices().then(setDevices).catch(() => setDevices([]))
    getDefaultAudioDevices().then(setDefaults).catch(() => {})
    // 自定义 API 引擎可用性与展示信息（endpoint · model）
    fetchCustomApiConfigured().then(setCustomApiConfigured).catch(() => setCustomApiConfigured(false))
    summaryGetConfig()
      .then((c) => setCustomApiInfo(c ? { endpoint: c.endpoint, model: c.model } : null))
      .catch(() => setCustomApiInfo(null))
    // 当前目标语言若已不在可选项内（引擎变化），回退默认目标并写回后端
    const cur = useAppStore.getState().translateTargetLang
    const eng = useAppStore.getState().translationEngine
    if (!getTranslateTargetLangs(eng).includes(cur)) {
      const next = defaultTargetLang(home)
      setTranslateTargetLang(next)
      ipcSetTranslationTargetLang(next).catch(() => {})
    }
  }, [open, setMicMuted, home, setTranslateTargetLang])

  const handleStart = () => {
    if (!canStart) return
    // 记住本次选择，下次打开弹窗优先恢复
    try {
      window.localStorage.setItem(LAST_ASR_MODEL_KEY, effectiveModelName)
    } catch {}
    onOpenChange(false)
    onConfirm({
      modelName: effectiveModelName,
      micDeviceName: micDevice || null,
      systemDeviceName: systemDevice || null,
      language,
      micMuted: isMicMuted,
    })
  }

  // 音频测试：后端 audio_test 只认 qwen3-asr-remote 前缀，把占位名 'remote' 映射过去
  // （与 useRecorder.startRecording 的映射保持一致）；本地模型名原样传。
  const audioTestModelName = effectiveModelName === 'remote' ? 'qwen3-asr-remote' : effectiveModelName
  const audioTestModelDisplay = asrOptions.label(effectiveModelName)

  const tabCls = (key: SetupTab) =>
    cn(
      'px-1 pb-2 text-xs font-medium border-b-2 -mb-px transition-colors',
      tab === key
        ? 'border-primary text-primary'
        : 'border-transparent text-muted-foreground hover:text-foreground'
    )

  return (
    <>
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[86vh] w-[860px] max-w-[94vw] flex-col">
        <DialogHeader className="shrink-0">
          <DialogTitle className="text-base">{t.recSetupTitle}</DialogTitle>
          <DialogDescription className="text-xs">
            {t.recSetupDesc}
          </DialogDescription>
        </DialogHeader>

        {/* TAB 切换（与历史页详情 tab 同一风格） */}
        <div className="flex shrink-0 gap-4 border-b border-border/60">
          <button className={tabCls('asr')} onClick={() => setTab('asr')}>
            {t.recTabAsr}
          </button>
          <button className={tabCls('translate')} onClick={() => setTab('translate')}>
            {t.recTabTranslate}
          </button>
          <button className={tabCls('audio')} onClick={() => setTab('audio')}>
            {t.recTabAudio}
          </button>
        </div>

        {/* 内容区：超出高度（英文长文案/矮窗口）时内部滚动，底部按钮行始终可见 */}
        <div className="py-1 min-h-[240px] flex-1 overflow-y-auto">
          {/* TAB1 语音识别模型（音频测试入口在底部与「开始录音」同一排） */}
          {tab === 'asr' && (
            <section className="space-y-2">
              {/* 模型选择 tips：小字弱化，放在模型选择最上方 */}
              <p className="text-[11px] leading-relaxed text-muted-foreground/80">{t.recModelPickTips}</p>
              <AsrModelPicker
                value={effectiveModelName}
                onChange={handleModelChange}
                scene="realtime"
                language={language}
                onLanguageChange={handleLanguageChange}
                showLanguagePicker={false}
              />
            </section>
          )}

          {/* TAB2 翻译模型（无论是否勾选实时翻译都展示模型列表，铺满宽度的 2 列网格） */}
          {tab === 'translate' && (
            <section className="space-y-2.5">
              {/* 勾选行 + 目标语言（右侧） */}
              <div className="flex items-center justify-between gap-3">
                <label className="flex items-center gap-2 h-8 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    className="h-3.5 w-3.5 accent-[#ff4b4b]"
                    checked={translateEnabled}
                    onChange={(e) => {
                      const next = e.target.checked
                      setTranslateEnabled(next)
                      ipcSetTranslationEnabled(next).catch(() => setTranslateEnabled(!next))
                    }}
                  />
                  <span className="text-xs">{t.recTranslateCheck}</span>
                </label>
                <label className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground whitespace-nowrap">{t.recTargetLang}</span>
                  <select
                    className={cn(selectCls, 'w-auto min-w-[110px]')}
                    value={translateTargetLang}
                    onChange={(e) => {
                      const lang = e.target.value
                      const prev = translateTargetLang
                      setTranslateTargetLang(lang)
                      ipcSetTranslationTargetLang(lang).catch(() => setTranslateTargetLang(prev))
                    }}
                  >
                    {targetLangOptions.map((o) => (
                      <option key={o.code} value={o.code}>
                        {o.name}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 items-start">
                {/* 本地引擎组（视觉分区，标题跨两列；标题与徽标不换行） */}
                <span className="sm:col-span-2 flex items-center gap-1.5 text-sm font-semibold text-foreground">
                  <HardDrive className="h-4 w-4 shrink-0" />
                  <span className="whitespace-nowrap">{t.recAsrGroupLocal}</span>
                  <span className="whitespace-nowrap rounded bg-muted px-1.5 py-0.5 text-[10px] font-normal text-muted-foreground">{t.recBadgeLocalFree}</span>
                </span>
                {translationEngines.includes('hymt2') && (
                  <ModelSelectCard
                    title={`Hy-MT2${t.mdLocalSuffix}`}
                    description={t.recEngineHymt2}
                    badges={[
                      { text: t.recRecommended.trim(), tone: 'recommended' },
                      { text: t.recBadgeLocalFree, tone: 'local' },
                    ]}
                    active={translationEngine === 'hymt2'}
                    onClick={() => handleEngineChange('hymt2')}
                  />
                )}
                {translationEngines.includes('opus') && (
                  <ModelSelectCard
                    title={`OPUS-MT${t.mdLocalSuffix}`}
                    description={t.recEngineOpus}
                    badges={[{ text: t.recBadgeLocalFree, tone: 'local' }]}
                    active={translationEngine === 'opus'}
                    onClick={() => handleEngineChange('opus')}
                  />
                )}
                {/* 远程/自定义组（中性底色分区，整组跨两列；内部卡片同样 2 列铺满） */}
                <div className="sm:col-span-2 flex flex-col gap-2 rounded-lg border border-border/60 bg-muted/30 p-2.5">
                  <span className="flex items-center gap-1.5 text-sm font-semibold text-foreground">
                    <Cloud className="h-4 w-4 shrink-0" />
                    <span className="whitespace-nowrap">{t.recAsrGroupRemote}</span>
                    <span className="whitespace-nowrap rounded bg-muted px-1.5 py-0.5 text-[10px] font-normal text-muted-foreground">{t.recBadgeRemote}</span>
                  </span>
                  {remoteEnabled && remoteTranslate.models.length > 0 && (
                    <select
                      className={selectCls}
                      value={remoteTranslate.value}
                      onChange={(e) => remoteTranslate.set(e.target.value)}
                    >
                      {remoteTranslate.models.map((m) => (
                        <option key={m.id} value={m.id}>
                          {remoteModelOptionLabel(m, t)}
                        </option>
                      ))}
                    </select>
                  )}
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                    {remoteEnabled && remoteTranslate.models.length > 0 && selectedRemoteTranslate && (
                      <ModelSelectCard
                        title={remoteModelDisplayName(selectedRemoteTranslate)}
                        badges={[
                          { text: t.recBadgeRemote, tone: 'remote' as const },
                          ...(selectedRemoteTranslate.recommended
                            ? [{ text: t.recRecommended.trim(), tone: 'recommended' as const }]
                            : []),
                        ]}
                        tags={selectedRemoteTranslate.tags}
                        languages={
                          selectedRemoteTranslate.languages
                            ? t.recLangSupported.replace('{list}', pickLangSegment(selectedRemoteTranslate.languages, home))
                            : undefined
                        }
                        collapsibleLanguages
                        priceLabel={formatModelPrice(selectedRemoteTranslate, t)}
                        active={translationEngine === 'remote'}
                        onClick={() => handleEngineChange('remote')}
                      />
                    )}
                    {/* 自定义 API 卡片始终显示（未配置时点击跳转「设置 → 自定义 LLM」配置） */}
                    <ModelSelectCard
                      title={t.recEngineCustomApi}
                      description={
                        customApiConfigured && customApiInfo
                          ? [customApiInfo.endpoint, customApiInfo.model].filter(Boolean).join(' · ')
                          : t.trCustomApiNotConfigured
                      }
                      badges={[{ text: t.recEngineCustomApi, tone: 'remote' }]}
                      active={translationEngine === 'custom-api'}
                      onClick={() => {
                        if (customApiConfigured) handleEngineChange('custom-api')
                        else router.push('/settings?tab=customApi')
                      }}
                    />
                  </div>
                </div>
              </div>
            </section>
          )}

          {/* TAB3 音频通路 */}
          {tab === 'audio' && (
            <section className="space-y-3">
              {/* 设备选择 */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <label className="flex flex-col gap-1">
                  <span className="text-xs text-muted-foreground flex items-center gap-1">
                    <Mic className="h-3 w-3" /> {t.recMic}
                  </span>
                  <select className={selectCls} value={micDevice} onChange={(e) => setMicDevice(e.target.value)}>
                    <option value="">{t.recSystemDefault}</option>
                    {micOptions.map((d) => (
                      <option key={d.name} value={d.name}>{d.name}</option>
                    ))}
                  </select>
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-xs text-muted-foreground flex items-center gap-1">
                    <MonitorSpeaker className="h-3 w-3" /> {t.recSystemAudio}
                  </span>
                  <select className={selectCls} value={systemDevice} onChange={(e) => setSystemDevice(e.target.value)}>
                    <option value="">{t.recSystemDefault}</option>
                    {systemOptions.map((d) => (
                      <option key={d.name} value={d.name}>{d.name}</option>
                    ))}
                  </select>
                </label>
              </div>

              {/* 语言 + 静音 */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 items-end">
                <label className="flex flex-col gap-1">
                  <span className="text-xs text-muted-foreground">{t.recRecogLang}</span>
                  <select
                    className={selectCls}
                    value={language}
                    onChange={(e) => handleLanguageChange(e.target.value)}
                    disabled={supportedAsrLangs.length === 0}
                    title={supportedAsrLangs.length === 0 ? t.recLangAutoOnly : undefined}
                  >
                    {languageOptions.map((l) => (
                      <option key={l.code} value={l.code}>{l.name}</option>
                    ))}
                  </select>
                </label>
                <label className="flex items-center gap-2 h-8 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    className="h-3.5 w-3.5 accent-[#ff4b4b]"
                    checked={isMicMuted}
                    onChange={(e) => setMicMuted(e.target.checked)}
                  />
                  <span className="text-xs">{t.recMuteOnStart}</span>
                </label>
              </div>

              {/* 默认设备提示 */}
              <div className="flex items-center justify-between gap-2 rounded-md bg-muted/50 px-3 py-2">
                <p className="text-[11px] text-muted-foreground truncate">
                  {t.recCurrentDefault
                    .replace('{mic}', defaults.microphone || t.recNoDevice)
                    .replace('{speaker}', defaults.speaker || t.recNoDevice)}
                </p>
                <button
                  type="button"
                  className="text-[11px] text-primary hover:underline flex items-center gap-0.5 shrink-0"
                  onClick={() => openSystemSoundSettings().catch(() => {})}
                >
                  <ExternalLink className="h-3 w-3" /> {t.recSoundSettings}
                </button>
              </div>
            </section>
          )}
        </div>

        {/* 底部行：左侧识别语言（从模型列表下方移到此处，页面更紧凑；与 TAB3 的语言下拉共享同一 state），
            右侧 取消 → 语音模型测试（次级 outline）→ 开始录音 */}
        <div className="flex shrink-0 flex-wrap items-center justify-between gap-2">
          <AsrLanguagePicker
            value={language}
            onChange={handleLanguageChange}
            supported={supportedAsrLangs}
            showSupported={false}
          />
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
              {t.comCancel}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              disabled={!canStart}
              title={t.recTestEntryHint}
              onClick={() => setAudioTestOpen(true)}
            >
              <Activity className="h-3.5 w-3.5" />
              {t.recAudioSelfTest}
            </Button>
            <Button size="sm" className="min-w-[120px]" disabled={!canStart} onClick={handleStart}>
              {t.recStart}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>

    {/* 音频测试为独立二级弹窗：点「完成」只关测试弹窗回到本设置弹窗，不会自动开始录音。
        测试弹窗内可切换 ASR 模型与采集设备——与本弹窗共享同一组 state，选择即时互通。
        注意：start_audio_test 会把被测模型保存为当前转写配置（既有行为，保留）。 */}
    <AudioTestDialog
      open={audioTestOpen}
      onOpenChange={setAudioTestOpen}
      modelName={audioTestModelName}
      modelDisplayName={audioTestModelDisplay}
      asrValue={effectiveModelName}
      onAsrChange={handleModelChange}
      remoteAsrValue={asrOptions.remoteAsr.value}
      onRemoteAsrChange={asrOptions.remoteAsr.set}
      devices={devices}
      micDevice={micDevice}
      systemDevice={systemDevice}
      onMicDeviceChange={setMicDevice}
      onSystemDeviceChange={setSystemDevice}
    />
    </>
  )
}
