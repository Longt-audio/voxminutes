'use client'

import { RecorderControls, RecorderInfo } from '@/components/recorder/RecorderPanel'
import { TranscriptPanel } from '@/components/recorder/TranscriptPanel'

export default function HomePage() {
  return (
    <div className="grid grid-cols-[auto_1fr] h-full bg-background">
      {/* 左栏：录音控制 */}
      <div className="px-5 pt-8 pb-2 overflow-y-auto custom-scrollbar">
        <RecorderControls />
      </div>

      {/* 右栏：信息行 + 转录文本 */}
      <div className="flex flex-col min-h-0 pr-5 pb-5">
        <div className="shrink-0 pt-8 pb-3">
          <RecorderInfo />
        </div>
        <TranscriptPanel />
      </div>
    </div>
  )
}
