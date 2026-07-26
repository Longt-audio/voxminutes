'use client'

import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  selectAndValidateAudio,
  startImportAudio,
  onImportProgress,
  onImportComplete,
  onImportError,
} from '@/services/ipc'
import { useMessages } from '@/i18n/useMessages'
import { Button } from '@/components/ui/button'

interface ImportButtonProps {
  /** 导入完成后回调：刷新列表并选中新 recording */
  onImported: (recordingId: string) => void
}

/** 「导入音频」按钮（列表底部整宽）：选择文件 → 导入 → 进度/完成/错误事件 */
export function ImportButton({ onImported }: ImportButtonProps) {
  const t = useMessages()
  const [importing, setImporting] = useState(false)
  const [progress, setProgress] = useState<number | null>(null)

  const onImportedRef = useRef(onImported)
  useEffect(() => {
    onImportedRef.current = onImported
  })

  // 导入事件监听（卸载时取消）
  useEffect(() => {
    let disposed = false
    let unlistens: UnlistenFn[] = []
    Promise.all([
      onImportProgress((p) => setProgress(p.progress_percentage)),
      onImportComplete((r) => {
        setImporting(false)
        setProgress(null)
        toast.success(t.histImportDone.replace('{title}', r.title))
        onImportedRef.current(r.meeting_id)
      }),
      onImportError((e) => {
        setImporting(false)
        setProgress(null)
        toast.error(t.histImportFailed.replace('{error}', e.error))
      }),
    ]).then((fns) => {
      if (disposed) fns.forEach((f) => f())
      else unlistens = fns
    })
    return () => {
      disposed = true
      unlistens.forEach((f) => f())
    }
  }, [t])

  const handleImport = async () => {
    let info
    try {
      info = await selectAndValidateAudio()
    } catch {
      toast.error(t.histImportSelectFailed)
      return
    }
    if (!info) return // 用户取消
    const title = info.filename.replace(/\.[^.]+$/, '') || info.filename
    setImporting(true)
    setProgress(0)
    try {
      await startImportAudio(info.path, title)
    } catch {
      setImporting(false)
      setProgress(null)
      toast.error(t.histImportStartFailed)
    }
  }

  return (
    <div className="flex flex-col gap-1.5">
      <Button className="w-full" size="sm" onClick={handleImport} disabled={importing}>
        {importing
          ? `${t.histImporting}${progress != null ? ` ${Math.round(progress)}%` : ''}`
          : t.histImportAudio}
      </Button>
      {importing && (
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <div className="h-full bg-primary transition-all" style={{ width: `${progress ?? 0}%` }} />
        </div>
      )}
    </div>
  )
}
