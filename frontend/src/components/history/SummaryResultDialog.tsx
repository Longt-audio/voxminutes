'use client'

import { useEffect, useRef, useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { toast } from 'sonner'
import { Square } from 'lucide-react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { useSummaryGeneration, type SummaryGenerateParams } from '@/hooks/useSummaryGeneration'
import { summaryExportMarkdown, summaryLoad, summarySave } from '@/services/ipc'

interface SummaryResultDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  recordingId: string
  source: 'realtime' | 'offline'
  /** 生成模式参数（打开时自动开始生成）；为 null 时是只读模式（加载已保存总结） */
  generate: SummaryGenerateParams | null
  onSaved?: () => void
}

/** 会议总结结果面板：流式生成 / 查看已保存，工具栏支持停止、重新生成、复制、导出 MD、保存 */
export function SummaryResultDialog({
  open,
  onOpenChange,
  recordingId,
  source,
  generate,
  onSaved,
}: SummaryResultDialogProps) {
  const t = useMessages()
  const { result, streaming, start, stop, reset, setContent } = useSummaryGeneration(
    recordingId,
    source,
    onSaved
  )
  const [copied, setCopied] = useState(false)
  // 防止同一轮打开重复发起生成（StrictMode 双调用 effect）
  const startedRef = useRef(false)

  // 打开：生成模式自动开始生成；只读模式加载已保存内容
  useEffect(() => {
    if (!open) {
      startedRef.current = false
      return
    }
    if (startedRef.current) return
    startedRef.current = true
    if (generate) {
      start(generate)
    } else {
      reset()
      summaryLoad(recordingId, source)
        .then((saved) => setContent(saved ?? ''))
        .catch(() => {})
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open])

  const handleRegenerate = () => {
    if (!generate || streaming) return
    reset()
    start(generate)
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(result)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1500)
    } catch {
      toast.error(t.comError)
    }
  }

  const handleExportMd = async () => {
    if (!result.trim()) return
    try {
      const path = await summaryExportMarkdown(recordingId, result)
      toast.success(t.sumExportedTo.replace('{path}', path))
    } catch (e) {
      toast.error(t.sumExportFailed.replace('{error}', String(e)))
    }
  }

  const handleSave = async () => {
    if (!result.trim()) return
    try {
      const path = await summarySave(recordingId, source, result)
      toast.success(t.sumSavedTo.replace('{path}', path))
      onSaved?.()
    } catch (e) {
      toast.error(t.sumSaveFailed.replace('{error}', String(e)))
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="text-base">{t.sumResultTitle}</DialogTitle>
        </DialogHeader>

        {/* 工具栏 */}
        <div className="shrink-0 flex items-center gap-1">
          {streaming && (
            <Button variant="outline" size="sm" className="h-7 px-2 text-xs gap-1" onClick={stop}>
              <Square className="h-3 w-3" />
              {t.sumStop}
            </Button>
          )}
          {generate && !streaming && (
            <Button variant="ghost" size="sm" className="h-7 px-2 text-xs" onClick={handleRegenerate}>
              {t.sumRegenerate}
            </Button>
          )}
          <div className="flex-1" />
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!result}
            onClick={handleCopy}
          >
            {copied ? t.comCopied : t.comCopy}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!result.trim() || streaming}
            onClick={handleExportMd}
          >
            {t.sumExportMd}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!result.trim()}
            onClick={handleSave}
          >
            {t.comSave}
          </Button>
        </div>

        {/* 结果区 */}
        <div className="flex-1 min-h-[200px] overflow-y-auto custom-scrollbar rounded-md border bg-muted/30 p-3">
          {result ? (
            <div className="md-body">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{result}</ReactMarkdown>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              {streaming ? t.sumGenerating : t.sumResultPlaceholder}
            </p>
          )}
        </div>

        <div className="shrink-0 flex items-center justify-end">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t.comClose}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
