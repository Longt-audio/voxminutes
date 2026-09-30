'use client'

import { useEffect, useMemo, useRef } from 'react'
import type { RemoteModelItem } from '@/services/ipc'
import { catalogByKind, useRemoteCatalogStore } from '@/stores/remoteCatalogStore'
import {
  useRemoteModelChoiceStore,
  type RemoteChoiceKind,
} from '@/stores/remoteModelChoiceStore'
import type { Messages } from '@/i18n/messages'

/** 远程模型选择（在各功能使用处选择，用户中心只读展示）。
 *  模型列表来自全局 remoteCatalogStore（60s 轮询，后台改动实时生效），
 *  当前选择来自全局 remoteModelChoiceStore（所有 hook 实例共享同一份值，
 *  任一使用处改动其余处立即同步——见 store 注释）；返回 [列表, 当前值, 设置函数]。
 *
 *  ASR 拆成两个独立选择（2026-09-17 起的产品约定）：
 *  - kind='asr'         → 实时转录用，只列流式（mode=streaming）模型
 *  - kind='asr_offline' → 历史记录离线重识别用，只列非流式（batch）模型
 *
 *  `usage`（2026-09-23）：同一批 LLM 对「逐句翻译」与「会议总结」的适用性不同，由网关
 *  后台配置并随目录下发。翻译处传 'translate'（排除 deepseek-flash 这类只做总结的），
 *  会议总结处传 'summary'（排除豆包机器翻译这类只做翻译的）；不传 = 不过滤。
 *  过滤结果**也参与下面的「失配自动纠正」**，所以存量用户若选了不可用模型会自动切到
 *  第一个可用模型并持久化 —— 否则网关会直接 400（它不静默换模型）。 */

/** 该模型能否用于指定用途。与网关 modelWire.ts 的 usageAllows 同一口径。
 *  缺字段（旧版网关不下发 usage）按 'both'，保证向后兼容。 */
export function modelUsageAllows(
  m: Pick<RemoteModelItem, 'usage'>,
  want: 'translate' | 'summary'
): boolean {
  const allow = m.usage ?? 'both'
  return allow === 'both' || allow === want
}

