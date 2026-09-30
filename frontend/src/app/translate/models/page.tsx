'use client'

import { useCallback, useEffect, useMemo, useState } from 'react'
import Link from 'next/link'
import { ArrowLeft } from 'lucide-react'
import {
  getRemoteEnabled,
  getTranslationEngine,
  setTranslationEngine as ipcSetTranslationEngine,
  setTranslationTargetLang as ipcSetTranslationTargetLang,
} from '@/services/ipc'
import { useTranslatePageStore } from '@/stores/translatePageStore'
import { useLanguageStore } from '@/stores/languageStore'
import {
  useTtsDefaultVoiceStore,
  getLangName,
  defaultConfigForLang,
} from '@/stores/ttsDefaultVoiceStore'
import { useMessages } from '@/i18n/useMessages'
import { defaultTargetLang, getTranslateTargetLangs } from '@/lib/translateTargetLangs'
import {
  formatModelPrice,
  remoteModelDisplayName,
  remoteModelOptionLabel,
  useRemoteModelChoice,
} from '@/lib/remoteModelChoice'
import { ModelSelectCard, type ModelCardBadge } from '@/components/models/ModelSelectCard'
import { TranslateModelPicker } from '@/components/translate/TranslateModelPicker'
import { pickLangSegment } from '@/lib/langSegment'
import { Button } from '@/components/ui/button'
import type { TranslationEngine } from '@/types'

