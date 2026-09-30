'use client'

import { RecorderControls, RecorderInfo } from '@/components/recorder/RecorderPanel'
import { TranscriptPanel } from '@/components/recorder/TranscriptPanel'
import { FloatingBallToggle } from '@/components/FloatingBallToggle'

export default function HomePage() {
  return (
    <div className="grid grid-cols-[auto_1fr] h-full bg-background">
      {/* 左栏：录音控制 */}
      <div className="px-5 pt-8 pb-2 overflow-y-auto custom-scrollbar">
        <RecorderControls />
      </div>

      {/* 右栏：信息行（含悬浮球开关）+ 转录文本 */}
      <div className="flex flex-col min-h-0 pr-5 pb-5">
        <div className="shrink-0 pt-8 pb-3 flex items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <RecorderInfo />
          </div>
          <FloatingBallToggle />
        </div>
        <TranscriptPanel />
      </div>
    </div>
  )
}