export function useRemoteModelChoice(
  kind: RemoteChoiceKind,
  opts?: {
    /** 排除推理模型（实时翻译场景用）。会议总结不要排除——那里预算充足、不要求实时。 */
    excludeReasoning?: boolean
    /** 按 LLM 用途过滤：'translate' 只列能做翻译的，'summary' 只列能做总结的。 */
    usage?: 'translate' | 'summary'
  }
) {
  const allModels = useRemoteCatalogStore((s) => s.models)
  const catalogLoading = useRemoteCatalogStore((s) => s.loading)
  const refresh = useRemoteCatalogStore((s) => s.refresh)
  const value = useRemoteModelChoiceStore((s) => s.values[kind])
  const loaded = useRemoteModelChoiceStore((s) => s.loaded)
  const load = useRemoteModelChoiceStore((s) => s.load)
  const persist = useRemoteModelChoiceStore((s) => s.persist)
  // asr_offline 自动纠正只做一次（见下方 effect）
  const autoFixedRef = useRef(false)

  // 挂载时确保目录刷新一次（非 force，updatedAt 未变则不触发重渲染）
  useEffect(() => {
    void refresh()
  }, [refresh])

  // 挂载时确保后端选择已读回（幂等：全局只读一次）
  useEffect(() => {
    load()
  }, [load])

  // 按 kind 过滤 + 推荐排序（recommended 在前，其次显示名字典序）；
  // ASR 再按用途过滤流式/非流式，LLM 再按 usage 过滤翻译/总结
  const excludeReasoning = opts?.excludeReasoning === true
  const wantUsage = opts?.usage
  const models = useMemo(() => {
    // usage 只对 LLM 有意义；'summary' 取的是同一批 LLM（网关 kind 对 LLM 恒为 translate，
    // 因为 kind 表达的是**能力/端点**、usage 表达的是**用途**），差异全部由下面的 usage 过滤体现。
    const base = catalogByKind(allModels, kind === 'asr_offline' ? 'asr' : kind === 'summary' ? 'translate' : kind)
    if (kind === 'asr') return base.filter((m) => m.mode === 'streaming')
    if (kind === 'asr_offline') return base.filter((m) => m.mode !== 'streaming')
    // LLM 用途过滤（见 hook 头部注释）。缺字段按 both，旧版网关不受影响。
    // 注意：kind='translate' 同时服务「逐句翻译」和「会议总结」两个界面（两者共用同一份
    // 持久化选择），所以过滤**必须由调用处显式声明用途**，不能在这里按 kind 猜。
    const byUsage = wantUsage ? base.filter((m) => modelUsageAllows(m, wantUsage)) : base
    // 实时翻译：排除推理模型（见 RemoteModelItem.reasoning 注释）。过滤后的列表
    // 也参与下面的「失配自动纠正」，所以存量用户选了推理模型会自动切走并持久化。
    if (excludeReasoning) return byUsage.filter((m) => !m.reasoning)
    return byUsage
  }, [allModels, kind, excludeReasoning, wantUsage])
  // 当前值必须落在可用列表里，否则取第一个
  const effectiveValue = models.some((m) => m.id === value) ? value : models[0]?.id || ''

  const set = (v: string) => {
    // ASR 需要把网关下发的 mode 一并透传，后端据此决定流式/非流式（不再依赖模型名后缀）
    const m = models.find((mm) => mm.id === v)
    persist(kind, v, m?.mode).catch(() => {})
  }

  // 兜底自动纠正：后端读回的选择可能已不在当前可用列表里（模型被下架 / 供应商 key 被移除 /
  // asr_offline 旧版共用字段带回流式模型）。列表非空时自动改选第一个可用模型并**持久化**。
  //
  // ⚠️ 必须持久化，不能只修 UI：Rust 侧 get_remote_tts_model() / get_remote_asr_model() 用的是
  // 后端存的值；调用方不传 model 时（如翻译页「播放译文」）仍会拿旧值去请求。而网关自
  // 2026-09-22 起对「已下架模型」**明确报 400**（不再静默回落成别的模型）→ 不纠正就直接失败。
  // 典型场景：TTS 只保留 mimo 后，老用户存的 qwen-audio-3.0-tts-flash 已下架。
  // 每个实例只自动纠正一次（持久化持续失败时不再反复重试，避免 IPC 死循环）。
  useEffect(() => {
    if (!loaded || models.length === 0) return
    if (models.some((m) => m.id === value)) {
      autoFixedRef.current = false
      return
    }
    if (autoFixedRef.current) return
    autoFixedRef.current = true
    persist(kind, models[0].id, models[0].mode).catch(() => {})
  }, [kind, loaded, models, value, persist])

  const loading = !loaded || (catalogLoading && models.length === 0)
  return { models, value: effectiveValue, set, loading }
}

/** 远程模型显示名：display_name（去空白）非空则用之，否则回退 id */
export function remoteModelDisplayName(m: RemoteModelItem): string {
  return m.display_name?.trim() || m.id
}

/** select option 文本：显示名 + （流式）? + 价格 + 推荐标记 */
export function remoteModelOptionLabel(m: RemoteModelItem, t: Messages): string {
  const streaming = m.mode === 'streaming' ? `（${t.accModeStreaming}）` : ''
  const rec = m.recommended ? t.recRecommended : ''
  return `${remoteModelDisplayName(m)}${streaming} · ${formatModelPrice(m, t)}${rec}`
}

/** 格式化模型单价为可读积分消耗文案：ASR 显示「积分/分钟」，LLM「积分/千token」，TTS「积分/千字符」
 *  （单位文案走 i18n：priceFree / pricePerMinute / pricePerKTokens / pricePerKChars）。
 *  注意：DB 里 ASR price 仍是「积分/秒」，这里仅做展示换算（×60）。 */
export function formatModelPrice(m: RemoteModelItem, t: Messages): string {
  const price = m.price ?? 0
  if (price <= 0) return t.priceFree
  if (m.price_unit === 'second') {
    const perMin = price * 60
    const shown = perMin < 0.01 ? perMin.toFixed(4) : perMin.toFixed(2)
    return t.pricePerMinute.replace('{price}', shown)
  }
  if (m.price_unit === 'char') return t.pricePerKChars.replace('{price}', (price * 1000).toFixed(2))
  return t.pricePerKTokens.replace('{price}', (price * 1000).toFixed(2))
}
