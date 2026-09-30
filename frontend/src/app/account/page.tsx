'use client'

import { useCallback, useEffect, useMemo, useState } from 'react'
import { toast } from 'sonner'
import { RefreshCw } from 'lucide-react'
import {
  getRemoteUsage,
  redeemCode,
  getRemoteLedger,
  getRemoteUsageByModel,
  getRemoteUsageTasks,
  getRemoteConfig,
  openExternalUrl,
  type LedgerItem,
  type ModelUsageItem,
  type TaskUsageItem,
} from '@/services/ipc'
import { RemoteServiceSection } from '@/components/settings/RemoteServiceSection'
import { FeedbackSection } from '@/components/settings/FeedbackSection'
import { ClaimCreditsButton } from '@/components/remote/ClaimCreditsButton'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import type { Messages } from '@/i18n/messages'
import { findRemoteModel, useRemoteCatalogStore } from '@/stores/remoteCatalogStore'
import { remoteModelDisplayName } from '@/lib/remoteModelChoice'
import { OFFICIAL_WEBSITE_URL, OFFICIAL_WEBSITE_LABEL, SUPPORT_EMAIL } from '@/lib/site'
import { cn } from '@/lib/utils'

/** 账本流水类型 → i18n 标签（未知类型原样显示，空类型归「其他」） */
function ledgerTypeLabel(type: string, t: Messages): string {
  switch (type) {
    case 'gift':
      return t.accLedgerGift
    case 'redeem':
      return t.accLedgerRedeem
    case 'consume':
      return t.accLedgerConsume
    case 'adjust':
      return t.accLedgerAdjust
    case 'refund':
      return t.accLedgerRefund
    case 'expire':
      return t.accLedgerExpire
    default:
      return type || t.accLedgerOther
  }
}

/** 用量单位 → i18n 标签（token 各语言通用） */
function unitLabel(unit: string, t: Messages): string {
  if (unit === 'seconds') return t.accUnitSeconds
  if (unit === 'chars') return t.accUnitChars
  return unit
}

/** 网关流水时间为 UTC（"YYYY-MM-DD HH:MM:SS" 或 ISO），解析时按 UTC 处理。
 *  供排序、按（设备本地）日期分组、格式化显示复用。 */
function parseLedgerTime(raw: string): Date | null {
  if (!raw) return null
  try {
    const iso = raw.includes('T') ? raw : raw.replace(' ', 'T')
    const hasZone = iso.endsWith('Z') || /[+-]\d{2}:\d{2}$/.test(iso)
    const d = new Date(hasZone ? iso : iso + 'Z')
    return isNaN(d.getTime()) ? null : d
  } catch {
    return null
  }
}

/** 按用户设备本地时区显示。直接展示原始字符串会固定显示 UTC（北京时间慢 8 小时），
 *  全球用户应看到各自本地时间。 */
