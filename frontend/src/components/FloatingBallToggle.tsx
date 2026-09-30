'use client'

import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'
import { toggleFloatingBall, getFloatingBallState, onFloatingBallState } from '@/services/ipc'

/** 悬浮球开关：小圆点 + 文字，点亮表示悬浮球当前可见（启动默认显示）。
 *  放在实时转录页信息行右侧（顶栏已拥挤，见 AppShell 历史）。 */
export function FloatingBallToggle() {
  const [on, setOn] = useState(true)
  const t = useMessages()

  // 初始同步一次真实可见性，并监听 Rust 侧广播（悬浮球菜单「隐藏」/窗口销毁时联动熄灭）
  useEffect(() => {
    getFloatingBallState().then(setOn).catch(() => {})
    let unlisten: (() => void) | undefined
    onFloatingBallState((payload) => setOn(payload.visible))
      .then((fn) => { unlisten = fn })
      .catch(() => {})
    return () => { unlisten?.() }
  }, [])

  const toggle = async () => {
    try {
      setOn(await toggleFloatingBall())
    } catch {
      // ignore
    }
  }

  return (
    <Button
      variant="ghost"
      size="sm"
      className="h-8 px-2 gap-1.5 text-xs shrink-0"
      onClick={toggle}
      title={on ? t.floatingBallHide : t.floatingBallShow}
    >
      <span
        className={`inline-block w-2.5 h-2.5 rounded-full transition-colors ${
          on ? 'bg-primary' : 'border border-muted-foreground/50'
        }`}
      />
      {t.floatingBall}
    </Button>
  )
}
