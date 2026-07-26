'use client'

import { useState } from 'react'
import { Rocket } from 'lucide-react'
import { ModelDownloadCard } from '@/components/settings/ModelDownloadCard'
import { AudioSection } from '@/components/settings/AudioSection'
import { ExportSection } from '@/components/settings/ExportSection'
import { SummarySection } from '@/components/settings/SummarySection'
import { RemoteAsrSection } from '@/components/settings/RemoteAsrSection'
import { AdvancedSection } from '@/components/settings/AdvancedSection'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useMessages } from '@/i18n/useMessages'
import { apiSaveSetting } from '@/services/ipc'
import { OPEN_ONBOARDING_EVENT } from '@/components/onboarding/OnboardingDialog'

type SettingsTab = 'models' | 'audio' | 'api' | 'advanced'

export default function SettingsPage() {
  const t = useMessages()
  const [tab, setTab] = useState<SettingsTab>('models')

  const tabs: { key: SettingsTab; label: string }[] = [
    { key: 'models', label: t.setTabModels },
    { key: 'audio', label: t.setTabAudioExport },
    { key: 'api', label: t.setTabApi },
    { key: 'advanced', label: t.setTabAdvanced },
  ]

  // 重新打开首次启动向导：清掉完成标记并通知 AppShell 弹出
  const reopenOnboarding = () => {
    apiSaveSetting('onboarding.completed', null).catch(() => {})
    window.dispatchEvent(new Event(OPEN_ONBOARDING_EVENT))
  }

  return (
    <div className="h-full flex flex-col gap-3 p-5 overflow-hidden">
      {/* 页头：标题 + 新手指引入口（右上） */}
      <header className="shrink-0 flex items-start justify-between gap-3 max-w-[860px]">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">{t.navSettings}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t.setPageSubtitle}</p>
        </div>
        <Button variant="outline" size="sm" className="gap-1.5 shrink-0" onClick={reopenOnboarding}>
          <Rocket className="h-3.5 w-3.5" />
          {t.setReopenOnboarding}
        </Button>
      </header>

      {/* tab 栏（下划线风格，与总结对话框一致） */}
      <div className="shrink-0 flex gap-4 border-b border-border/60 max-w-[860px]">
        {tabs.map((item) => (
          <button
            key={item.key}
            className={cn(
              'px-1 pb-2 text-sm font-medium border-b-2 -mb-px transition-colors',
              tab === item.key
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            )}
            onClick={() => setTab(item.key)}
          >
            {item.label}
          </button>
        ))}
      </div>

      {/* 内容区（可滚动，最大宽度 860px） */}
      <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar">
        <div className="flex flex-col gap-6 max-w-[860px] pt-4 pb-6">
          {tab === 'models' && <ModelDownloadCard />}
          {tab === 'audio' && (
            <>
              <AudioSection />
              <ExportSection />
            </>
          )}
          {tab === 'api' && (
            <>
              <SummarySection />
              <RemoteAsrSection />
            </>
          )}
          {tab === 'advanced' && <AdvancedSection />}
        </div>
      </div>
    </div>
  )
}
