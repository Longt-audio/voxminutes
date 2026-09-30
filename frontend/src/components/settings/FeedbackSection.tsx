'use client'

import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import {
  submitFeedback,
  collectDiagLogRange,
  collectManualLogs,
  getLogDir,
  openLogFolder,
  type DiagLogRange,
} from '@/services/ipc'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 日志附加范围下拉框的取值。'day' 走 24 小时时间窗，其余按文件数。 */
type LogRangeKey = '1' | '2' | '5' | 'day'

function toDiagRange(key: LogRangeKey): DiagLogRange {
  return key === 'day' ? { sinceHours: 24 } : { maxFiles: Number(key) }
}

/** 取路径末段做展示（同时兼容 macOS/Linux 的 / 与 Windows 的 \）。 */
function baseName(p: string): string {
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p
}

/**
 * 意见反馈（用户中心右侧栏卡片）：内容 + 截图 + 联系方式 + 诊断日志 + 提交。
 *
 * 诊断日志（2026-09-25 起默认自动附带；2026-09-28 可选范围 + 支持手动附加日志文件）：
 * 软件发布后我们拿不到用户机器上的日志文件，用户口述的「识别不了了」又不带关键信息
 * （关闭码、重连次数、看门狗判定）。所以组件挂载时就由 Rust 侧收集最近日志尾部并脱敏，
 * 默认勾选「已自动附加」；用户不想传可以取消勾选，也可以用下拉框改附加范围
 * （最近 1/2/5 次运行、最近一天），或通过「附加日志文件…」手动挑文件（对话框默认
 * 定位到应用日志目录，多选 .log，同样脱敏后并入上传）。
 */
