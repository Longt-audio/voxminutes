'use client'

import { useEffect, useState } from 'react'
import { getDefaultAudioDevices, openSystemSoundSettings, getFlowPauseSecs, setFlowPauseSecs } from '@/services/ipc'
import type { DefaultDevicesInfo } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

const FLOW_PAUSE_OPTIONS = [5, 10, 20]

/** 设置页 Section 2：音频 —— 当前默认设备展示 + 系统声音设置入口 + 流式分段停顿 */
export function AudioSection() {
  const t = useMessages()
  const [devices, setDevices] = useState<DefaultDevicesInfo>({ microphone: null, speaker: null })
  const [flowPauseSecs, setFlowPauseSecsState] = useState<number | null>(null)

  useEffect(() => {
    getDefaultAudioDevices()
      // 后端异常/返回 null 时保持空对象——直接 setDevices(null) 会让下方
      // devices.microphone 渲染期抛 TypeError，整个 tab 进错误边界
      .then((d) => setDevices(d ?? { microphone: null, speaker: null }))
      .catch(() => {})
    getFlowPauseSecs()
      .then(setFlowPauseSecsState)
      .catch(() => {})
  }, [])

  const handleFlowPauseChange = (v: string) => {
    const secs = Number(v)
    const prev = flowPauseSecs
    setFlowPauseSecsState(secs)
    setFlowPauseSecs(secs).catch(() => setFlowPauseSecsState(prev))
  }

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

      {/* 流式分段停顿（仅流式 ASR 引擎生效） */}
      <div className="mt-4 flex items-center gap-3">
        <div className="flex-1 min-w-0 flex flex-col gap-0.5">
          <span className="text-sm">{t.flowPauseLabel}</span>
          <span className="text-xs text-muted-foreground">{t.flowPauseDesc}</span>
        </div>
        {/* value 必须始终受控：未加载时给 ''（空值占位），不能传 undefined——
            否则 IPC 读回后 undefined → string 会触发 Radix「uncontrolled → controlled」警告 */}
        <Select
          value={flowPauseSecs != null ? String(flowPauseSecs) : ''}
          onValueChange={handleFlowPauseChange}
        >
          <SelectTrigger className="w-28 shrink-0 h-8 text-xs">
            <SelectValue placeholder={t.comLoading} />
          </SelectTrigger>
          <SelectContent>
            {FLOW_PAUSE_OPTIONS.map((n) => (
              <SelectItem key={n} value={String(n)} className="text-xs">
                {t.flowPauseSecsOption.replace('{n}', String(n))}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </SettingsSection>
  )
}
