'use client'

import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import type { DownloadableModelInfo } from '@/types'

/**
 * 模型下载源直链面板：每个源一行（用此源下载 / 复制链接）。
 * 设置页 ModelDownloadCard 与首启向导 OnboardingDialog 共用。
 * 复制链接可粘贴到迅雷 / IDM / aria2 等外部下载器，下完用「导入」安装。
 */
export function SourceLinksPanel({
  model,
  disabled,
  onUseSource,
}: {
  model: DownloadableModelInfo
  /** 禁用"用此源下载"（已安装 / 下载中） */
  disabled: boolean
  onUseSource: (sourceIndex: number) => void
}) {
  const t = useMessages()

  const copyLinks = (label: string, urls: string[]) => {
    navigator.clipboard
      .writeText(urls.join('\n'))
      .then(() =>
        toast.success(t.setCopiedLinks.replace('{count}', String(urls.length)).replace('{label}', label))
      )
      .catch(() => toast.error(t.setCopyFailed))
  }

  return (
    <div className="mt-2 rounded-md border border-border/60 bg-muted/30 px-3 py-2">
      <p className="text-xs text-muted-foreground mb-2">{t.setLinksHint}</p>
      <div className="flex flex-col gap-1.5">
        {model.sources.map((s, i) => (
          <div key={s.label} className="flex items-center gap-2">
            <span className="flex-1 min-w-0 truncate text-xs" title={s.urls.join('\n')}>
              {s.label}
            </span>
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs"
              disabled={disabled}
              onClick={() => onUseSource(i)}
            >
              {t.setDownloadThisSource}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs"
              onClick={() => copyLinks(s.label, s.urls)}
            >
              {t.setCopyLinks}
            </Button>
          </div>
        ))}
      </div>
    </div>
  )
}
