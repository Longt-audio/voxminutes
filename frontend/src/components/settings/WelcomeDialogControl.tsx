'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Bell } from 'lucide-react'
import { getWelcomeDisabled, setWelcomeDisabled, OPEN_WELCOME_EVENT } from '@/components/onboarding/WelcomeDialog'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

/** 设置页 header 里的欢迎弹窗控制：「以后不再打开」勾选框 + 「再次打开欢迎弹窗」按钮。 */
export function WelcomeDialogControl() {
  const t = useMessages()
  const [disabled, setDisabled] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    getWelcomeDisabled()
      .then(setDisabled)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  const handleToggle = async (next: boolean) => {
    setDisabled(next)
    try {
      await setWelcomeDisabled(next)
    } catch {
      setDisabled(!next)
      toast.error(t.setSaveFailed.replace('{error}', ''))
    }
  }

  const handleReshow = () => {
    window.dispatchEvent(new Event(OPEN_WELCOME_EVENT))
  }

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-muted-foreground">{t.setWelcomeDialog}</span>
      <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer select-none">
        <input
          type="checkbox"
          checked={disabled}
          onChange={(e) => handleToggle(e.target.checked)}
          className="h-3.5 w-3.5"
          disabled={loading}
        />
        {t.welDontShowAgain}
      </label>
      <Button variant="outline" size="sm" className="h-7 gap-1.5 text-xs" onClick={handleReshow}>
        <Bell className="h-3 w-3" />
        {t.setReshowWelcome}
      </Button>
    </div>
  )
}
