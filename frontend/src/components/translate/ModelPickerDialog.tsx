'use client'

import Link from 'next/link'
import { Settings2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useMessages } from '@/i18n/useMessages'
import { TranslateModelPicker, type TranslateModelValue } from './TranslateModelPicker'

interface ModelPickerDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  value: TranslateModelValue
  onChange: (next: Partial<TranslateModelValue>) => void
}

/**
 * 翻译主页的模型选择弹窗：卡片式展示所有可用翻译模型（本地 / 远程 / 自定义 API）。
 * 点卡片即切换引擎并关闭弹窗（远程卡片同时记下所选模型 id）。
 * 底部「更多设置」跳转 /translate/models 子页面。
 */
export function ModelPickerDialog({ open, onOpenChange, value, onChange }: ModelPickerDialogProps) {
  const t = useMessages()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl max-h-[80vh] overflow-y-auto custom-scrollbar">
        <DialogHeader>
          <DialogTitle>{t.trPickModel}</DialogTitle>
        </DialogHeader>

        <TranslateModelPicker
          value={value}
          onChange={(next) => {
            onChange(next)
            // 点卡片即切换并关闭弹窗；仅改远程模型时保持打开
            if (next.engine) onOpenChange(false)
          }}
        />

        <div className="flex justify-end border-t pt-3">
          <Link
            href="/translate/models"
            onClick={() => onOpenChange(false)}
            className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground"
          >
            <Settings2 className="h-3.5 w-3.5" />
            {t.trMoreSettings}
          </Link>
        </div>
      </DialogContent>
    </Dialog>
  )
}
