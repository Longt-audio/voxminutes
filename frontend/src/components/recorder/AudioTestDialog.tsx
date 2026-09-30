'use client'

import { useEffect, useRef, useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useAudioTest } from '@/hooks/useAudioTest'
import { AudioWaveformBar } from '@/components/recorder/AudioWaveformBar'
import {
  useAsrModelOptions,
  normalizeAsrModelName,
  localAsrModelLabel,
} from '@/components/models/AsrModelPicker'
import { ModelSelectCard } from '@/components/models/ModelSelectCard'
import { remoteModelDisplayName, formatModelPrice } from '@/lib/remoteModelChoice'
import { pickLangSegment } from '@/lib/langSegment'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { Activity, AlertCircle, Mic, MonitorSpeaker, Play, Volume2 } from 'lucide-react'
import type { AudioDevice, TranscriptSegment } from '@/types'

interface AudioTestDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** 传给后端的实际模型名（远程占位名已映射为 qwen3-asr-remote） */
  modelName: string
  /** 展示用的模型名（本地带「（本地）」后缀 / 远程显示名） */
  modelDisplayName: string
  /** ASR 模型选择（与上级录音设置弹窗共享同一 state，切换即时同步） */
  asrValue: string
  onAsrChange: (name: string) => void
  /** 远程 ASR 子模型选择（与上级共享 useRemoteModelChoice('asr') 实例，避免选择状态分叉） */
  remoteAsrValue: string
  onRemoteAsrChange: (id: string) => void
  /** 采集设备列表与当前选择（同样与上级共享） */
  devices: AudioDevice[]
  micDevice: string
  systemDevice: string
  onMicDeviceChange: (name: string) => void
  onSystemDeviceChange: (name: string) => void
}

