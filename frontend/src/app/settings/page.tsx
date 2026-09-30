'use client'

import { Suspense, useEffect, useState } from 'react'
import { useSearchParams } from 'next/navigation'
import { ModelDownloadCard } from '@/components/settings/ModelDownloadCard'
import { AudioSection } from '@/components/settings/AudioSection'
import { ExportSection } from '@/components/settings/ExportSection'
import { WelcomeDialogControl } from '@/components/settings/WelcomeDialogControl'
import { AboutSection } from '@/components/settings/AboutSection'
import { CustomApiConfigForm } from '@/components/settings/CustomApiConfigForm'
import { RemoteServerCard } from '@/components/settings/RemoteServerCard'
import { LanguageSwitcher } from '@/components/LanguageSwitcher'
import { cn } from '@/lib/utils'
import { useMessages } from '@/i18n/useMessages'

type SettingsTab = 'models' | 'audio' | 'customApi' | 'about'

const TAB_KEYS: readonly SettingsTab[] = ['models', 'audio', 'customApi', 'about']

function SettingsPageInner() {
  const t = useMessages()
  const searchParams = useSearchParams()
  const [tab, setTab] = useState<SettingsTab>('models')

  // 支持 URL query 定位 tab（如 /settings?tab=customApi，供 SummaryDialog / 翻译页等处跳转）
  useEffect(() => {
    const q = searchParams.get('tab')
    if (q && (TAB_KEYS as readonly string[]).includes(q)) setTab(q as SettingsTab)
  }, [searchParams])

  const tabs: { key: SettingsTab; label: string }[] = [
    { key: 'models', label: t.setTabModels },
    { key: 'audio', label: t.setTabAudioExport },
    { key: 'customApi', label: t.setTabCustomApi },
    { key: 'about', label: t.setTabAbout },
  ]

  return (
    <div className="h-full flex flex-col gap-3 p-5 overflow-hidden">
      {/* 页头：标题 + 欢迎弹窗入口 + 界面语言（右上同一行） */}
      <header className="shrink-0 flex items-start justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">{t.navSettings}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t.setPageSubtitle}</p>
        </div>
        <div className="flex items-center gap-3 shrink-0">
          {/* 欢迎弹窗：「以后不再打开」勾选框 + 再次打开 */}
          <WelcomeDialogControl />
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">{t.languageLabel}</span>
            <LanguageSwitcher />
          </div>
        </div>
      </header>

      {/* tab 栏（下划线风格，与总结对话框一致；与内容同宽，铺满可用宽度） */}
      <div className="shrink-0 flex gap-4 border-b border-border/60">
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

      {/* 内容区（可滚动，铺满可用宽度） */}
      <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar">
        <div className="flex flex-col gap-6 pt-4 pb-6">
          {tab === 'models' && <ModelDownloadCard />}
          {tab === 'audio' && (
            <>
              <AudioSection />
              <ExportSection />
            </>
          )}
          {tab === 'customApi' && (
            <>
              {/* 远程服务器（高级）：自定义网关地址，一般无需修改 */}
              <RemoteServerCard />
              <section className="flex flex-col gap-3 rounded-md border border-border/60 p-4">
                <p className="text-xs text-muted-foreground">{t.setCustomApiDesc}</p>
                <CustomApiConfigForm />
              </section>
            </>
          )}
          {tab === 'about' && <AboutSection />}
        </div>
      </div>
    </div>
  )
}

export default function SettingsPage() {
  // useSearchParams 在静态导出下必须包 Suspense
  return (
    <Suspense>
      <SettingsPageInner />
    </Suspense>
  )
}
