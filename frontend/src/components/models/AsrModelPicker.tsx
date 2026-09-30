'use client'

import { useEffect, useMemo, useRef, useState } from 'react'
import { useAppStore } from '@/state'
import { getRemoteEnabled } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import {
  useRemoteModelChoice,
  remoteModelDisplayName,
  formatModelPrice,
} from '@/lib/remoteModelChoice'
import { pickLangSegment } from '@/lib/langSegment'
import {
  getCachedCredits,
  creditsPerSecond,
  estimateRecordableSeconds,
  formatRecordableDuration,
  minimumCreditsToStart,
} from '@/lib/creditsEstimate'
import { useLanguageStore } from '@/stores/languageStore'
import { cn } from '@/lib/utils'
import { ModelSelectCard, type ModelCardBadge } from '@/components/models/ModelSelectCard'
import { AsrLanguagePicker } from '@/components/models/AsrLanguagePicker'
import { LOCAL_ASR_LANGUAGE_SUPPORT } from '@/lib/asrLanguages'
import { Cloud, HardDrive } from 'lucide-react'
import { toast } from 'sonner'
import type { ModelInfo } from '@/types'

/** 「远程」占位名归一化：后端写回的 qwen3-asr-remote* 统一视为前端占位名 'remote' */
export function normalizeAsrModelName(name: string | null | undefined): string {
  if (!name) return ''
  if (name === 'remote' || name.startsWith('qwen3-asr-remote')) return 'remote'
  return name
}

/** 本地模型名 → 展示名（不含「（本地）」后缀） */
export function localAsrModelLabel(
  name: string,
  t: { recModelXAsr: string; recModelSenseVoice: string },
): string {
  if (name.startsWith('x-asr-')) return t.recModelXAsr
  if (name === 'sense-voice') return t.recModelSenseVoice
  return name
}

/** 使用场景：realtime=实时转录（远程只列流式模型）；offline=历史记录离线重识别（远程只列非流式，本地只列 SenseVoice） */
export type AsrPickerScene = 'realtime' | 'offline'

export interface AsrModelOptions {
  /** 本地模型（realtime：X-ASR 排第一，SenseVoice 其次；offline：仅 SenseVoice） */
  localModels: ModelInfo[]
  remoteAsr: ReturnType<typeof useRemoteModelChoice>
  remoteEnabled: boolean
  /** 远程可用：开关开启且目录里有适用场景的 ASR 模型 */
  remoteAvailable: boolean
  /** 校验候选模型名当前是否可选（本地已下载 / 远程可用） */
  isValid: (name: string) => boolean
  /** 默认选择：realtime 为 X-ASR > SenseVoice > 远程；offline 为 SenseVoice > 远程 */
  defaultChoice: () => string
  /** 展示名：本地带（本地）后缀；远程显示当前选中的远程模型名 */
  label: (name: string) => string
  /** 当前模型支持的**识别语言**规范码（远程来自网关目录 language_codes；
   *  本地来自 LOCAL_ASR_LANGUAGE_SUPPORT）。空数组 = 未声明（仅自动检测）。 */
  supportedLanguages: (name: string) => string[]
}

