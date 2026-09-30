'use client'

import { useEffect, useMemo, useRef, useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { toast } from 'sonner'
import { Brain, ChevronDown, ChevronRight, Columns2, Eye, FileCode2, Square } from 'lucide-react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { useSummaryGeneration, type SummaryGenerateParams } from '@/hooks/useSummaryGeneration'
import { summaryExportMarkdown, summaryLoad, summarySave } from '@/services/ipc'
import { openSummaryPrintWindow } from '@/lib/summaryPdf'

interface SummaryResultDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  recordingId: string
  /** 会议标题（导出 PDF 的文档标题/窗口标题用） */
  title: string
  source: 'realtime' | 'offline'
  /** 生成模式参数（打开时自动开始生成）；为 null 时是只读模式（加载已保存总结） */
  generate: SummaryGenerateParams | null
  onSaved?: () => void
}

/** Markdown → 简易 HTML（"带格式复制"用；只做富文本剪贴板需要的最小转换）。 */
function markdownToHtml(md: string): string {
  const esc = (s: string) =>
    s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  const inline = (s: string) =>
    esc(s)
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>')
  const out: string[] = []
  let inList = false
  const closeList = () => {
    if (inList) {
      out.push('</ul>')
      inList = false
    }
  }
  for (const raw of md.split('\n')) {
    const line = raw.trimEnd()
    const h = /^(#{1,6})\s+(.*)$/.exec(line)
    const li = /^[-*+]\s+(.*)$/.exec(line)
    const ol = /^\d+[.)]\s+(.*)$/.exec(line)
    if (h) {
      closeList()
      const lvl = h[1].length
      out.push(`<h${lvl}>${inline(h[2])}</h${lvl}>`)
    } else if (li || ol) {
      if (!inList) {
        out.push('<ul>')
        inList = true
      }
      out.push(`<li>${inline((li ?? ol)![1])}</li>`)
    } else if (!line.trim()) {
      closeList()
    } else {
      closeList()
      out.push(`<p>${inline(line)}</p>`)
    }
  }
  closeList()
  return `<div>${out.join('\n')}</div>`
}

/**
 * 会议总结结果面板：流式生成 / 查看已保存。
 *
 * 2026-09-24 改版（参考 mig 的 AiSummaryResultPanel）：
 *  · 思维链（reasoning_content）可折叠展示 —— 「AI 思考过程」，默认收起、思考中自动展开；
 *  · 默认**带格式预览**，可展开右侧 Markdown 源码并就地编辑（左右分栏、默认不展开）；
 *  · 「带格式复制」= 同时写 text/html 与 text/plain 到剪贴板（粘到 Word/微信保留格式）；
 *  · 模型只思考不产出正文时给出明确提示（网关侧对空正文已免单），而不是显示一片空白。
 */
