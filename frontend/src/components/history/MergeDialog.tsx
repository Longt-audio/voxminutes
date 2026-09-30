'use client'

import { useEffect, useMemo, useState } from 'react'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'
import type { RecordingListItem } from '@/types'
import { apiMergeRecordings } from '@/services/ipc'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useMessages } from '@/i18n/useMessages'
import { useHistoryFormat } from './format'

interface MergeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** 选中的工程（组件内按创建时间升序展示、按此顺序合并） */
  recordings: RecordingListItem[]
  /** 合并成功后回调（参数为新工程 id） */
  onMerged: (id: string) => void
}

/** 合并录音工程确认对话框 */
export function MergeDialog({ open, onOpenChange, recordings, onMerged }: MergeDialogProps) {
  const t = useMessages()
  const fmt = useHistoryFormat()
  const [title, setTitle] = useState('')
  const [deleteSources, setDeleteSources] = useState(false)
  const [merging, setMerging] = useState(false)

  // 按创建时间升序（合并顺序 = 时间顺序）
  const sorted = useMemo(
    () => [...recordings].sort((a, b) => a.created_at.localeCompare(b.created_at)),
    [recordings]
  )

  // 每次打开重置表单
  useEffect(() => {
    if (open) {
      setTitle('')
      setDeleteSources(false)
      setMerging(false)
    }
  }, [open])

  const handleConfirm = async () => {
    if (sorted.length < 2) {
      toast.error(t.histMergeNeedTwo)
      return
    }
    setMerging(true)
    try {
      const result = await apiMergeRecordings(
        sorted.map((r) => r.id),
        title.trim() || null,
        deleteSources,
        t.histMergeMarker
      )
      toast.success(t.histMergeSuccess)
      onOpenChange(false)
      onMerged(result.id)
    } catch (e) {
      toast.error(t.histMergeFailed.replace('{error}', e instanceof Error ? e.message : String(e)))
    } finally {
      setMerging(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !merging && onOpenChange(o)}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="text-base">{t.histMergeTitle}</DialogTitle>
        </DialogHeader>

        {/* 待合并工程列表（按时间顺序） */}
        <div className="max-h-[220px] overflow-y-auto custom-scrollbar rounded-md border divide-y">
          {sorted.map((r, i) => (
            <div key={r.id} className="flex items-center gap-2 px-3 py-2">
              <span className="shrink-0 text-[11px] text-muted-foreground w-4 text-right">{i + 1}.</span>
              <div className="flex-1 min-w-0">
                <div className="truncate text-xs font-medium">{r.title}</div>
                <div className="text-[11px] text-muted-foreground">{fmt.formatCreatedAtFull(r.created_at)}</div>
              </div>
            </div>
          ))}
        </div>

        {/* 新工程标题 */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.histMergeNewTitle}</label>
          <Input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t.histMergeNewTitleHint}
            disabled={merging}
            // 标题会成为导出文件名（Rust sanitize_filename 截到 80 字符）
            maxLength={80}
          />
        </div>

        {/* 删除原工程选项 */}
        <label className="flex items-center gap-2 text-xs cursor-pointer select-none">
          <input
            type="checkbox"
            className="h-3.5 w-3.5 accent-primary"
            checked={deleteSources}
            onChange={(e) => setDeleteSources(e.target.checked)}
            disabled={merging}
          />
          {t.histMergeDeleteSources}
        </label>

        <div className="flex justify-end gap-2 pt-1">
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)} disabled={merging}>
            {t.comCancel}
          </Button>
          <Button size="sm" className="gap-1.5" onClick={handleConfirm} disabled={merging || sorted.length < 2}>
            {merging && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
            {merging ? t.histMergeInProgress : t.histMergeConfirm}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
