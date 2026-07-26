'use client'

import { useEffect, useRef } from 'react'
import { useAppStore } from '@/state'
import { cn } from '@/lib/utils'

const SEG_H = 4 // 方块高（px）
const SEG_GAP = 1 // 方块间距（px）
const BAR_GAP = 3 // 柱间距（px）

/**
 * PotPlayer 风格方块堆叠频谱：读取 store 里的真实频谱，canvas 逐帧绘制。
 * 主红色系：底部暗红 → 顶部亮红（Streamlit 红）。
 */
export function AudioSpectrumBars({ className = '' }: { className?: string }) {
  const audioSpectrum = useAppStore((s) => s.audioSpectrum)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const spectrumRef = useRef<number[]>([])

  useEffect(() => {
    spectrumRef.current = audioSpectrum
  }, [audioSpectrum])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let raf = 0
    const draw = () => {
      const w = Math.round(canvas.clientWidth)
      const h = Math.round(canvas.clientHeight)
      if (w > 0 && (canvas.width !== w || canvas.height !== h)) {
        canvas.width = w
        canvas.height = h
      }
      ctx.clearRect(0, 0, canvas.width, canvas.height)

      const spectrum = spectrumRef.current
      const bands = spectrum.length || 48
      const barW = Math.max(2, Math.floor((canvas.width - (bands - 1) * BAR_GAP) / bands))
      const maxSegs = Math.max(1, Math.floor((canvas.height + SEG_GAP) / (SEG_H + SEG_GAP)))

      for (let b = 0; b < bands; b++) {
        const v = Math.max(0, Math.min(1, spectrum[b] ?? 0))
        // 静默时保留 1 格暗块作基线
        const litSegs = Math.max(v > 0.02 ? 1 : 0, Math.round(v * maxSegs))
        const x = b * (barW + BAR_GAP)
        for (let s = 0; s < maxSegs; s++) {
          const y = canvas.height - (s + 1) * (SEG_H + SEG_GAP) + SEG_GAP
          if (y < 0) break
          if (s < litSegs) {
            const t = maxSegs > 1 ? s / (maxSegs - 1) : 0 // 0=底部 1=顶部
            const light = 42 + t * 22.7 // 42% → 64.7%（Streamlit 红）
            ctx.fillStyle = `hsl(0, ${78 + t * 22}%, ${light}%)`
            ctx.globalAlpha = 1
          } else {
            ctx.fillStyle = 'hsl(0, 0%, 50%)'
            ctx.globalAlpha = 0.12
          }
          ctx.fillRect(x, y, barW, SEG_H)
        }
      }
      ctx.globalAlpha = 1
      raf = requestAnimationFrame(draw)
    }
    raf = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(raf)
  }, [])

  return <canvas ref={canvasRef} className={cn('block h-full w-full', className)} />
}