export function SummaryResultDialog({
  open,
  onOpenChange,
  recordingId,
  title,
  source,
  generate,
  onSaved,
}: SummaryResultDialogProps) {
  const t = useMessages()
  const { result, streaming, thinking, truncated, start, stop, reset, setContent } = useSummaryGeneration(
    recordingId,
    source,
    onSaved
  )
  const [copied, setCopied] = useState(false)
  /** 源码编辑（Markdown 原文）：默认不展开（用户只要格式预览）；编辑后即时预览 */
  const [sourceOpen, setSourceOpen] = useState(false)
  const [draft, setDraft] = useState('')
  /** 思考过程折叠块：默认收起；思考中自动展开一次 */
  const [thinkOpen, setThinkOpen] = useState(false)
  const autoOpenedRef = useRef(false)
  // 防止同一轮打开重复发起生成（StrictMode 双调用 effect）
  const startedRef = useRef(false)

  // 打开：生成模式自动开始生成；只读模式加载已保存内容
  useEffect(() => {
    if (!open) {
      startedRef.current = false
      autoOpenedRef.current = false
      setSourceOpen(false)
      setThinkOpen(false)
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

  // 正文/思考更新时同步草稿（未手动改过就跟随；编辑模式下以用户输入为准）
  useEffect(() => {
    setDraft(result)
  }, [result])

  // 思考中自动展开思考过程（每轮只自动展开一次，用户收起后不打扰）
  useEffect(() => {
    if (thinking && streaming && !autoOpenedRef.current) {
      autoOpenedRef.current = true
      setThinkOpen(true)
    }
    if (streaming && !thinking) autoOpenedRef.current = false
  }, [thinking, streaming])

  /** 预览用内容：源码编辑中显示草稿（所见即所得），否则显示生成结果 */
  const previewContent = sourceOpen && draft !== result ? draft : result
  const emptyWithThinking = useMemo(
    () => !streaming && !result.trim() && thinking.trim().length > 0,
    [streaming, result, thinking]
  )

  const handleRegenerate = () => {
    if (!generate || streaming) return
    reset()
    setSourceOpen(false)
    setDraft('')
    start(generate)
  }

  /** 带格式复制：text/html（保留标题/列表/加粗）+ text/plain（Markdown 原文） */
  const handleCopy = async () => {
    const md = previewContent
    if (!md.trim()) return
    try {
      const html = markdownToHtml(md)
      const ClipboardItemCtor = (globalThis as { ClipboardItem?: typeof ClipboardItem }).ClipboardItem
      if (navigator.clipboard?.write && ClipboardItemCtor) {
        await navigator.clipboard.write([
          new ClipboardItemCtor({
            'text/html': new Blob([html], { type: 'text/html' }),
            'text/plain': new Blob([md], { type: 'text/plain' }),
          }),
        ])
      } else {
        await navigator.clipboard.writeText(md)
      }
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1500)
    } catch {
      // 富文本剪贴板被拒（部分平台只允许纯文本）→ 退回纯文本
      try {
        await navigator.clipboard.writeText(md)
        setCopied(true)
        window.setTimeout(() => setCopied(false), 1500)
      } catch {
        toast.error(t.comError)
      }
    }
  }

  const handleExportMd = async () => {
    const md = previewContent
    if (!md.trim()) return
    try {
      const path = await summaryExportMarkdown(recordingId, md)
      toast.success(t.sumExportedTo.replace('{path}', path))
    } catch (e) {
      toast.error(t.sumExportFailed.replace('{error}', String(e)))
    }
  }

  // 导出 PDF：本地 marked 渲染 → 打印预览窗口 → 系统打印对话框「存为 PDF」
  const handleExportPdf = async () => {
    const md = previewContent
    if (!md.trim()) return
    try {
      await openSummaryPrintWindow(title, md)
    } catch (e) {
      toast.error(t.sumExportFailed.replace('{error}', String(e)))
    }
  }

  const handleSave = async () => {
    const md = previewContent
    if (!md.trim()) return
    try {
      const path = await summarySave(recordingId, source, md)
      if (sourceOpen) setContent(md)
      toast.success(t.sumSavedTo.replace('{path}', path))
      onSaved?.()
    } catch (e) {
      toast.error(t.sumSaveFailed.replace('{error}', String(e)))
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-5xl max-h-[88vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="text-base">{t.sumResultTitle}</DialogTitle>
        </DialogHeader>

        {/* 工具栏 */}
        <div className="shrink-0 flex items-center gap-1 flex-wrap">
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
          <Button
            variant={sourceOpen ? 'secondary' : 'ghost'}
            size="sm"
            className="h-7 px-2 text-xs gap-1"
            disabled={!result.trim() && !streaming}
            onClick={() => setSourceOpen((v) => !v)}
            title={t.sumSourceHint}
          >
            {sourceOpen ? <Eye className="h-3 w-3" /> : <FileCode2 className="h-3 w-3" />}
            {sourceOpen ? t.sumPreviewOnly : t.sumShowSource}
          </Button>
          <div className="flex-1" />
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!previewContent.trim()}
            onClick={handleCopy}
          >
            {copied ? t.comCopied : t.sumCopyRich}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!previewContent.trim() || streaming}
            onClick={handleExportMd}
          >
            {t.sumExportMd}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!previewContent.trim() || streaming}
            onClick={handleExportPdf}
          >
            {t.sumExportPdf}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            disabled={!previewContent.trim()}
            onClick={handleSave}
          >
            {t.comSave}
          </Button>
        </div>

        {/* 思考过程（可折叠）：参考 mig 的 amber 折叠块 */}
        {(thinking || (streaming && !result)) && (
          <div className="shrink-0 mb-1 border border-amber-200 dark:border-amber-800 rounded-md overflow-hidden">
            <button
              type="button"
              className="w-full flex items-center gap-1.5 px-3 py-1.5 text-xs bg-amber-50 dark:bg-amber-950/30 hover:bg-amber-100 dark:hover:bg-amber-950/50 transition-colors"
              onClick={() => setThinkOpen((v) => !v)}
            >
              <Brain className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400" />
              <span className="text-amber-700 dark:text-amber-300 font-medium">{t.sumThinkingTitle}</span>
              {thinking && (
                <span className="text-[10px] text-amber-600/80 dark:text-amber-400/80">
                  {t.sumThinkingChars.replace('{chars}', String(thinking.length))}
                </span>
              )}
              {streaming && !result && (
                <span className="text-[10px] text-amber-600/80 dark:text-amber-400/80">{t.sumGenerating}</span>
              )}
              <span className="flex-1" />
              {thinkOpen ? (
                <ChevronDown className="h-3.5 w-3.5 text-amber-500" />
              ) : (
                <ChevronRight className="h-3.5 w-3.5 text-amber-500" />
              )}
            </button>
            {thinkOpen && (
              <pre className="px-3 py-2 text-[11px] font-mono text-amber-800 dark:text-amber-200 whitespace-pre-wrap break-words max-h-40 overflow-y-auto custom-scrollbar m-0">
                {thinking || t.sumThinkingWaiting}
              </pre>
            )}
          </div>
        )}

        {/* 结果区：默认格式预览；展开源码后左右分栏（源码可编辑，预览实时跟随） */}
        <div
          className={
            sourceOpen
              ? 'flex-1 min-h-[240px] grid grid-cols-1 md:grid-cols-2 gap-2 overflow-hidden'
              : 'flex-1 min-h-[240px] overflow-y-auto custom-scrollbar rounded-md border bg-muted/30 p-3'
          }
        >
          {sourceOpen ? (
            <>
              <div className="min-h-0 overflow-y-auto custom-scrollbar rounded-md border bg-muted/30 p-3">
                {previewContent.trim() ? (
                  <div className="md-body">
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>{previewContent}</ReactMarkdown>
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">{t.sumResultPlaceholder}</p>
                )}
              </div>
              <textarea
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                spellCheck={false}
                className="min-h-0 h-full w-full text-xs font-mono p-3 rounded-md border border-border/60 bg-background resize-none custom-scrollbar"
                placeholder={t.sumSourcePlaceholder}
              />
            </>
          ) : result.trim() ? (
            <div className="md-body">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{result}</ReactMarkdown>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              {streaming
                ? thinking
                  ? t.sumThinkingChars.replace('{chars}', String(thinking.length))
                  : t.sumGenerating
                : t.sumResultPlaceholder}
            </p>
          )}
        </div>

        {/* 输出被 max_tokens 截断（finish_reason=length）：内联警告 + 重新生成入口。
            截断的半截总结已照常自动保存，这里只是提示用户它可能不完整。 */}
        {!streaming && truncated && result.trim() && (
          <div className="shrink-0 mt-2 rounded-md border border-amber-300/60 bg-amber-50/60 dark:bg-amber-950/20 px-3 py-2">
            <p className="text-xs text-amber-800 dark:text-amber-200">{t.sumOutputTruncated}</p>
            {generate && (
              <Button variant="outline" size="sm" className="h-7 px-2 text-xs mt-2" onClick={handleRegenerate}>
                {t.sumRegenerate}
              </Button>
            )}
          </div>
        )}

        {/* 只思考没正文：明确告知（网关对空正文已免单，不计费） */}
        {emptyWithThinking && (
          <div className="shrink-0 mt-2 rounded-md border border-amber-300/60 bg-amber-50/60 dark:bg-amber-950/20 px-3 py-2">
            <p className="text-xs text-amber-800 dark:text-amber-200">{t.sumEmptyWithThinking}</p>
            {generate && (
              <Button variant="outline" size="sm" className="h-7 px-2 text-xs mt-2" onClick={handleRegenerate}>
                {t.sumRegenerate}
              </Button>
            )}
          </div>
        )}

        <div className="shrink-0 flex items-center justify-between">
          {sourceOpen && draft !== result ? (
            <span className="text-[11px] text-muted-foreground flex items-center gap-1">
              <Columns2 className="h-3 w-3" />
              {t.sumSourceDirty}
            </span>
          ) : (
            <span />
          )}
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t.comClose}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
