'use client'

import { useEffect, useRef } from 'react'
import { cn } from '@/lib/utils'

const MAX_BUF = 80
const WINDOW_MS = 5000

/** 线性电平 → dB */
function levelToDb(level: number): number {
  if (level <= 0.00001) return -60
  return 20 * Math.log10(level)
}

/** dB → 可视高度：-48dB(静音)→0%，-6dB(大音量)→100%（收窄窗口让正常语音占据大部分高度） */
function dbToVisual(db: number): number {
  return Math.max(0, Math.min(1, (db + 48) / 42))
}

function getColor(db: number): string {
  if (db < -36) return '#10b981'
  if (db < -24) return '#84cc16'
  if (db < -12) return '#f59e0b'
  if (db < -6) return '#f97316'
  return '#ef4444'
}

interface AudioWaveformBarProps {
  /** 平滑后的峰值电平（0~1） */
  peak: number
  /** 是否有活跃音频输入 */
  active: boolean
  className?: string
}

/** 音频能量条（画布绘制滚动波形，自检弹窗用） */
export function AudioWaveformBar({ peak, active, className = '' }: AudioWaveformBarProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const bufferRef = useRef<Array<{ t: number; peak: number; active: boolean }>>([])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    bufferRef.current.push({ t: performance.now(), peak, active })
    if (bufferRef.current.length > MAX_BUF) {
      bufferRef.current = bufferRef.current.slice(-MAX_BUF)
    }

    // 无输入且近期峰值近零时跳过绘制
    const recent = bufferRef.current.slice(-10)
    if (!active && recent.every((e) => e.peak < 0.001)) {
      ctx.clearRect(0, 0, canvas.width, canvas.height)
      return
    }

    // 画布分辨率对齐 CSS 尺寸
    const clientW = Math.round(canvas.clientWidth)
    const clientH = Math.round(canvas.clientHeight)
    if (clientW > 0 && (canvas.width !== clientW || canvas.height !== clientH)) {
      canvas.width = clientW
      canvas.height = clientH
    }

    const w = canvas.width
    const h = canvas.height
    const now = performance.now()
    const cutoff = now - WINDOW_MS
    const cy = h / 2
    const halfH = cy - 4

    ctx.clearRect(0, 0, w, h)

    // 中线虚线
    ctx.strokeStyle = 'rgba(128,128,128,0.15)'
    ctx.setLineDash([2, 4])
    ctx.beginPath()
    ctx.moveTo(0, cy)
    ctx.lineTo(w, cy)
    ctx.stroke()
    ctx.setLineDash([])

    for (const e of bufferRef.current) {
      if (e.t < cutoff) continue
      const x = ((e.t - cutoff) / WINDOW_MS) * w
      const db = levelToDb(e.peak)
      const visualLevel = dbToVisual(db)
      const barH = Math.max(1.5, visualLevel * halfH)
      ctx.fillStyle = getColor(db)
      ctx.globalAlpha = e.active ? 1.0 : 0.35
      ctx.fillRect(x - 1.5, cy - barH, 3, barH * 2)
    }
    ctx.globalAlpha = 1
  }, [peak, active])

  return (
    <canvas
      ref={canvasRef}
      className={cn('h-full w-full', className)}
      width={400}
      height={52}
    />
  )
}