function formatLedgerTime(raw: string): string {
  const d = parseLedgerTime(raw)
  if (!d) return raw
  return d.toLocaleString(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
}

/** 积分金额：毫积分时代单笔可小至 0.001（自建 TTS 几个字符），
 *  toFixed(2) 会显示成 -0.00，用户会以为没扣费/出 bug。最少 2 位、最多 3 位小数。 */
function formatCredits(n: number): string {
  const abs = Math.abs(n)
  const trimmed = abs.toFixed(3).replace(/0+$/, '')
  const frac = trimmed.split('.')[1] ?? ''
  return frac.length >= 2 ? trimmed : abs.toFixed(2)
}

/** 历史任务名归一化 + 翻译：2026-09-23 之前网关存的是「一次录音」这类说法，
 *  用户在翻译页看到自己写的翻译被标成「一次录音」会误解，现统一为功能名（与 App tab 同名）；
 *  功能名再按界面语言走 i18n（taskName* key），未知任务名原样显示。 */
type TaskKey = 'live' | 'offline' | 'summary' | 'tts' | 'translate' | 'other'
const TASK_KEY_MAP: Record<string, TaskKey> = {
  实时转录: 'live',
  一次录音: 'live',
  离线识别: 'offline',
  一次离线识别: 'offline',
  会议总结: 'summary',
  一次会议总结: 'summary',
  语音合成: 'tts',
  一次语音合成: 'tts',
  翻译: 'translate',
  一次翻译: 'translate',
  其他调用: 'other',
  一次调用: 'other',
}
/** 任务形态 → 消耗行用的图标字符（「其他调用」无图标，与旧版一致）。 */
const TASK_ICON: Partial<Record<TaskKey, string>> = {
  live: '🎙',
  offline: '📄',
  summary: '📝',
  tts: '🔊',
  translate: '🌐',
}
function taskDisplay(task: string, t: Messages): { label: string; icon?: string } {
  const key = TASK_KEY_MAP[task]
  if (!key) return { label: task }
  const label = {
    live: t.taskNameLive,
    offline: t.taskNameOffline,
    summary: t.taskNameSummary,
    tts: t.taskNameTts,
    translate: t.taskNameTranslate,
    other: t.taskNameOther,
  }[key]
  return { label, icon: TASK_ICON[key] }
}

/** 环节名：breakdown.kind 是机器键（asr/translate/summary/tts），按它走 i18n；
 *  未知 kind 回退网关下发的 label 原文（向后兼容新版网关新增的环节）。 */
function stepDisplayLabel(kind: string, fallback: string, t: Messages): string {
  switch (kind) {
    case 'asr':
      return t.stepKindAsr
    case 'translate':
      return t.taskNameTranslate
    case 'summary':
      return t.taskNameSummary
    case 'tts':
      return t.taskNameTts
    default:
      return fallback
  }
}

/** 任务时间列的口径（2026-09-27 修正）：展示任务内**首笔 ~ 末笔**消费的时间跨度
 *  （started_at ~ ended_at），而不是只显示首笔。语音识别费在录音**结束后**才结算，
 *  只显示首笔会让用户误以为「21:31 就被扣了 1220 秒的钱」（实际结算于 21:51）。
 *  两个时间都过 formatLedgerTime（UTC → 本地时区）；同一刻只显示一个；
 *  同一本地自然日内，结尾只留时分秒，避免时间列过长挤压中间内容。 */
function formatTaskTimeRange(start: string, end: string): string {
  const s = formatLedgerTime(start)
  if (!end || end === start) return s
  const ds = parseLedgerTime(start)
  const de = parseLedgerTime(end)
  if (!ds || !de || ds.getTime() === de.getTime()) return s
  const sameDay = localDateKey(ds) === localDateKey(de)
  const eText = sameDay
    ? de.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })
    : formatLedgerTime(end)
  return `${s} ~ ${eText}`
}

/** 任务消耗的一行 = 一个任务的一个环节（平铺展示，不再父子折叠）。
 *  例：一次实时转录会产生两行「实时转录 · 语音识别 …」「实时转录 · 翻译 …」。 */
interface TaskUsageRow {
  key: string
  /** 任务内首笔消费时间（排序、按日期分组用；展示见 timeEnd/formatTaskTimeRange） */
  time: string
  /** 任务内末笔消费时间（结算完成时间；展示用） */
  timeEnd: string
  task: string
  /** 任务图标（🎙/📄/📝/🔊/🌐；未知任务名没有） */
  taskIcon?: string
  stepLabel: string
  quantity: string
  credits: number
  models: string[]
  /** 原始用量数值与单位（seconds/tokens/chars），供按日期聚合时合并同类单位 */
  units: number
  unit: string
}

function flattenTaskUsage(items: TaskUsageItem[], t: Messages): TaskUsageRow[] {
  const rows: TaskUsageRow[] = []
  for (const t0 of items) {
    for (const b of t0.breakdown) {
      // 过滤 0 用量 0 费用的行（2026-09-28）：流式通道预检/预热会在网关留下
      // proc- 任务（0 秒 0 积分），属于连接自检的痕迹而不是消费，显示出来是纯噪音。
      // 注意只滤「双零」：免费/引流价模型（units>0、cost=0）必须保留。
      if (b.units === 0 && b.credits === 0) continue
      const td = taskDisplay(t0.task, t)
      rows.push({
        key: `${t0.task_key}:${b.kind}`,
        time: t0.started_at,
        timeEnd: t0.ended_at,
        task: td.label,
        taskIcon: td.icon,
        stepLabel: stepDisplayLabel(b.kind, b.label, t),
        quantity:
          `${Math.round(b.units)} ${unitLabel(b.unit, t)}` +
          (b.calls > 1 ? ` · ${t.accCallsFmt.replace('{n}', String(b.calls))}` : ''),
        credits: b.credits,
        models: b.models ?? [],
        units: b.units,
        unit: b.unit,
      })
    }
  }
  return rows
}

