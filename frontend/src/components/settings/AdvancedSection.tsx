'use client'

import { Badge } from '@/components/ui/badge'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'
import type { Messages } from '@/i18n/messages'

/** 设置页 Advanced tab：即将上线功能占位（置灰，不响应点击） */
export function AdvancedSection() {
  const t = useMessages()
  const plannedFeatures: { name: keyof Messages; desc: keyof Messages }[] = [
    { name: 'setAdvTtsName', desc: 'setAdvTtsDesc' },
    { name: 'setAdvPttName', desc: 'setAdvPttDesc' },
    { name: 'setAdvSubtitleName', desc: 'setAdvSubtitleDesc' },
    { name: 'setAdvSelectionName', desc: 'setAdvSelectionDesc' },
    { name: 'setAdvLiveSummaryName', desc: 'setAdvLiveSummaryDesc' },
    { name: 'setAdvSpeakerIdName', desc: 'setAdvSpeakerIdDesc' },
    { name: 'setAdvCloudAsrName', desc: 'setAdvCloudAsrDesc' },
    { name: 'setAdvHelpName', desc: 'setAdvHelpDesc' },
  ]

  return (
    <SettingsSection title={t.setAdvancedTitle}>
      <div className="flex flex-col divide-y divide-border/60">
        {plannedFeatures.map((f) => (
          <div
            key={f.name}
            className="flex items-center justify-between gap-3 py-2.5 opacity-50 cursor-not-allowed"
            aria-disabled="true"
          >
            <div className="flex-1 min-w-0">
              <div className="text-xs font-medium text-muted-foreground">{t[f.name]}</div>
              <div className="text-[11px] text-muted-foreground/70">{t[f.desc]}</div>
            </div>
            <Badge variant="secondary" className="shrink-0">
              {t.setPlanned}
            </Badge>
          </div>
        ))}
      </div>
    </SettingsSection>
  )
}
