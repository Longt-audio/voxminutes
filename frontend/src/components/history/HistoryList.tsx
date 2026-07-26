'use client'

import type { ReactNode } from 'react'
import { toast } from 'sonner'
import { FolderOpen } from 'lucide-react'
import type { RecordingListItem } from '@/types'
import { openRecordingFolder } from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useMessages } from '@/i18n/useMessages'
import { useHistoryFormat } from './format'

interface HistoryListProps {
  recordings: RecordingListItem[]
  selectedId: string | null
  onSelect: (id: string) => void
  /** 列表卡片底部内容（导入按钮） */
  footer?: ReactNode
}

/** 左栏录音列表 + 底部操作区 */
export function HistoryList({ recordings, selectedId, onSelect, footer }: HistoryListProps) {
  const t = useMessages()
  const fmt = useHistoryFormat()

  const handleOpenFolder = async (id: string) => {
    try {
      await openRecordingFolder(id)
    } catch {
      toast.error(t.histOpenFolderFailed)
    }
  }

  return (
    <div className="flex-1 min-h-0 flex flex-col gap-2 rounded-lg border bg-card shadow-sm p-3">
      <div className="shrink-0 text-xs text-muted-foreground">{t.histListCount.replace('{count}', String(recordings.length))}</div>

      <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar flex flex-col gap-1">
        {recordings.length === 0 ? (
          <div className="py-8 text-center">
            <div className="text-sm font-medium text-muted-foreground">{t.histListEmptyTitle}</div>
            <div className="mt-1 text-xs text-muted-foreground/70">{t.histListEmptyHint}</div>
          </div>
        ) : (
          recordings.map((r) => (
            <div
              key={r.id}
              role="button"
              tabIndex={0}
              className={cn(
                'group w-full text-left rounded-md px-2.5 py-2 transition-colors hover:bg-muted/60 shrink-0 cursor-pointer flex items-center gap-1',
                selectedId === r.id && 'bg-primary/10 text-primary hover:bg-primary/10'
              )}
              onClick={() => onSelect(r.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') onSelect(r.id)
              }}
            >
              <div className="flex-1 min-w-0">
                <div className="truncate text-xs font-medium leading-tight">{r.title}</div>
                <div className="mt-1 flex items-center justify-between gap-2">
                  <span className="text-[11px] text-muted-foreground leading-tight">{fmt.formatCreatedAt(r.created_at)}</span>
                  {!r.folder_path && <Badge variant="warning">{t.histFileMissing}</Badge>}
                </div>
              </div>
              {r.folder_path && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
                  title={t.histOpenFolder}
                  onClick={(e) => {
                    e.stopPropagation()
                    handleOpenFolder(r.id)
                  }}
                >
                  <FolderOpen className="h-3 w-3" />
                </Button>
              )}
            </div>
          ))
        )}
      </div>

      {footer && <div className="shrink-0 border-t border-border/60 pt-2">{footer}</div>}
    </div>
  )
}
