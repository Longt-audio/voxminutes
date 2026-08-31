'use client'

import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { getStartupNoticeDisabled, setStartupNoticeDisabled } from '@/components/StartupNoticeDialog'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 设置页通用区：启动时长信息弹窗开关（默认显示；用户可在启动弹窗里"不再显示"，这里可恢复）。 */
export function GeneralSection() {
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

  return (
    <SettingsSection title={t.setGeneralTitle} description={t.setGeneralHint}>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={!disabled}
          onChange={(e) => handleToggle(!e.target.checked)}
          className="h-4 w-4"
          disabled={loading}
        />
        <span>{t.setStartupNotice}</span>
      </label>
    </SettingsSection>
  )
}