/** ASR 模型选项数据源（录音设置弹窗 TAB1 与历史页重识别弹窗共用） */
export function useAsrModelOptions(scene: AsrPickerScene = 'realtime'): AsrModelOptions {
  const t = useMessages()
  const models = useAppStore((s) => s.models)
  // 实时/离线是两个独立的远程模型选择（后端分开持久化）
  const remoteAsr = useRemoteModelChoice(scene === 'offline' ? 'asr_offline' : 'asr')
  const [remoteEnabled, setRemoteEnabled] = useState(false)

  useEffect(() => {
    getRemoteEnabled()
      .then(setRemoteEnabled)
      .catch(() => setRemoteEnabled(false))
  }, [])

  // realtime：X-ASR 排第一，其余保持原有顺序（SenseVoice 自然排第二）
  // offline：只保留非流式的 SenseVoice（产品约定：离线重识别只用非流式模型）
  const localModels = useMemo(() => {
    let list = models.filter((m) => !m.hidden && !m.is_remote)
    if (scene === 'offline') list = list.filter((m) => m.name === 'sense-voice')
    return [...list].sort((a, b) => {
      const ax = a.name.startsWith('x-asr-') ? 0 : 1
      const bx = b.name.startsWith('x-asr-') ? 0 : 1
      return ax - bx
    })
  }, [models, scene])

  const remoteAvailable = remoteEnabled && remoteAsr.models.length > 0

  const isValid = (name: string): boolean => {
    const n = normalizeAsrModelName(name)
    if (!n) return false
    if (n === 'remote') return remoteAvailable
    return localModels.some((m) => m.name === n && m.status !== 'Missing')
  }

  const defaultChoice = (): string => {
    if (scene === 'realtime') {
      const xasr = localModels.find((m) => m.name.startsWith('x-asr-') && m.status !== 'Missing')
      if (xasr) return xasr.name
    }
    const sv = localModels.find((m) => m.name === 'sense-voice' && m.status !== 'Missing')
    if (sv) return sv.name
    if (remoteAvailable) return 'remote'
    return ''
  }

  const label = (name: string): string => {
    const n = normalizeAsrModelName(name)
    if (n === 'remote') {
      const m = remoteAsr.models.find((mm) => mm.id === remoteAsr.value)
      return m ? remoteModelDisplayName(m) : t.recRemoteModel
    }
    if (!n) return t.recNoModel
    return localAsrModelLabel(n, t) + t.mdLocalSuffix
  }

  /**
   * 当前模型支持的识别语言（规范码）。
   * · 远程 → 网关随模型目录下发的 `language_codes`（网关按协议维护，见
   *   gateway/src/asrLanguages.ts；客户端只发规范码，代码转换在网关）；
   * · 本地 → LOCAL_ASR_LANGUAGE_SUPPORT（SenseVoice 5 语种 / X-ASR 中英）；
   * · 未声明（旧网关没有该字段）→ 空数组，UI 只提供「自动检测」。
   */
  const supportedLanguages = (name: string): string[] => {
    const n = normalizeAsrModelName(name)
    if (n === 'remote') {
      const m = remoteAsr.models.find((mm) => mm.id === remoteAsr.value)
      return m?.language_codes ?? []
    }
    if (n.startsWith('x-asr-')) return LOCAL_ASR_LANGUAGE_SUPPORT['x-asr'] ?? []
    if (n === 'sense-voice') return LOCAL_ASR_LANGUAGE_SUPPORT['sense-voice'] ?? []
    return []
  }

  return { localModels, remoteAsr, remoteEnabled, remoteAvailable, isValid, defaultChoice, label, supportedLanguages }
}

interface AsrModelPickerProps {
  /** 当前选中的模型名（本地模型名或占位名 'remote'） */
  value: string
  onChange: (name: string) => void
  /** 使用场景：决定远程模型列流式还是非流式、本地是否只列 SenseVoice（默认 realtime） */
  scene?: AsrPickerScene
  /** 是否在底部展示流式/非流式说明小字（默认 true） */
  showHint?: boolean
  /** 识别语言（规范码）或 'auto'。传了 `onLanguageChange` 才启用语言选择（含模型切换时的自动回落）。 */
  language?: string
  onLanguageChange?: (code: string) => void
  /** 是否在组件内渲染识别语言选择行（默认 true）。
   *  录音设置弹窗把语言选择放到了底部按钮行，传 false 关掉内嵌行；
   *  历史页重识别弹窗保持默认（内嵌）。 */
  showLanguagePicker?: boolean
}

/** 分组标题行（本地 / 远程的视觉分区标题，加大加粗便于区分）。
 *  标题与徽标不换行不收缩（英文 "Local models" 曾被挤成两行）；图标同理。 */
function GroupHeader({ icon, title, badge }: { icon: React.ReactNode; title: string; badge: string }) {
  return (
    <div className="flex shrink-0 items-center gap-1.5 text-sm font-semibold text-foreground">
      {icon}
      <span className="whitespace-nowrap">{title}</span>
      <span className="whitespace-nowrap rounded bg-muted px-1.5 py-0.5 text-[10px] font-normal text-muted-foreground">{badge}</span>
    </div>
  )
}

/**
 * ASR 模型选择内容区（录音弹窗 TAB1 / 音频测试弹窗 / 历史页重识别弹窗共用）：
 * 本地模型与远程模型分区展示——本地卡片网格 + 远程高亮区（推荐引导 + 全部远程模型卡片）。
 */
