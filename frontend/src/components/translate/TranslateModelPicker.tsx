'use client'

import { useEffect, useState } from 'react'
import { useRouter } from 'next/navigation'
import { getDownloadableModels, getRemoteEnabled, summaryGetConfig } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { availableTranslationEngines, fetchCustomApiConfigured } from '@/lib/translationEngines'
import {
  formatModelPrice,
  remoteModelDisplayName,
  useRemoteModelChoice,
} from '@/lib/remoteModelChoice'
import { ModelSelectCard, type ModelCardBadge } from '@/components/models/ModelSelectCard'
import { pickLangSegment } from '@/lib/langSegment'
import { useLanguageStore } from '@/stores/languageStore'
import { Cloud, HardDrive } from 'lucide-react'
import type { SummaryApiConfig, TranslationEngine, DownloadableModelInfo } from '@/types'

/** 翻译模型选择器的受控值：当前引擎 + 当前远程翻译模型 id（engine 为 remote 时生效）。 */
export interface TranslateModelValue {
  engine: TranslationEngine
  remoteModel: string
}

export interface TranslateModelPickerProps {
  value: TranslateModelValue
  /**
   * 点卡片即选中：本地/自定义卡片传 { engine }；远程模型卡片传 { engine: 'remote', remoteModel }
   * （同时切引擎并改选模型）。目标语言回退、写后端等副作用由调用方处理。
   */
  onChange: (next: Partial<TranslateModelValue>) => void
  /** 自定义 API 配置保存后递增，触发卡片「已配置」状态与描述刷新 */
  refreshKey?: number
}

/**
 * 翻译模型卡片式选择器（翻译主页弹窗 / 模型设置子页面共用）：
 * 本地模型卡片 + 远程模型卡片全部平铺展示（无下拉）+ 自定义 API 卡片，点选高亮。
 * 模型目录 / 远程开关 / 自定义 API 配置均内部自取；选择值由父组件受控，
 * 避免与页面自身的 useRemoteModelChoice('translate') 实例分叉。
 */
export function TranslateModelPicker({ value, onChange, refreshKey = 0 }: TranslateModelPickerProps) {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const router = useRouter()
  const [translationModels, setTranslationModels] = useState<DownloadableModelInfo[] | null>(null)
  const [remoteEnabled, setRemoteEnabled] = useState(false)
  const [customApiConfigured, setCustomApiConfigured] = useState(false)
  const [customApiConfig, setCustomApiConfig] = useState<SummaryApiConfig | null>(null)

  // 仅用远程模型目录；选择值由父组件受控传入（value.remoteModel）
  const remoteModels = useRemoteModelChoice('translate', { excludeReasoning: true, usage: 'translate' }).models

  useEffect(() => {
    getDownloadableModels()
      .then(setTranslationModels)
      .catch(() => setTranslationModels([]))
    getRemoteEnabled()
      .then(setRemoteEnabled)
      .catch(() => setRemoteEnabled(false))
  }, [])

  // 读回自定义 API 配置：决定卡片是否显示 + 卡片描述（endpoint · model）
  useEffect(() => {
    fetchCustomApiConfigured().then(setCustomApiConfigured)
    summaryGetConfig()
      .then((config) => {
        setCustomApiConfig(
          config && (config.protocol === 'openai' || config.protocol === 'anthropic') ? config : null,
        )
      })
      .catch(() => setCustomApiConfig(null))
  }, [refreshKey])

  const localEngines = translationModels === null ? [] : availableTranslationEngines(translationModels)

  const remoteBadges = (recommended?: boolean): ModelCardBadge[] => [
    { text: t.recBadgeRemote, tone: 'remote' },
    ...(recommended ? [{ text: t.recRecommended.trim(), tone: 'recommended' as const }] : []),
  ]

  const languagesLabel = (languages?: string) =>
    languages?.trim() ? t.recLangSupported.replace('{list}', pickLangSegment(languages, lang)) : undefined

  return (
    <div className="flex flex-col gap-3">
      {/* 本地引擎组 */}
      {localEngines.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
            <HardDrive className="h-3.5 w-3.5" />
            {t.recAsrGroupLocal}
            <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] font-normal">{t.recBadgeLocalFree}</span>
          </span>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {localEngines.includes('opus') && (
              <ModelSelectCard
                title={`OPUS-MT${t.mdLocalSuffix}`}
                description={t.trOpusDesc}
                badges={[{ text: t.recBadgeLocalFree, tone: 'local' }]}
                active={value.engine === 'opus'}
                onClick={() => onChange({ engine: 'opus' })}
              />
            )}
            {localEngines.includes('hymt2') && (
              <ModelSelectCard
                title={`Hy-MT2${t.mdLocalSuffix}`}
                description={t.trHymt2Desc}
                badges={[{ text: t.recBadgeLocalFree, tone: 'local' }]}
                active={value.engine === 'hymt2'}
                onClick={() => onChange({ engine: 'hymt2' })}
              />
            )}
          </div>
        </div>
      )}

      {/* 远程/自定义组（视觉分区 + 高亮） */}
      <div className="flex flex-col gap-2 rounded-lg border border-primary/25 bg-primary/[0.04] p-2.5">
        <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
          <Cloud className="h-3.5 w-3.5 text-primary" />
          {t.recAsrGroupRemote}
          <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] font-normal">{t.recBadgeRemote}</span>
        </span>

        {/* 远程翻译模型：全部平铺为卡片（不用下拉），点选即切换引擎并使用该模型 */}
        {remoteEnabled && remoteModels.length > 0 && (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {remoteModels.map((m) => (
              <ModelSelectCard
                key={m.id}
                title={remoteModelDisplayName(m)}
                badges={remoteBadges(m.recommended)}
                tags={m.tags}
                languages={languagesLabel(m.languages)}
                priceLabel={formatModelPrice(m, t)}
                active={value.engine === 'remote' && value.remoteModel === m.id}
                onClick={() => onChange({ engine: 'remote', remoteModel: m.id })}
              />
            ))}
          </div>
        )}

        {/* 自定义 API 卡片始终显示（2026-09-17 修复「功能找不到」：旧逻辑仅在已配置时渲染，
            未配置用户看不到入口以为功能被删）；未配置时点击跳转「设置 → 自定义 LLM」完成配置 */}
        <ModelSelectCard
          title={t.recEngineCustomApi}
          description={
            customApiConfigured && customApiConfig
              ? `${customApiConfig.endpoint} · ${customApiConfig.model}`
              : t.trCustomApiNotConfigured
          }
          badges={[{ text: t.recEngineCustomApi, tone: 'remote' }]}
          active={value.engine === 'custom-api'}
          onClick={() => {
            if (customApiConfigured && customApiConfig) onChange({ engine: 'custom-api' })
            else router.push('/settings?tab=customApi')
          }}
        />
      </div>
    </div>
  )
}
