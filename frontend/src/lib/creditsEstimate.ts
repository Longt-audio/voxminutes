/**
 * 「按积分余额估算还能录多久」（2026-09-29）。
 *
 * 背景：远程流式 ASR **按收到的音频秒数计费**（静音/音乐照扣，= 录音墙钟时长），
 * 余额用尽时网关会下发 `error: 积分不足，请充值` 并主动断开——识别停止、录音继续。
 * 用户此前完全无感（只有一条 5 秒 toast），因此录音前给出一个保守预估。
 *
 * ⚠️ 必须复刻网关的冻结规则（`gateway/src/routes/realtime.ts::SessionBilling`）：
 *   · 建会话时先冻结 **2 分钟**额度，余额不够连会话都建不起来（连接即被拒）；
 *   · 之后按 **最小 60 秒**粒度阶梯补冻。
 * 所以可用时长 = `60 × floor(B / (60c))` 秒，且 `B < 120c` 时为 0。
 * 直接用 `B / c` 会高估，并在余额不足 2 分钟时给出「还能录 90 秒」这种假答案。
 */
import { getRemoteEnabled, getRemoteUsage, type RemoteModelItem } from '@/services/ipc'

/** 余额缓存 TTL：`getRemoteUsage()` 每次都会实打网关 `/v1/usage`，弹窗反复开合不该反复请求。 */
const BALANCE_TTL_MS = 60_000

let balanceCache: { at: number; credits: number } | null = null

/** 取余额（积分），带 60s 缓存。远程未启用 / 未配置授权码 / 请求失败 → null（调用方静默不显示）。 */
export async function getCachedCredits(force = false): Promise<number | null> {
  const now = Date.now()
  if (!force && balanceCache && now - balanceCache.at < BALANCE_TTL_MS) {
    return balanceCache.credits
  }
  try {
    const enabled = await getRemoteEnabled()
    if (!enabled) return null
    const usage = await getRemoteUsage()
    const credits = Number(usage?.credits)
    if (!Number.isFinite(credits)) return null
    balanceCache = { at: now, credits }
    return credits
  } catch {
    return null
  }
}

/** 供录音结束/充值后主动失效（下一次读取会重新拉取）。 */
export function invalidateCreditsCache() {
  balanceCache = null
}

/** 模型的「积分/秒」单价；非按秒计费（理论不会出现在 ASR 上）返回 null。 */
export function creditsPerSecond(model: Pick<RemoteModelItem, 'price' | 'price_unit'>): number | null {
  const price = Number(model?.price ?? 0)
  if (!Number.isFinite(price) || price <= 0) return null
  if (model?.price_unit && model.price_unit !== 'second') return null
  return price
}

/** 建会话预冻的秒数（网关固定 2 分钟）。 */
export const GATEWAY_FREEZE_SECONDS = 120
/** 补冻粒度（网关固定 60 秒）。 */
export const GATEWAY_FREEZE_STEP_SECONDS = 60

/**
 * 预估可录秒数（保守值，向下取整到网关的补冻粒度）。
 * 返回 0 表示**连会话都建不起来**（余额不足 2 分钟额度），不是「能录 0 秒」。
 */
export function estimateRecordableSeconds(balanceCredits: number, pricePerSecond: number): number {
  if (!Number.isFinite(balanceCredits) || !Number.isFinite(pricePerSecond) || pricePerSecond <= 0) {
    return 0
  }
  if (balanceCredits < GATEWAY_FREEZE_SECONDS * pricePerSecond) return 0
  return GATEWAY_FREEZE_STEP_SECONDS * Math.floor(balanceCredits / (GATEWAY_FREEZE_STEP_SECONDS * pricePerSecond))
}

/** 「至少需要多少积分才能开始」= 2 分钟单价（展示用，保留 2 位小数）。 */
export function minimumCreditsToStart(pricePerSecond: number): number {
  return Number((GATEWAY_FREEZE_SECONDS * pricePerSecond).toFixed(2))
}

export interface DurationLabels {
  recDurHourMin: string
  recDurMin: string
  recDurSec: string
}

/** 把秒数格式化成「1 小时 30 分钟 / 25 分钟 / 40 秒」（向下取整到分钟，越保守越好）。 */
export function formatRecordableDuration(seconds: number, t: DurationLabels): string {
  if (seconds <= 0) return t.recDurSec.replace('{s}', '0')
  if (seconds < 120) return t.recDurSec.replace('{s}', String(Math.floor(seconds)))
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return t.recDurMin.replace('{m}', String(minutes))
  const h = Math.floor(minutes / 60)
  const m = minutes % 60
  return t.recDurHourMin.replace('{h}', String(h)).replace('{m}', String(m))
}