function formatSeconds(totalSeconds: number): string {
  const seconds = Math.floor(totalSeconds % 60)
  const minutes = Math.floor(totalSeconds / 60)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(minutes)}:${pad(seconds)}`
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

function TranscriptLine({ seg }: { seg: TranscriptSegment }) {
  if (seg.is_partial) {
    return (
      <p className="text-sm leading-relaxed italic text-muted-foreground">
        {seg.text}
      </p>
    )
  }

  return (
    <div className="flex items-start gap-3">
      {seg.audio_start_time > 0 && (
        <span className="text-xs text-muted-foreground/60 tabular-nums shrink-0 mt-0.5 min-w-[50px] text-right">
          {formatTime(seg.audio_start_time)}
        </span>
      )}
      <p className="text-sm leading-relaxed text-foreground/80">{seg.text}</p>
    </div>
  )
}

const selectCls =
  'h-8 w-full rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50'

/** 音频链路自检弹窗（RecordingSetupDialog 的二级弹窗；点「完成」仅关闭本弹窗）。
 *  打开后处于待开始状态：用户选好模型/设备后点「开始测试」才启动链路；
 *  播放结束后自动停止（远程流式模型按时长计费，不能挂着空转）。
 *  弹窗内可直接切换 ASR 模型与采集设备——与上级弹窗共享 state，选择即时同步；
 *  ⚠️ 更换 ASR 模型或采集设备后不会自动重启测试：必须重新点击「开始测试」
 *  才会用新配置跑一次（测试进行中切换会先停掉当前测试，回到待开始状态）。 */
export function AudioTestDialog({
  open,
  onOpenChange,
  modelName,
  modelDisplayName,
  asrValue,
  onAsrChange,
  remoteAsrValue,
  onRemoteAsrChange,
  devices,
  micDevice,
  systemDevice,
  onMicDeviceChange,
  onSystemDeviceChange,
}: AudioTestDialogProps) {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  // 待开始状态：打开弹窗不自动启动；「重新测试」通过递增 runId 重跑
  const [started, setStarted] = useState(false)
  const [runId, setRunId] = useState(0)
  // 紧凑下拉的选项数据（实时场景：本地 + 远程流式）。选择状态仍由上级共享（asrValue / remoteAsrValue props）。
  const { localModels, remoteAsr, remoteAvailable } = useAsrModelOptions('realtime')
  const { isLoading, error, segments, duration, progress, isPlaying, finished, levelPeak, levelActive, replay } =
    useAudioTest(modelName, open && started, micDevice || null, systemDevice || null, runId, remoteAsrValue || null)

  const scrollRef = useRef<HTMLDivElement>(null)

  // 弹窗关闭时回到待开始状态（下次打开重新点「开始测试」）
  useEffect(() => {
    if (!open) {
      setStarted(false)
      setRunId(0)
    }
  }, [open])

  // ⚠️ 更换 ASR 模型/采集设备后必须重新点击「开始测试」：
  // 测试进行中（含播放结束保留结果时）切换任意选择 → 停掉当前测试，回到待开始状态。
  // started 走 ref 读取，避免把 started 放进依赖导致「点开始→立即被重置」。
  const startedRef = useRef(false)
  startedRef.current = started
  useEffect(() => {
    if (startedRef.current) {
      setStarted(false)
    }
  }, [asrValue, remoteAsrValue, micDevice, systemDevice])

  // 新识别结果到达时自动滚到底部
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    el.scrollTo({ top: el.scrollHeight, behavior: 'instant' })
  }, [segments])

  // 模型加载中禁止通过点击遮罩/ESC 关闭（加载完成后由 hook 清理负责 stop）
  const handleOpenChange = (value: boolean) => {
    if (isLoading && !value) return
    onOpenChange(value)
  }

  const handleStart = () => {
    if (started) setRunId((r) => r + 1) // 重新测试：started 已为 true，用 runId 触发重启
    setStarted(true)
  }

  const elapsedSeconds = (progress / 100) * duration
  const remainingSeconds = Math.max(0, duration - elapsedSeconds)

  const micOptions = devices.filter((d) => d.device_type === 'Input')
  const systemOptions = devices.filter((d) => d.device_type === 'Output')

  // 紧凑下拉：本地选项值 = 模型名；远程选项值 = `remote:{真实模型id}`
  const isRemote = normalizeAsrModelName(asrValue) === 'remote'
  const asrSelectValue = isRemote ? `remote:${remoteAsrValue}` : asrValue
  const handleAsrSelect = (v: string) => {
    if (v.startsWith('remote:')) {
      // 先切换远程子模型（写后端选择），再激活「远程」占位选择
      onRemoteAsrChange(v.slice('remote:'.length))
      onAsrChange('remote')
    } else {
      onAsrChange(v)
    }
  }

  // 当前所选 ASR 模型的卡片信息（名称/徽标/支持语言/价格，选中态展示）
  const selectedLocal = !isRemote ? localModels.find((m) => m.name === asrValue) : undefined
  const selectedRemote = isRemote ? remoteAsr.models.find((m) => m.id === remoteAsrValue) : undefined

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-lg max-h-[88vh] overflow-y-auto custom-scrollbar [&_*]:min-w-0">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            {t.recTestTitle}
          </DialogTitle>
          <DialogDescription>{t.recTestDesc}</DialogDescription>
        </DialogHeader>

        <div className="space-y-2.5 py-1.5">
          {/* ── 顶部：ASR 模型 + 采集设备 + 开始测试按钮 ── */}
          <div className="rounded-lg border border-border/60 bg-card/50 p-3 space-y-2">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-foreground">{t.recTestModelLabel}</span>
              <select className={selectCls} value={asrSelectValue} onChange={(e) => handleAsrSelect(e.target.value)}>
                {localModels.map((m) => (
                  <option key={m.name} value={m.name} disabled={m.status === 'Missing'}>
                    {localAsrModelLabel(m.name, t) + t.mdLocalSuffix}
                    {m.status === 'Missing' ? ` · ${t.recNotDownloaded}` : ''}
                  </option>
                ))}
                {remoteAvailable &&
                  remoteAsr.models.map((m) => (
                    <option key={m.id} value={`remote:${m.id}`}>
                      {remoteModelDisplayName(m)}
                    </option>
                  ))}
              </select>
            </label>

            {/* 所选 ASR 模型卡片（选中态，展示名称/徽标/支持语言/价格等关键信息） */}
            {selectedLocal && (
              <ModelSelectCard
                title={localAsrModelLabel(selectedLocal.name, t) + t.mdLocalSuffix}
                description={
                  selectedLocal.name.startsWith('x-asr-') ? t.recXAsrDesc : t.recSenseVoiceDesc
                }
                badges={
                  selectedLocal.name.startsWith('x-asr-')
                    ? [
                        { text: t.recRecommended.trim(), tone: 'recommended' as const },
                        { text: t.recBadgeLocalFree, tone: 'local' as const },
                        { text: t.recBadgeStreaming, tone: 'streaming' as const },
                      ]
                    : [
                        { text: t.recBadgeLocalFree, tone: 'local' as const },
                        { text: t.recBadgeBatch, tone: 'batch' as const },
                      ]
                }
                languages={t.recLangSupported.replace(
                  '{list}',
                  selectedLocal.name.startsWith('x-asr-') ? t.recLangsXAsr : t.recLangsSenseVoice,
                )}
                active
              />
            )}
            {selectedRemote && (
              <ModelSelectCard
                title={remoteModelDisplayName(selectedRemote)}
                badges={[
                  { text: t.recBadgeRemote, tone: 'remote' as const },
                  selectedRemote.mode === 'streaming'
                    ? { text: t.recBadgeStreaming, tone: 'streaming' as const }
                    : { text: t.recBadgeBatch, tone: 'batch' as const },
                  ...(selectedRemote.recommended
                    ? [{ text: t.recRecommended.trim(), tone: 'recommended' as const }]
                    : []),
                ]}
                tags={selectedRemote.tags}
                languages={
                  selectedRemote.languages
                    ? t.recLangSupported.replace('{list}', pickLangSegment(selectedRemote.languages, lang))
                    : undefined
                }
                collapsibleLanguages
                priceLabel={formatModelPrice(selectedRemote, t)}
                active
              />
            )}

            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 [&>*]:min-w-0">
              <label className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground flex items-center gap-1">
                  <Mic className="h-3 w-3" /> {t.recMic}
                </span>
                <select
                  className={selectCls}
                  value={micDevice}
                  onChange={(e) => onMicDeviceChange(e.target.value)}
                >
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
                <select
                  className={selectCls}
                  value={systemDevice}
                  onChange={(e) => onSystemDeviceChange(e.target.value)}
                >
                  <option value="">{t.recSystemDefault}</option>
                  {systemOptions.map((d) => (
                    <option key={d.name} value={d.name}>{d.name}</option>
                  ))}
                </select>
              </label>
            </div>

            <div className="flex items-center gap-2 pt-0.5">
              {(!started || finished) && (
                <Button size="sm" className="flex-1" onClick={handleStart} disabled={!asrValue}>
                  {finished ? t.recTestRetest : t.recTestStart}
                </Button>
              )}
              {started && !finished && (
                <p className="flex-1 text-xs text-muted-foreground">
                  {isLoading ? t.recTestLoadingModel : t.recTestWaiting}
                </p>
              )}
            </div>
          </div>

          {/* ── 播放器 / 进度条 ── */}
          <div className="rounded-lg border border-border/60 bg-card/50 p-3 space-y-2">
            <div className="flex items-center gap-3">
              <button
                type="button"
                disabled={isLoading || isPlaying || !started || finished}
                onClick={replay}
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary disabled:opacity-60 disabled:cursor-not-allowed hover:bg-primary/20 transition-colors"
                aria-label={isPlaying ? t.recTestPlaying : t.recTestReplay}
                title={isPlaying ? t.recTestPlaying : t.recTestReplay}
              >
                {isPlaying ? <Volume2 className="h-4 w-4" /> : <Play className="h-4 w-4" />}
              </button>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">example_audio.wav</p>
                <div className="mt-1.5 h-1.5 w-full rounded-full bg-muted overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all duration-100"
                    style={{ width: `${progress}%` }}
                  />
                </div>
              </div>
              <span className="text-xs text-muted-foreground tabular-nums whitespace-nowrap">
                {formatSeconds(remainingSeconds)} / {formatSeconds(duration)}
              </span>
            </div>
          </div>

          {/* 识别结果 */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-foreground">
              {t.recTestResultLabel}（{modelDisplayName}）
            </label>
            <div
              ref={scrollRef}
              className="min-h-[72px] max-h-[150px] overflow-y-auto rounded-md border border-border/60 bg-card/50 p-2.5 space-y-2"
            >
              {segments.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  {!started ? t.recTestIdleHint : isLoading ? t.recTestLoadingModel : t.recTestWaiting}
                </p>
              ) : (
                segments.map((seg) => (
                  <div key={seg.id}>
                    <TranscriptLine seg={seg} />
                  </div>
                ))
              )}
            </div>
            {/* 播放结束后自动停止（保留结果显示），远程按时长计费的提示 */}
            {finished && (
              <p className="text-xs text-muted-foreground">{t.recTestAutoStopped}</p>
            )}
          </div>

          {/* 音频能量 */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-foreground">{t.recTestEnergy}</label>
            <div className="h-10 w-full rounded-md border border-border/60 bg-card/50 p-1">
              <AudioWaveformBar peak={levelPeak} active={levelActive} />
            </div>
          </div>

          {/* 错误提示 */}
          {error && (
            <div className="rounded-lg border border-red-200 bg-red-50 dark:border-red-900/50 dark:bg-red-900/20 p-3">
              <div className="flex gap-2.5">
                <AlertCircle className="h-4 w-4 text-red-600 dark:text-red-400 shrink-0 mt-0.5" />
                <div className="space-y-1">
                  <p className="text-sm font-medium text-red-800 dark:text-red-300">{t.recTestErrorTitle}</p>
                  <p className="text-xs text-red-700 dark:text-red-400">{error}</p>
                  <p className="text-xs text-red-700 dark:text-red-400">{t.recTestErrorHint}</p>
                </div>
              </div>
            </div>
          )}
        </div>

        <div className="flex justify-end pt-1.5">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)} disabled={isLoading}>
            {t.recTestClose}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
