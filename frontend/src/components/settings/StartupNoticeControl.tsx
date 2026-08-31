'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Bell } from 'lucide-react'
import { getStartupNoticeDisabled, setStartupNoticeDisabled, RESHOW_STARTUP_NOTICE_EVENT } from '@/components/StartupNoticeDialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 设置页 header 里的启动弹窗控制：开关 + 「再次弹出」按钮。 */
export function StartupNoticeControl() {
  const t = useMessages()
  const [disabled, setDisabled] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    getStartupNoticeDisabled()
      .then(setDisabled)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  const handleToggle = async (next: boolean) => {
    setDisabled(next)
    try {
      await setStartupNoticeDisabled(next)
    } catch {
      setDisabled(!next)
      toast.error(t.setSaveFailed.replace('{error}', ''))
    }
  }

  const handleReshow = () => {
    window.dispatchEvent(new Event(RESHOW_STARTUP_NOTICE_EVENT))
  }

  return (
    <div className="flex items-center gap-2">
      <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer select-none">
        <input
          type="checkbox"
          checked={!disabled}
          onChange={(e) => handleToggle(!e.target.checked)}
          className="h-3.5 w-3.5"
          disabled={loading}
        />
        {t.setStartupNotice}
      </label>
      <Button variant="outline" size="sm" className="h-7 gap-1.5 text-xs" onClick={handleReshow}>
        <Bell className="h-3 w-3" />
        {t.setReshowStartupNotice}
      </Button>
    </div>
  )
}