export function AsrModelPicker({
  value,
  onChange,
  scene = 'realtime',
  showHint = true,
  language,
  onLanguageChange,
  showLanguagePicker = true,
}: AsrModelPickerProps) {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const { localModels, remoteAsr, remoteEnabled, supportedLanguages } = useAsrModelOptions(scene)
  const supported = supportedLanguages(value)
  const supportedKey = supported.join(',')

  /** 「按余额估算还能录多久」：只在实时转录场景、且当前确实选了远程模型时显示。
   *  余额走 60s 缓存（getRemoteUsage 每次实打网关），拉不到就整行不渲染——
   *  绝不因为拿不到余额而影响开始录音。 */
  const remoteSelected = normalizeAsrModelName(value) === 'remote'
  const selectedRemote = remoteAsr.models.find((m) => m.id === remoteAsr.value)
  const [credits, setCredits] = useState<number | null>(null)
  useEffect(() => {
    if (scene !== 'realtime' || !remoteSelected || !remoteEnabled) return
    let alive = true
    getCachedCredits()
      .then((c) => {
        if (alive) setCredits(c)
      })
      .catch(() => {})
    return () => {
      alive = false
    }
  }, [scene, remoteSelected, remoteEnabled, remoteAsr.value])

  // 模型切换后，原先选的语言可能不再受支持（例如从 Deepgram 的日语切到 MiMo 的中/英）
  // → 自动回落 auto，避免把上游不认识的代码发上去。
  // 首次挂载（例如从 localStorage 恢复了上个模型的语言）只静默回落、不弹提示，
  // 否则每次打开弹窗都会看到一个莫名其妙的提示。
  const langInitRef = useRef(false)
  useEffect(() => {
    if (!onLanguageChange) return
    const cur = language || 'auto'
    const ok = supportedKey.split(',').includes(cur)
    if (cur === 'auto' || ok) {
      langInitRef.current = true
      return
    }
    const firstRun = !langInitRef.current
    langInitRef.current = true
    onLanguageChange('auto')
    if (!firstRun) toast.info(t.recLangResetByModel)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, supportedKey])

  return (
    <div className="space-y-3">
      {/* ── 本地模型区 ── */}
      <div className="space-y-1.5">
        <div className="flex items-center justify-between gap-2">
          <GroupHeader
            icon={<HardDrive className="h-4 w-4" />}
            title={t.recAsrGroupLocal}
            badge={t.recBadgeLocalFree}
          />
          {/* 流式/非流式说明（realtime）：与「本地模型」标题同行右侧，解释本地卡片上的流式/非流式徽标。
              允许收缩换行但保持右对齐；完整文字同时在 title tooltip 里兜底。 */}
          {showHint && scene === 'realtime' && (
            <span
              className="min-w-0 text-right text-[11px] leading-snug text-muted-foreground/70"
              title={t.recStreamingHint}
            >
              {t.recStreamingHint}
            </span>
          )}
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {localModels.map((m) => {
            const available = m.status !== 'Missing'
            const isX = m.name.startsWith('x-asr-')
            return (
              <ModelSelectCard
                key={m.name}
                title={localAsrModelLabel(m.name, t) + t.mdLocalSuffix}
                description={
                  (isX ? t.recXAsrDesc : t.recSenseVoiceDesc) +
                  (available ? '' : ` · ${t.recNotDownloaded}`)
                }
                badges={
                  isX
                    ? [
                        { text: t.recRecommended.trim(), tone: 'recommended' },
                        { text: t.recBadgeLocalFree, tone: 'local' },
                        { text: t.recBadgeStreaming, tone: 'streaming' },
                      ]
                    : [
                        { text: t.recBadgeLocalFree, tone: 'local' },
                        { text: t.recBadgeBatch, tone: 'batch' },
                      ]
                }
                languages={t.recLangSupported.replace('{list}', isX ? t.recLangsXAsr : t.recLangsSenseVoice)}
                active={value === m.name}
                disabled={!available}
                onClick={() => available && onChange(m.name)}
              />
            )
          })}
        </div>
      </div>

      {/* ── 远程模型区（中性底色分区 + 推荐引导；选中态高亮在卡片上） ── */}
      <div className="space-y-1.5 rounded-lg border border-border/60 bg-muted/30 p-2.5">
        <GroupHeader
          icon={<Cloud className="h-4 w-4" />}
          title={t.recAsrGroupRemote}
          badge={t.recBadgeRemote}
        />
        {/* 远程模型的好处（引导用户使用远程） */}
        <p className="text-[11px] leading-relaxed text-muted-foreground">{t.recRemoteBenefits}</p>

        {!remoteEnabled ? (
          <p className="rounded-md border border-dashed border-border px-3 py-2 text-[11px] text-muted-foreground">
            {t.recRemoteEnableGuide}
          </p>
        ) : remoteAsr.models.length === 0 ? (
          !remoteAsr.loading && (
            <p className="text-[11px] text-amber-700">
              {scene === 'realtime' ? t.recRemoteNoStreaming : t.recRemoteNoBatch}
            </p>
          )
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {remoteAsr.models.map((m) => {
              const badges: ModelCardBadge[] = [
                { text: t.recBadgeRemote, tone: 'remote' as const },
                m.mode === 'streaming'
                  ? { text: t.recBadgeStreaming, tone: 'streaming' as const }
                  : { text: t.recBadgeBatch, tone: 'batch' as const },
                ...(m.recommended
                  ? [{ text: t.recRecommended.trim(), tone: 'recommended' as const }]
                  : []),
              ]
              const active = normalizeAsrModelName(value) === 'remote' && remoteAsr.value === m.id
              return (
                <ModelSelectCard
                  key={m.id}
                  title={remoteModelDisplayName(m)}
                  badges={badges}
                  tags={m.tags}
                  languages={m.languages ? t.recLangSupported.replace('{list}', pickLangSegment(m.languages, lang)) : undefined}
                  collapsibleLanguages
                  priceLabel={formatModelPrice(m, t)}
                  active={active}
                  onClick={() => {
                    // 先切换远程子模型（写后端选择），再激活「远程」占位选择
                    remoteAsr.set(m.id)
                    onChange('remote')
                  }}
                />
              )
            })}
          </div>
        )}

        {/* 额度预估（2026-09-29）：只在「实时转录 + 已选远程模型」时出现。
            口径严格复刻网关的冻结规则（先冻 2 分钟、之后每 60 秒阶梯补冻），
            所以预估值是保守的；余额不足 2 分钟额度时明确提示「无法开始」——
            那种情况下网关会在建会话时直接拒绝，整场都不会有字幕。 */}
        {scene === 'realtime' && remoteSelected && remoteEnabled && credits !== null && (() => {
          const cps = selectedRemote ? creditsPerSecond(selectedRemote) : null
          if (cps === null) return null
          const seconds = estimateRecordableSeconds(credits, cps)
          const tooLow = seconds <= 0
          return (
            <div className="space-y-0.5 pt-0.5">
              <p
                className={cn(
                  'text-[11px] leading-relaxed',
                  tooLow ? 'font-medium text-red-600 dark:text-red-400' : 'text-muted-foreground'
                )}
              >
                {tooLow
                  ? t.recCreditTooLow.replace('{min}', String(minimumCreditsToStart(cps)))
                  : t.recCreditEstimate
                      .replace('{credits}', String(Number(credits.toFixed(2))))
                      .replace('{duration}', formatRecordableDuration(seconds, t))}
              </p>
              <p className="text-[10px] leading-relaxed text-muted-foreground/70">{t.recCreditEstimateHint}</p>
            </div>
          )
        })()}
      </div>

      {/* 识别语言：紧挨模型区底部的紧凑单行（小标题 + 紧凑下拉，不再独占一个大区块），
          且**按所选模型过滤**（各模型支持语言不同，下拉即见，不再单列「支持：…」说明）。
          客户端只发规范码，到上游代码的转换由网关按协议完成。
          showLanguagePicker=false 时由调用方自行放置（录音设置弹窗放到底部按钮行）。 */}
      {onLanguageChange && showLanguagePicker && (
        <div className="flex justify-end">
          <AsrLanguagePicker
            value={language || 'auto'}
            onChange={onLanguageChange}
            supported={supported}
            showSupported={false}
          />
        </div>
      )}

      {/* 离线场景的说明小字仍留底部（realtime 的流式说明已上移到「本地模型」标题行） */}
      {showHint && scene === 'offline' && (
        <p className="text-[11px] text-muted-foreground/70">{t.recOfflineHint}</p>
      )}
    </div>
  )
}
