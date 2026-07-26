'use client'

import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { ChevronDown, Download } from 'lucide-react'
import { apiExportRecording } from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { Button } from '@/components/ui/button'

type ExportFormat = 'txt' | 'srt' | 'markdown'

const FORMAT_OPTIONS: { value: ExportFormat; label: string }[] = [
  { value: 'txt', label: 'TXT' },
  { value: 'srt', label: 'SRT' },
  { value: 'markdown', label: 'Markdown' },
]

interface ExportMenuProps {
  recordingId: string
  /** 导出哪份结果：实时识别 / 离线识别 */
  source: 'realtime' | 'offline_asr'
  disabled?: boolean
}

/** 三合一导出：一个按钮，点击后选择 TXT / SRT / Markdown 格式 */
export function ExportMenu({ recordingId, source, disabled }: ExportMenuProps) {
  const t = useMessages()
  const [open, setOpen] = useState(false)
  const [exporting, setExporting] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)

  // 点击外部关闭菜单
  useEffect(() => {
    if (!open) return
    const onDocMouseDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', onDocMouseDown)
    return () => document.removeEventListener('mousedown', onDocMouseDown)
  }, [open])

  const handleExport = async (format: ExportFormat) => {
    setOpen(false)
    setExporting(true)
    try {
      const result = await apiExportRecording(recordingId, format, source)
      toast.success(t.histExported.replace('{path}', result.path))
    } catch {
      toast.error(t.histExportFailed)
    } finally {
      setExporting(false)
    }
  }

  return (
    <div ref={rootRef} className="relative">
      <Button
        variant="outline"
        size="sm"
        className="gap-1.5"
        disabled={disabled || exporting}
        onClick={() => setOpen((v) => !v)}
      >
        <Download className="h-3.5 w-3.5" />
        {exporting ? t.histExporting : t.comExport}
        <ChevronDown className="h-3 w-3 opacity-60" />
      </Button>
      {open && (
        <div className="absolute right-0 z-20 mt-1 w-[120px] rounded-md border bg-popover p-1 shadow-md">
          {FORMAT_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              className="w-full rounded-sm px-2 py-1.5 text-left text-xs hover:bg-muted"
              onClick={() => handleExport(opt.value)}
            >
              {t.histExportFormat.replace('{format}', opt.label)}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