/** 翻译「模型与设置」子页面：卡片式选择翻译模型 + 自定义 API 配置引导（表单在设置页）+ TTS 模型与默认音色。 */
export default function TranslateModelsPage() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const setTargetLang = useTranslatePageStore((s) => s.setTargetLang)
  // 目标语言：默认音色是按语种存的，展示时要按它取
  const targetLang = useTranslatePageStore((s) => s.targetLang)

  const [engine, setEngine] = useState<TranslationEngine>('opus')
  const [remoteEnabled, setRemoteEnabled] = useState(false)

  // 远程翻译 / TTS 模型选择（与后端远程模型同步，目录按推荐置顶）
  const remoteTranslate = useRemoteModelChoice('translate', { excludeReasoning: true, usage: 'translate' })
  const remoteTts = useRemoteModelChoice('tts')
  // 语音播报的默认音色（2026-09-23 起**按语种**存，见 ttsDefaultVoiceStore）。
  // 这里展示「当前翻译目标语言」实际会用的音色，与翻译页播放译文的行为一致——
  // 以前展示的是"当前选中模型"的音色，跟真正会用到的那条链路对不上。
  //
  // ⚠️ 2026-09-26 白屏事故：这里以前写的是
  //     `useTtsDefaultVoiceStore((s) => s.resolve(targetLang, remoteTts.models))`
  //   `resolve()` 每次调用都返回**新对象**（`defaultConfigForLang` 造字面量），
  //   而 zustand v5 内部直接用 React 的 `useSyncExternalStore`——它按引用比较快照，
  //   于是每次 getSnapshot 都"变了" → 无限重渲染 → React 抛
  //   "Maximum update depth exceeded" → 没有 error boundary，整棵树卸载 → **整个窗口白屏**，
  //   侧边栏一起消失，用户回不到任何页面（点翻译页模型选择器右侧的齿轮必现）。
  //   修法：selector 只取**稳定引用**（byLang 记录），派生对象交给 useMemo。
  //   同理：不要在任何 zustand selector 里调用会构造新对象/新数组的方法。
  const byLang = useTtsDefaultVoiceStore((s) => s.byLang)
  const ttsDefault = useMemo(
    () => byLang[targetLang] ?? defaultConfigForLang(targetLang, remoteTts.models),
    [byLang, targetLang, remoteTts.models],
  )
  const ttsVoiceCustom = !!byLang[targetLang]

  useEffect(() => {
    getRemoteEnabled()
      .then(setRemoteEnabled)
      .catch(() => setRemoteEnabled(false))
    getTranslationEngine()
      .then(setEngine)
      .catch(() => {})
  }, [])

  // 与翻译页一致的引擎切换：写后端 + 本地 state；目标语言不再可用时回退默认目标
  const handleEngineChange = useCallback(
    (next: TranslationEngine) => {
      setEngine((prev) => {
        ipcSetTranslationEngine(next).catch(() => setEngine(prev))
        return next
      })
      const home = useLanguageStore.getState().language
      if (!getTranslateTargetLangs(next).includes(useTranslatePageStore.getState().targetLang)) {
        const fallback = defaultTargetLang(home)
        setTargetLang(fallback)
        ipcSetTranslationTargetLang(fallback).catch(() => {})
      }
    },
    [setTargetLang],
  )

  const selectedTtsModel = remoteTts.models.find((m) => m.id === remoteTts.value)

  const remoteBadges = (recommended?: boolean): ModelCardBadge[] => [
    { text: t.recBadgeRemote, tone: 'remote' },
    ...(recommended ? [{ text: t.recRecommended.trim(), tone: 'recommended' as const }] : []),
  ]

  const languagesLabel = (languages?: string) =>
    languages?.trim() ? t.recLangSupported.replace('{list}', pickLangSegment(languages, lang)) : undefined

  return (
    <div className="h-full flex flex-col bg-background px-5 pt-8 pb-5 gap-5 overflow-y-auto custom-scrollbar">
      {/* 页头：返回翻译 + 标题 */}
      <header className="shrink-0 flex items-center gap-3">
        <Link
          href="/translate"
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          {t.trBackToTranslate}
        </Link>
        <h1 className="text-xl font-semibold">{t.trModelSettings}</h1>
      </header>

      {/* 翻译模型区（与主页模型选择弹窗共用 TranslateModelPicker） */}
      <section className="shrink-0 flex flex-col gap-2">
        <h2 className="text-sm font-medium">{t.trTranslateModelsTitle}</h2>
        <TranslateModelPicker
          value={{ engine, remoteModel: remoteTranslate.value }}
          onChange={(next) => {
            if (next.engine) handleEngineChange(next.engine)
            if (next.remoteModel) remoteTranslate.set(next.remoteModel)
          }}
        />
      </section>

      {/* 自定义 API 配置区：表单已移到「设置 → 自定义 LLM」，这里只保留引导 */}
      <section className="shrink-0 flex flex-col gap-2 max-w-2xl">
        <h2 className="text-sm font-medium">{t.trCustomApiSection}</h2>
        <div className="flex items-center justify-between gap-2 rounded-md border border-primary/20 bg-primary/5 px-4 py-3">
          <span className="text-sm">{t.trCustomApiGuide}</span>
          <Button variant="outline" size="sm" className="shrink-0" asChild>
            <Link href="/settings?tab=customApi">{t.sumGoSettings}</Link>
          </Button>
        </div>
      </section>

      {/* 语音合成（TTS）区 */}
      {remoteEnabled && remoteTts.models.length > 0 && (
        <section className="shrink-0 flex flex-col gap-2">
          <h2 className="text-sm font-medium">{t.trTtsSection}</h2>
          <select
            className="h-8 w-full sm:max-w-[360px] rounded-md border border-input bg-background px-2 text-xs shadow-sm focus:outline-none"
            value={remoteTts.value}
            onChange={(e) => remoteTts.set(e.target.value)}
            title={`${t.trRemoteModel} (TTS)`}
          >
            {remoteTts.models.map((m) => (
              <option key={m.id} value={m.id}>
                {remoteModelOptionLabel(m, t)}
              </option>
            ))}
          </select>
          {selectedTtsModel && (
            <ModelSelectCard
              title={remoteModelDisplayName(selectedTtsModel)}
              badges={remoteBadges(selectedTtsModel.recommended)}
              tags={selectedTtsModel.tags}
              languages={languagesLabel(selectedTtsModel.languages)}
              priceLabel={formatModelPrice(selectedTtsModel, t)}
              active={false}
            />
          )}
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <span>
              {t.trDefaultVoice}（{getLangName(targetLang)}）：
              {ttsDefault.voice?.trim() || t.trVoiceProviderDefault}

            </span>
            <Link href="/tts" className="text-primary hover:underline">
              {t.trGotoTtsPage}
            </Link>
          </div>
        </section>
      )}
    </div>
  )
}