export function FeedbackSection() {
  const t = useMessages()
  const [feedback, setFeedback] = useState('')
  const [screenshot, setScreenshot] = useState<string | null>(null)
  const [contact, setContact] = useState('')
  const [attachLog, setAttachLog] = useState(true)
  const [logRange, setLogRange] = useState<LogRangeKey>('2')
  // 预收集的诊断日志（收集失败为 null，提交时退化为不附加，绝不挡住反馈）；随范围变化重新收集
  const [diagLog, setDiagLog] = useState<string | null>(null)
  // 用户手动挑选的日志文件绝对路径（提交时由 Rust 侧读内容、脱敏后并入 diag_log）
  const [manualLogPaths, setManualLogPaths] = useState<string[]>([])
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    collectDiagLogRange(toDiagRange(logRange))
      .then(setDiagLog)
      .catch(() => setDiagLog(null))
  }, [logRange])

  const handleScreenshot = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => setScreenshot(reader.result as string)
    reader.readAsDataURL(file)
  }

  /** 「附加日志文件…」：文件对话框默认定位到应用日志目录（Rust 侧定位，macOS/Windows/dev 通用），多选 .log。 */
  const handlePickLogs = useCallback(async () => {
    try {
      const logDir = await getLogDir().catch(() => null)
      const picked = await openDialog({
        multiple: true,
        defaultPath: logDir ?? undefined,
        filters: [{ name: 'Log', extensions: ['log'] }],
      })
      if (!picked) return
      const paths = (Array.isArray(picked) ? picked : [picked]).filter(
        (p): p is string => typeof p === 'string' && !!p
      )
      if (!paths.length) return
      setManualLogPaths((prev) => [...prev, ...paths.filter((p) => !prev.includes(p))])
    } catch (e) {
      toast.error(String(e))
    }
  }, [])

  const handleSubmit = async () => {
    if (!feedback.trim()) {
      toast.error(t.accFeedbackEmpty)
      return
    }
    setSubmitting(true)
    try {
      // 自动日志：优先用预收集的；还没收集好（或失败）且用户勾选了，就现场再试一次，
      // 仍拿不到就当没勾选（收集函数本身也只在极端情况下抛）
      const parts: string[] = []
      if (attachLog) {
        const auto = diagLog ?? (await collectDiagLogRange(toDiagRange(logRange)).catch(() => null))
        if (auto) parts.push(auto)
      }
      // 手动附加的日志文件：Rust 侧读取 + 脱敏，头部标注「用户手动附加」
      if (manualLogPaths.length) {
        const manual = await collectManualLogs(manualLogPaths).catch(() => null)
        if (manual) parts.push(manual)
      }
      const log = parts.length ? parts.join('\n') : null
      await submitFeedback(feedback.trim(), screenshot, contact.trim() || null, log)
      toast.success(t.accFeedbackSent)
      setFeedback('')
      setScreenshot(null)
      setContact('')
      setManualLogPaths([])
    } catch (e) {
      toast.error(t.accFeedbackFailed.replace('{error}', String(e)))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="rounded-md border border-border/60 p-4">
      <div className="text-sm font-medium">{t.accFeedback}</div>
      <textarea
        className="mt-2 h-24 w-full rounded-md border border-input bg-background p-2 text-sm"
        value={feedback}
        onChange={(e) => setFeedback(e.target.value)}
        placeholder={t.accFeedbackPlaceholder}
      />
      {/* 截图附件：标注清楚是「附加截图」，避免与下方的日志文件混淆 */}
      <div className="mt-2 flex items-center gap-2 text-xs">
        <span className="shrink-0 text-muted-foreground">{t.accScreenshotAttach}:</span>
        <input type="file" accept="image/*" onChange={handleScreenshot} />
        {screenshot && <span className="text-muted-foreground">{t.accScreenshotAttached}</span>}
      </div>
      {/* 手动附加日志文件：对话框默认定位到应用日志目录，多选 .log，可逐个移除 */}
      <div className="mt-2 flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" className="h-7 w-fit px-2 text-xs" onClick={handlePickLogs}>
            {t.accAttachLogs}
          </Button>
          {/* 一键打开日志文件夹：用户报障时不用自己去找 %LOCALAPPDATA%\VoxMinutes\logs */}
          <Button
            variant="outline"
            size="sm"
            className="h-7 w-fit px-2 text-xs"
            onClick={() => {
              openLogFolder()
                .then((p) => toast.success(p))
                .catch((e) => toast.error(String(e)))
            }}
          >
            {t.accOpenLogFolder}
          </Button>
        </div>
        {manualLogPaths.length > 0 && (
          <div className="flex flex-col gap-0.5">
            {manualLogPaths.map((p) => (
              <div key={p} className="flex items-center gap-1 text-xs text-muted-foreground">
                <span className="min-w-0 flex-1 truncate" title={p}>
                  {baseName(p)}
                </span>
                <button
                  type="button"
                  className="shrink-0 px-1 text-muted-foreground/70 hover:text-foreground"
                  onClick={() => setManualLogPaths((prev) => prev.filter((x) => x !== p))}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
      <input
        className="mt-2 w-full rounded-md border border-input bg-background p-2 text-sm"
        value={contact}
        onChange={(e) => setContact(e.target.value)}
        placeholder={t.accContact}
      />
      {/* 自动附加日志 + 范围下拉（最近 1/2/5 次运行、最近一天） */}
      <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
        <label className="flex min-w-0 items-center gap-2">
          <input type="checkbox" checked={attachLog} onChange={(e) => setAttachLog(e.target.checked)} />
          <span className="min-w-0">{t.accDiagAttach}</span>
        </label>
        <select
          className="h-6 shrink-0 rounded-md border border-input bg-background px-1 text-xs"
          value={logRange}
          disabled={!attachLog}
          onChange={(e) => setLogRange(e.target.value as LogRangeKey)}
        >
          <option value="1">{t.accDiagRange1}</option>
          <option value="2">{t.accDiagRange2}</option>
          <option value="5">{t.accDiagRange5}</option>
          <option value="day">{t.accDiagRangeDay}</option>
        </select>
      </div>
      <Button className="mt-3" disabled={submitting} onClick={handleSubmit}>
        {submitting ? t.comLoading : t.accSubmit}
      </Button>
      <p className="mt-3 text-xs text-muted-foreground/80">{t.accLogHint}</p>
    </div>
  )
}