/** 「积分消耗」卡的三个 tab。 */
type UsageTab = 'tasks' | 'date' | 'model'

const USAGE_PAGE_SIZE: Record<UsageTab, number> = { tasks: 10, date: 5, model: 10 }

/** 合并行 = 任务消耗行 + 账本非 consume 行（赠送/兑换/调整/退款/过期）。
 *  consume 账本条目已由任务行覆盖，不重复显示（避免一笔钱看两遍）。 */
interface MergedUsageRow {
  key: string
  time: string
  /** 任务行的末笔消费时间（ledger 行没有；展示时间跨度用） */
  timeEnd?: string
  sortMs: number
  date: Date | null
  kind: 'task' | 'ledger'
  /** task 行为负数（消耗）；账本行带原始符号 */
  amount: number
  // kind === 'task'
  task?: string
  taskIcon?: string
  stepLabel?: string
  quantity?: string
  units?: number
  unit?: string
  models?: string[]
  // kind === 'ledger'
  ledgerType?: string
  remark?: string
}

function buildMergedUsageRows(taskUsage: TaskUsageItem[], ledger: LedgerItem[], t: Messages): MergedUsageRow[] {
  const rows: MergedUsageRow[] = []
  for (const r of flattenTaskUsage(taskUsage, t)) {
    const d = parseLedgerTime(r.time)
    rows.push({
      key: `task:${r.key}`,
      time: r.time,
      timeEnd: r.timeEnd,
      sortMs: d?.getTime() ?? -1,
      date: d,
      kind: 'task',
      amount: -r.credits,
      task: r.task,
      taskIcon: r.taskIcon,
      stepLabel: r.stepLabel,
      quantity: r.quantity,
      units: r.units,
      unit: r.unit,
      models: r.models,
    })
  }
  for (const l of ledger) {
    if (l.type === 'consume') continue
    const d = parseLedgerTime(l.created_at)
    rows.push({
      key: `ledger:${l.id}`,
      time: l.created_at,
      sortMs: d?.getTime() ?? -1,
      date: d,
      kind: 'ledger',
      amount: l.amount,
      ledgerType: l.type,
      remark: l.remark,
    })
  }
  rows.sort((a, b) => b.sortMs - a.sortMs)
  return rows
}

