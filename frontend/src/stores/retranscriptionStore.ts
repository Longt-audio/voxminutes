'use client'

// 离线（重）识别的全局状态。
//
// 为什么必须是全局的（2026-09-22 用户反馈：「离线识别过程中切换页面，识别就中断了」）：
// 后端 `start_retranscription_command` 是把任务 spawn 到 async runtime 的，切页面**不会**
// 真的中断它；但进度、增量结果、完成/失败事件过去全挂在 /history 页面的局部 state 与
// useEffect 监听里 —— 一旦离开该页面：
//   ① 监听被卸载 → 完成/失败事件无人接收（不刷新、不提示）；
//   ② 局部 state 归零 → 回到历史页看到的是「没有识别在进行」，进度条消失，
//      离线 tab 显示「暂无结果」，用户看到的就是「识别被中断了」；
//   ③ 再去点「重新识别」会被后端以 "already in progress" 拒绝，更加确信是坏了。
// 把状态与事件监听提升到全局（AppShell 挂载监听，本 store 存状态），切页面后
// 进度照常推进、完成后照常刷新与提示。

import { create } from 'zustand'
import type { RetranscriptionPartial, RetranscriptionProgress, RetranscriptionResult } from '@/types'

export interface RetranscriptionResultInfo {
  meetingId: string
  segmentsCount: number
  /** 音频时长（秒），由后端探测 */
  durationSeconds: number
  /** 识别耗时（秒） */
  elapsedSeconds: number | null
  /** 用户可见告警（上游内容风控部分拦截 / 档位降级 / 部分分片失败…） */
  warnings: string[]
}

interface RetranscriptionState {
  /** 正在识别的录音 id（null = 空闲）。后端一次只允许一个任务。 */
  meetingId: string | null
  /** 本次识别使用的离线 ASR 模型（显示名，用于进度提示） */
  modelId: string
  modelLabel: string
  progress: number | null
  message: string
  stage: string
  /** 已完成 chunk 的增量结果（仅当前任务） */
  partials: RetranscriptionPartial[]
  /** 已点「停止识别」、等待后端中断的过渡态 */
  stopping: boolean
  /** 完成一次识别 +1：历史页据此刷新列表与详情（跨页面也能收到） */
  completionTick: number
  /** 最近一次完成的结果信息（音频时长 / 识别耗时） */
  lastResult: RetranscriptionResultInfo | null

  start: (meetingId: string, modelId: string, modelLabel: string) => void
  setProgress: (p: RetranscriptionProgress) => void
  setPartial: (p: RetranscriptionPartial) => void
  finish: (r: RetranscriptionResult) => void
  conclude: () => void
  setStopping: (v: boolean) => void
}

export const useRetranscriptionStore = create<RetranscriptionState>((set) => ({
  meetingId: null,
  modelId: '',
  modelLabel: '',
  progress: null,
  message: '',
  stage: '',
  partials: [],
  stopping: false,
  completionTick: 0,
  lastResult: null,

  start: (meetingId, modelId, modelLabel) =>
    set({
      meetingId,
      modelId,
      modelLabel,
      progress: 0,
      message: '',
      stage: 'starting',
      partials: [],
      stopping: false,
    }),

  setProgress: (p) =>
    set((s) => {
      // 只接受当前任务的事件；进度单调不回退（心跳消息与阶段消息可能交错）
      if (s.meetingId && p.meeting_id !== s.meetingId) return s
      const progress = s.progress == null ? p.progress_percentage : Math.max(s.progress, p.progress_percentage)
      return { progress, message: p.message || s.message, stage: p.stage || s.stage }
    }),

  setPartial: (p) =>
    set((s) => {
      if (s.meetingId && p.meeting_id !== s.meetingId) return s
      const next = s.partials.filter((x) => x.chunk_index !== p.chunk_index)
      next.push(p)
      next.sort((a, b) => a.chunk_index - b.chunk_index)
      return { partials: next }
    }),

  finish: (r) =>
    set((s) => ({
      meetingId: null,
      progress: null,
      message: '',
      stage: '',
      partials: [],
      stopping: false,
      completionTick: s.completionTick + 1,
      lastResult: {
        meetingId: r.meeting_id,
        segmentsCount: r.segments_count,
        durationSeconds: r.duration_seconds,
        elapsedSeconds: r.elapsed_seconds ?? null,
        warnings: r.warnings ?? [],
      },
    })),

  conclude: () =>
    set({
      meetingId: null,
      progress: null,
      message: '',
      stage: '',
      partials: [],
      stopping: false,
    }),

  setStopping: (v) => set({ stopping: v }),
}))
