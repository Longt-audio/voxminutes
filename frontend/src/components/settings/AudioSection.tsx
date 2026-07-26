'use client'

import { useEffect, useState } from 'react'
import { getDefaultAudioDevices, openSystemSoundSettings } from '@/services/ipc'
import type { DefaultDevicesInfo } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 设置页 Section 2：音频 —— 当前默认设备展示 + 系统声音设置入口 */
export function AudioSection() {
  const t = useMessages()
  const [devices, setDevices] = useState<DefaultDevicesInfo>({ microphone: null, speaker: null })

  useEffect(() => {
    getDefaultAudioDevices()
      .then(setDevices)
      .catch(() => {})
  }, [])

  return (
    <SettingsSection title={t.setAudio}>
      <div className="flex gap-4">
        <div className="flex-1 min-w-0 flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.setMicrophone}</span>
          <Input readOnly value={devices.microphone || t.setNoDevice} />
        </div>
        <div className="flex-1 min-w-0 flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{t.setSystemAudio}</span>
          <Input readOnly value={devices.speaker || t.setNoDevice} />
        </div>
      </div>

      <div className="mt-4 flex items-center gap-3">
        <Button onClick={() => openSystemSoundSettings().catch(() => {})}>
          {t.setOpenSoundSettings}
        </Button>
        <span className="text-xs text-muted-foreground">
          {t.setDeviceHint}
        </span>
      </div>
    </SettingsSection>
  )
}
