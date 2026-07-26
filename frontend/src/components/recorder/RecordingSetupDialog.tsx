'use client'

import { useEffect, useMemo, useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useAppStore } from '@/state'
import { listAudioDevices, getDefaultAudioDevices, openSystemSoundSettings, setTranslationEnabled as ipcSetTranslationEnabled, setTranslationTargetLang as ipcSetTranslationTargetLang, setTranslationEngine as ipcSetTranslationEngine } from '@/services/ipc'
import { cn } from '@/lib/utils'
import { Check, ExternalLink, Mic, MonitorSpeaker } from 'lucide-react'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { getTranslateTargetLangs, translateTargetLangLabel, defaultTargetLang } from '@/lib/translateTargetLangs'
import type { AudioDevice, DefaultDevicesInfo, TranslationEngine } from '@/types'

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

/** 开始录音前的设置对话框：模型 / 设备 / 语言 / 静音选项 */
export function RecordingSetupDialog({ open, onOpenChange, onConfirm }: RecordingSetupDialogProps) {
  const models = useAppStore((s) => s.models)
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

  const [modelName, setModelName] = useState(selectedModel)
  const [micDevice, setMicDevice] = useState('')
  const [systemDevice, setSystemDevice] = useState('')
  const [language, setLanguage] = useState('auto')
  const [devices, setDevices] = useState<AudioDevice[]>([])
  const [defaults, setDefaults] = useState<DefaultDevicesInfo>({ microphone: null, speaker: null })

  const localModels = useMemo(() => models.filter((m) => !m.hidden && !m.is_remote), [models])
  const micOptions = useMemo(() => devices.filter((d) => d.device_type === 'Input'), [devices])
  const systemOptions = useMemo(() => devices.filter((d) => d.device_type === 'Output'), [devices])

  const isXAsr = modelName.startsWith('x-asr-')
  const selectedInfo = localModels.find((m) => m.name === modelName)
  const canStart = !!selectedInfo && selectedInfo.status !== 'Missing'

  const languageOptions = [
    { code: 'auto', name: t.recLangAuto },
    { code: 'zh', name: t.recLangZh },
    { code: 'en', name: t.recLangEn },
  ]

  const modelLabel = (name: string): string => {
    if (name === 'x-asr-480ms') return t.recModelXAsr
    if (name === 'sense-voice') return t.recModelSenseVoice
    return name
  }

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

  // 打开时重置为 store 里的当前选择并刷新设备列表；麦克风默认静音（与主窗口状态互通）
  useEffect(() => {
    if (!open) return
    setModelName(useAppStore.getState().selectedModel)
    setMicDevice('')
    setSystemDevice('')
    setLanguage('auto')
    setMicMuted(true)
    listAudioDevices().then(setDevices).catch(() => setDevices([]))
    getDefaultAudioDevices().then(setDefaults).catch(() => {})
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
    onOpenChange(false)
    onConfirm({
      modelName,
      micDeviceName: micDevice || null,
      systemDeviceName: systemDevice || null,
      language,
      micMuted: isMicMuted,
    })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle className="text-base">{t.recSetupTitle}</DialogTitle>
          <DialogDescription className="text-xs">
            {t.recSetupDesc}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-1">
          {/* ASR 模型选择 */}
          <section>
            <h3 className="text-xs font-semibold mb-2 text-muted-foreground">{t.recAsrModel}</h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
              {localModels.map((m) => {
                const available = m.status !== 'Missing'
                const active = modelName === m.name
                return (
                  <button
                    key={m.name}
                    type="button"
                    disabled={!available}
                    onClick={() => available && setModelName(m.name)}
                    className={cn(
                      'relative text-left rounded-lg border p-3 transition-all',
                      active
                        ? 'border-primary bg-primary/5 ring-1 ring-primary'
                        : 'border-border/60 bg-card/50 hover:border-primary/30',
                      !available && 'opacity-50 cursor-not-allowed'
                    )}
                  >
                    {active && (
                      <span className="absolute top-2 right-2 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-primary-foreground">
                        <Check className="h-2.5 w-2.5" />
                      </span>
                    )}
                    <div className="text-sm font-semibold">{modelLabel(m.name)}</div>
                    <div className="mt-0.5 text-[11px] text-muted-foreground">
                      {m.name === 'x-asr-480ms' ? t.recXAsrDesc : t.recSenseVoiceDesc}
                      {available ? '' : ` · ${t.recNotDownloaded}`}
                    </div>
                  </button>
                )
              })}
            </div>
          </section>

          {/* 设备选择 */}
          <section className="grid grid-cols-1 sm:grid-cols-2 gap-3">
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
          </section>

          {/* 语言 + 静音 */}
          <section className="grid grid-cols-1 sm:grid-cols-2 gap-3 items-end">
            <label className="flex flex-col gap-1">
              <span className="text-xs text-muted-foreground">{t.recRecogLang}</span>
              <select
                className={selectCls}
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
                disabled={isXAsr}
                title={isXAsr ? t.recXAsrLangTitle : undefined}
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
          </section>

          {/* 实时翻译 */}
          <section className="grid grid-cols-1 sm:grid-cols-2 gap-3 items-start">
            <div className="flex flex-col gap-2">
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
              {translateEnabled && (
                <label className="flex flex-col gap-1">
                  <span className="text-xs text-muted-foreground">{t.recTranslateEngine}</span>
                  <select
                    className={selectCls}
                    value={translationEngine}
                    onChange={(e) => handleEngineChange(e.target.value as TranslationEngine)}
                  >
                    <option value="opus">{t.recEngineOpus}</option>
                    <option value="hymt2">{t.recEngineHymt2}</option>
                  </select>
                </label>
              )}
            </div>
            {translateEnabled && (
              <label className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">{t.recTargetLang}</span>
                <select
                  className={selectCls}
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
            )}
          </section>

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
        </div>

        <div className="flex items-center justify-end gap-2">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t.comCancel}
          </Button>
          <Button size="sm" className="min-w-[120px]" disabled={!canStart} onClick={handleStart}>
            {t.recStart}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
