'use client'

import { useEffect, useState, type ReactNode } from 'react'
import { toast } from 'sonner'
import { FolderOpen, X } from 'lucide-react'
import type { RecordingListItem } from '@/types'
import { openRecordingFolder } from '@/services/ipc'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useMessages } from '@/i18n/useMessages'
import { useHistoryFormat } from './format'
import { MergeDialog } from './MergeDialog'

interface HistoryListProps {
  recordings: RecordingListItem[]
  selectedId: string | null
  onSelect: (id: string) => void
  /** 列表卡片底部内容（导入按钮） */
  footer?: ReactNode
  /** 合并成功后回调（参数为新工程 id） */
  onMerged?: (id: string) => void
}

/** 左栏录音列表 + 多选合并 + 底部操作区 */
export function HistoryList({ recordings, selectedId, onSelect, footer, onMerged }: HistoryListProps) {
  const t = useMessages()
  const fmt = useHistoryFormat()
  // 合并多选：选中的工程 id 集合
  const [checkedIds, setCheckedIds] = useState<string[]>([])
  const [mergeOpen, setMergeOpen] = useState(false)

  // 列表刷新后剔除已不存在的选中项（如合并后删除原工程）
  useEffect(() => {
    setCheckedIds((prev) => {
      const alive = prev.filter((id) => recordings.some((r) => r.id === id))
      return alive.length === prev.length ? prev : alive
    })
  }, [recordings])

  const toggleChecked = (id: string) => {
    setCheckedIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))
  }

  const handleOpenFolder = async (id: string) => {
    try {
      await openRecordingFolder(id)
    } catch {
      toast.error(t.histOpenFolderFailed)
    }
  }

  const checkedRecordings = recordings.filter((r) => checkedIds.includes(r.id))

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
          recordings.map((r) => {
            const checked = checkedIds.includes(r.id)
            return (
              <div
                key={r.id}
                role="button"
                tabIndex={0}
                className={cn(
                  'group w-full text-left rounded-md px-2.5 py-2 transition-colors hover:bg-muted/60 shrink-0 cursor-pointer flex items-center gap-1.5',
                  selectedId === r.id && 'bg-primary/10 text-primary hover:bg-primary/10'
                )}
                onClick={() => onSelect(r.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') onSelect(r.id)
                }}
              >
                {/* 合并选择框：hover / 已勾选 / 已有选中项时显示 */}
                <input
                  type="checkbox"
                  aria-label={t.histMerge}
                  className={cn(
                    'h-3.5 w-3.5 shrink-0 accent-primary cursor-pointer transition-opacity',
                    checked || checkedIds.length > 0 ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
                  )}
                  checked={checked}
                  onChange={() => toggleChecked(r.id)}
                  onClick={(e) => e.stopPropagation()}
                />
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
            )
          })
        )}
      </div>

      {/* 合并工具条：有选中项时出现 */}
      {checkedIds.length > 0 && (
        <div className="shrink-0 flex items-center gap-1.5 border-t border-border/60 pt-2">
          <Button
            size="sm"
            className="flex-1 h-7 text-xs"
            disabled={checkedIds.length < 2}
            title={checkedIds.length < 2 ? t.histMergeNeedTwo : undefined}
            onClick={() => {
              if (checkedIds.length < 2) {
                toast.error(t.histMergeNeedTwo)
                return
              }
              setMergeOpen(true)
            }}
          >
            {t.histMergeSelected.replace('{n}', String(checkedIds.length))}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 shrink-0"
            title={t.histMergeClear}
            onClick={() => setCheckedIds([])}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}

      {footer && <div className="shrink-0 border-t border-border/60 pt-2">{footer}</div>}

      <MergeDialog
        open={mergeOpen}
        onOpenChange={setMergeOpen}
        recordings={checkedRecordings}
        onMerged={(id) => {
          setCheckedIds([])
          onMerged?.(id)
        }}
      />
    </div>
  )
}