/** 设备本地日期 key（YYYY-MM-DD），字典序即时间序。 */
function localDateKey(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

/** 按日期视图里，一天内同「功能 + 环节」聚合后的一行。 */
interface DayTaskAgg {
  key: string
  task: string
  taskIcon?: string
  stepLabel: string
  credits: number
  /** 按单位分别汇总后的文本（同单位合并数值，不同单位分列） */
  quantities: string[]
}

interface DayGroup {
  date: string
  /** 当天消耗总额（正数） */
  consume: number
  /** 当天入账总额（正数） */
  income: number
  tasks: DayTaskAgg[]
  /** 正/负入账行，按账本类型聚合 */
  incomes: { key: string; label: string; amount: number }[]
}

function groupUsageByDay(rows: MergedUsageRow[], t: Messages): DayGroup[] {
  interface TaskAcc extends DayTaskAgg {
    unitSums: Map<string, number>
  }
  const days = new Map<
    string,
    { g: DayGroup; taskMap: Map<string, TaskAcc>; incomeMap: Map<string, number> }
  >()
  for (const r of rows) {
    if (!r.date) continue
    const key = localDateKey(r.date)
    let e = days.get(key)
    if (!e) {
      e = { g: { date: key, consume: 0, income: 0, tasks: [], incomes: [] }, taskMap: new Map(), incomeMap: new Map() }
      days.set(key, e)
    }
    if (r.amount < 0) e.g.consume += -r.amount
    else if (r.amount > 0) e.g.income += r.amount
    if (r.kind === 'task') {
      const ak = `${r.task}:${r.stepLabel}`
      let a = e.taskMap.get(ak)
      if (!a) {
        a = { key: ak, task: r.task ?? '', taskIcon: r.taskIcon, stepLabel: r.stepLabel ?? '', credits: 0, quantities: [], unitSums: new Map() }
        e.taskMap.set(ak, a)
      }
      a.credits += -r.amount
      if (r.unit) a.unitSums.set(r.unit, (a.unitSums.get(r.unit) ?? 0) + (r.units ?? 0))
    } else {
      const label = ledgerTypeLabel(r.ledgerType ?? '', t)
      e.incomeMap.set(label, (e.incomeMap.get(label) ?? 0) + r.amount)
    }
  }
  const result: DayGroup[] = []
  for (const { g, taskMap, incomeMap } of days.values()) {
    for (const a of taskMap.values()) {
      a.quantities = [...a.unitSums.entries()].map(([u, v]) =>
        t.accTotalFmt.replace('{n}', String(Math.round(v))).replace('{unit}', unitLabel(u, t))
      )
      g.tasks.push(a)
    }
    for (const [label, amount] of incomeMap) {
      g.incomes.push({ key: label, label, amount })
    }
    result.push(g)
  }
  result.sort((a, b) => (a.date < b.date ? 1 : -1))
  return result
}

/** 用户中心：积分余额（+刷新+预警+官网链接） + 充值兑换 + 积分消耗（任务明细/按日期/按模型）
 *  + 意见反馈（右列第一位） + 远程服务。 */
export default function AccountPage() {
  const t = useMessages()
  const [usage, setUsage] = useState<{ name: string; credits: number; low_balance?: boolean } | null>(null)
  const [license, setLicense] = useState('')
  const [ledger, setLedger] = useState<LedgerItem[]>([])
  const [modelUsage, setModelUsage] = useState<ModelUsageItem[]>([])
  const [taskUsage, setTaskUsage] = useState<TaskUsageItem[]>([])
  const [refreshing, setRefreshing] = useState(false)
  const [redeemInput, setRedeemInput] = useState('')
  const [redeeming, setRedeeming] = useState(false)
  const [refreshKey, setRefreshKey] = useState(0)
  // 授权码配置是否已加载（未加载时不渲染领取按钮，防闪现）
  const [licenseLoaded, setLicenseLoaded] = useState(false)
  // 远程模型目录（全局 store，60s 轮询）：用于把消耗记录里的 model id 映射为显示名
  const catalogModels = useRemoteCatalogStore((s) => s.models)
  // 「积分消耗」卡：tab + 分页（切 tab 或点刷新后页码归 1）
  const [usageTab, setUsageTab] = useState<UsageTab>('tasks')
  const [usagePage, setUsagePage] = useState(1)

  const load = useCallback(() => {
    getRemoteUsage()
      .then((u) => setUsage({ name: u.name, credits: u.credits, low_balance: u.low_balance }))
      .catch(() => setUsage(null))
    getRemoteConfig()
      .then((c) => setLicense(c.license || ''))
      .catch(() => setLicense(''))
      .finally(() => setLicenseLoaded(true))
    getRemoteLedger()
      .then((r) => setLedger(r.items || []))
      .catch(() => setLedger([]))
    getRemoteUsageByModel()
      .then((r) => setModelUsage(r.items || []))
      .catch(() => setModelUsage([]))
    getRemoteUsageTasks()
      .then((r) => setTaskUsage(r.items || []))
      .catch(() => setTaskUsage([]))
  }, [])

  useEffect(() => {
    load()
  }, [load, refreshKey])

  useEffect(() => {
    setUsagePage(1)
  }, [usageTab, refreshKey])

  const handleRefresh = useCallback(() => {
    setRefreshing(true)
    load()
    setRefreshKey((k) => k + 1)
    setTimeout(() => setRefreshing(false), 400)
  }, [load])

  const handleRedeem = useCallback(async () => {
    const code = redeemInput.trim()
    if (!code) {
      toast.error(t.accRedeemPlaceholder)
      return
    }
    setRedeeming(true)
    try {
      const r = await redeemCode(code)
      toast.success(t.accRedeemSuccess.replace('{added}', String(r.added)))
      setRedeemInput('')
      handleRefresh()
    } catch (e) {
      toast.error(String(e))
    } finally {
      setRedeeming(false)
    }
  }, [redeemInput, t, handleRefresh])

  const mergedRows = useMemo(() => buildMergedUsageRows(taskUsage, ledger, t), [taskUsage, ledger, t])
  const dayGroups = useMemo(() => groupUsageByDay(mergedRows, t), [mergedRows, t])

  const usagePageSize = USAGE_PAGE_SIZE[usageTab]
  const usageTotal =
    usageTab === 'tasks' ? mergedRows.length : usageTab === 'date' ? dayGroups.length : modelUsage.length
  const usageTotalPages = Math.max(1, Math.ceil(usageTotal / usagePageSize))
  const page = Math.min(usagePage, usageTotalPages)
  const pageSlice = <T,>(arr: T[]): T[] => arr.slice((page - 1) * usagePageSize, page * usagePageSize)

  const usageTabCls = (tab: UsageTab) =>
    cn(
      'px-1 pb-2 text-xs font-medium border-b-2 -mb-px transition-colors',
      usageTab === tab
        ? 'border-primary text-primary'
        : 'border-transparent text-muted-foreground hover:text-foreground'
    )

  return (
    <div className="h-full overflow-y-auto p-6">
      <h1 className="text-lg font-semibold">{t.accTitle}</h1>

      {/* 醒目提示：遇到问题找谁。用户中心是用户最常来的页面，
          把客服邮箱放在这里，比藏在「意见反馈」卡片里更容易被看到。 */}
      <div className="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 rounded-md border border-primary/30 bg-primary/5 px-3 py-2 text-xs">
        <span className="font-medium text-foreground">{t.accSupportHint}</span>
        <button
          type="button"
          className="font-medium text-primary underline underline-offset-2 hover:opacity-80"
          title={`mailto:${SUPPORT_EMAIL}`}
          onClick={() => {
            // 复制而不是直接 mailto：桌面端没配默认邮件客户端时 mailto 会静默失败，
            // 复制到剪贴板对用户更稳妥（提示里也说了可以手动发邮件）。
            navigator.clipboard
              ?.writeText(SUPPORT_EMAIL)
              .then(() => toast.success(t.accSupportCopied.replace('{email}', SUPPORT_EMAIL)))
              .catch(() => toast.message(SUPPORT_EMAIL))
          }}
        >
          {SUPPORT_EMAIL}
        </button>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
        {/* 左列：积分 + 授权码 + 充值兑换 + 积分消耗（tab 分页） + 使用说明 */}
        <div className="flex flex-col gap-4">
          <div className="rounded-md border border-border/60 p-4">
            <div className="flex items-center justify-between">
              <div className="text-xs text-muted-foreground">{t.accCredits}</div>
              <Button variant="ghost" size="sm" className="h-6 gap-1 px-2 text-xs" disabled={refreshing} onClick={handleRefresh}>
                <RefreshCw className={`h-3 w-3 ${refreshing ? 'animate-spin' : ''}`} />
                {t.accRefresh}
              </Button>
            </div>
            <div className="mt-1 text-2xl font-semibold tabular-nums">
              {usage ? usage.credits : '—'}
            </div>
            {usage?.name && <div className="mt-1 text-xs text-muted-foreground">{usage.name}</div>}
            {usage?.low_balance && (
              <div className="mt-2 rounded bg-red-500/10 px-2 py-1.5 text-xs text-red-600">
                {t.accLowBalance.replace('{credits}', String(usage.credits))}
              </div>
            )}
            {/* 官网 / 发卡站链接（原独立卡片并入余额卡醒目位置）：购买充值卡充值积分 */}
            <button
              type="button"
              className="mt-2 text-xs font-medium text-primary hover:underline"
              title={OFFICIAL_WEBSITE_URL}
              onClick={() => void openExternalUrl(OFFICIAL_WEBSITE_URL).catch(() => {})}
            >
              {t.accWebsite} {OFFICIAL_WEBSITE_LABEL} · {t.accWebsitePromo}
            </button>
            <p className="mt-2 text-[11px] text-muted-foreground/70">{t.accGiftNote}</p>
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="flex items-center justify-between">
              <div className="text-sm font-medium">{t.accLicenseTitle}</div>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-xs"
                onClick={() => {
                  navigator.clipboard?.writeText(license).then(() => toast.success(t.accLicenseCopied))
                }}
              >
                {t.comCopy}
              </Button>
            </div>
            <div className="mt-1 break-all text-xs text-muted-foreground">{license || t.comNotSet}</div>
            <p className="mt-1 text-xs text-muted-foreground/70">{t.accLicenseBackupHint}</p>
          </div>

          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">{t.accRedeemTitle}</div>
            <div className="mt-2 flex items-center gap-2">
              <input
                className="h-8 flex-1 rounded-md border border-input bg-background px-2 text-sm"
                value={redeemInput}
                onChange={(e) => setRedeemInput(e.target.value)}
                placeholder={t.accRedeemPlaceholder}
                onKeyDown={(e) => e.key === 'Enter' && handleRedeem()}
              />
              <Button size="sm" disabled={redeeming} onClick={handleRedeem}>
                {t.accRedeemBtn}
              </Button>
            </div>
            {/* 无授权码时的醒目领取入口（已有授权码则不渲染；成功态显示授权码 + 保存提示） */}
            <div className="mt-2">
              <ClaimCreditsButton
                hasLicense={licenseLoaded ? !!license : null}
                onClaimed={handleRefresh}
              />
            </div>
          </div>

          {/* 积分消耗：三 tab（任务明细 = 任务消耗 + 账本非 consume 流水合并 / 按日期 / 按模型），
              固定高度 + 底部分页，记录再多也不撑长页面。 */}
          <div className="rounded-md border border-border/60 p-4">
            <div className="text-sm font-medium">{t.accUsageTitle}</div>
            <div className="mt-2 flex gap-4 border-b border-border/60">
              <button className={usageTabCls('tasks')} onClick={() => setUsageTab('tasks')}>
                {t.accUsageTabTasks}
              </button>
              <button className={usageTabCls('date')} onClick={() => setUsageTab('date')}>
                {t.accUsageTabDate}
              </button>
              <button className={usageTabCls('model')} onClick={() => setUsageTab('model')}>
                {t.accUsageTabModel}
              </button>
            </div>

            <div className="mt-2 h-[360px] overflow-hidden">
              {usageTab === 'tasks' &&
                (mergedRows.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.accUsageEmpty}</p>
                ) : (
                  <div className="flex flex-col gap-1">
                    {pageSlice(mergedRows).map((r) => (
                      <div key={r.key} className="flex items-center justify-between gap-2 text-xs">
                        {/* 任务行显示时间跨度（首笔~末笔消费，末笔才是结算完成时间）；
                            账本行是单条流水，显示单点时间 */}
                        {r.kind === 'task' && r.timeEnd ? (
                          <span
                            className="shrink-0 tabular-nums text-muted-foreground"
                            title={t.accTaskTimeSpanHint}
                          >
                            {formatTaskTimeRange(r.time, r.timeEnd)}
                          </span>
                        ) : (
                          <span className="shrink-0 tabular-nums text-muted-foreground">{formatLedgerTime(r.time)}</span>
                        )}
                        {r.kind === 'task' ? (
                          <>
                            <span
                              className="min-w-0 flex-1 truncate px-2 text-muted-foreground"
                              title={r.models?.length ? t.accModelsTooltip.replace('{list}', r.models.join(', ')) : undefined}
                            >
                              {r.taskIcon ? `${r.taskIcon} ` : ''}
                              {r.task}
                              <span className="text-muted-foreground/70"> · {r.stepLabel} · {r.quantity}</span>
                            </span>
                            <span className="shrink-0 text-red-500">-{formatCredits(r.amount)} {t.accCreditsUnit}</span>
                          </>
                        ) : (
                          <>
                            <span
                              className="min-w-0 flex-1 truncate px-2 text-muted-foreground"
                              title={r.remark || undefined}
                            >
                              {ledgerTypeLabel(r.ledgerType ?? '', t)}
                              {r.remark ? ` · ${r.remark}` : ''}
                            </span>
                            <span className={`shrink-0 ${r.amount >= 0 ? 'text-green-600' : 'text-red-500'}`}>
                              {r.amount >= 0 ? '+' : ''}
                              {r.amount}
                            </span>
                          </>
                        )}
                      </div>
                    ))}
                  </div>
                ))}

              {usageTab === 'date' &&
                (dayGroups.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.accUsageEmpty}</p>
                ) : (
                  <div className="flex flex-col gap-2">
                    {pageSlice(dayGroups).map((g) => (
                      <div key={g.date} className="flex flex-col gap-1">
                        <div className="flex items-center gap-1 text-xs">
                          <span className="font-medium">{g.date}</span>
                          {g.consume > 0 && (
                            <span className="text-muted-foreground">
                              · {t.accConsumeLabel} <span className="text-red-500">-{formatCredits(g.consume)} {t.accCreditsUnit}</span>
                            </span>
                          )}
                          {g.income > 0 && (
                            <span className="text-muted-foreground">
                              · {t.accIncomeLabel} <span className="text-green-600">+{formatCredits(g.income)} {t.accCreditsUnit}</span>
                            </span>
                          )}
                        </div>
                        {g.tasks.map((a) => (
                          <div key={a.key} className="flex items-center justify-between gap-2 pl-4 text-xs">
                            <span className="min-w-0 flex-1 truncate text-muted-foreground">
                              {a.taskIcon ? `${a.taskIcon} ` : ''}
                              {a.task}
                              <span className="text-muted-foreground/70">
                                {' '}
                                · {a.stepLabel}
                                {a.quantities.length ? ` · ${a.quantities.join(' · ')}` : ''}
                              </span>
                            </span>
                            <span className="shrink-0 text-red-500">-{formatCredits(a.credits)} {t.accCreditsUnit}</span>
                          </div>
                        ))}
                        {g.incomes.map((i) => (
                          <div key={i.key} className="flex items-center justify-between gap-2 pl-4 text-xs">
                            <span className="min-w-0 flex-1 truncate text-muted-foreground">{i.label}</span>
                            <span className={`shrink-0 ${i.amount >= 0 ? 'text-green-600' : 'text-red-500'}`}>
                              {i.amount >= 0 ? '+' : ''}
                              {i.amount} {t.accCreditsUnit}
                            </span>
                          </div>
                        ))}
                      </div>
                    ))}
                  </div>
                ))}

              {usageTab === 'model' &&
                (modelUsage.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.accUsageEmpty}</p>
                ) : (
                  <div className="flex flex-col gap-1">
                    {pageSlice(modelUsage).map((m) => {
                      const rm = findRemoteModel(catalogModels, m.model)
                      return (
                        <div key={m.model} className="flex items-center justify-between text-xs">
                          <span className="text-muted-foreground">
                            {rm ? remoteModelDisplayName(rm) : m.model}
                            <span className="text-muted-foreground/60"> · {m.kind === 'asr' ? t.accModelKindAsr : m.kind === 'tts' ? t.accModelKindTts : t.accModelKindTranslate} · {t.accCallsFmt.replace('{n}', String(m.calls))}</span>
                          </span>
                          <span className="text-red-500">-{formatCredits(m.credits)} {t.accCreditsUnit}</span>
                        </div>
                      )
                    })}
                  </div>
                ))}
            </div>

            {usageTotal > 0 && (
              <div className="mt-2 flex items-center justify-between border-t border-border/60 pt-2">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 px-2 text-xs"
                  disabled={page <= 1}
                  onClick={() => setUsagePage(page - 1)}
                >
                  {t.comPrev}
                </Button>
                <span className="text-xs text-muted-foreground">
                  {page}/{usageTotalPages}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 px-2 text-xs"
                  disabled={page >= usageTotalPages}
                  onClick={() => setUsagePage(page + 1)}
                >
                  {t.comNext}
                </Button>
              </div>
            )}

            {/* 积分使用说明（压缩为一行小字，不再单独成卡） */}
            <p className="mt-2 border-t border-border/60 pt-2 text-[11px] text-muted-foreground/80">
              {t.accBillingHint}
            </p>
          </div>
        </div>

        {/* 右列：意见反馈（第一位，用户遇到问题先看到入口） + 远程服务 */}
        <div className="flex flex-col gap-4">
          <FeedbackSection />
          <RemoteServiceSection key={`remote-${refreshKey}`} onChanged={handleRefresh} />
        </div>
      </div>
    </div>
  )
}
