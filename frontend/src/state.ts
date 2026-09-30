import { create } from 'zustand'
import type { TranscriptSegment, ModelInfo, DefaultDevicesInfo, TranslateTargetLang, TranslationEngine } from '@/types'

interface AppState {
  isRecording: boolean
  isPaused: boolean
  isProcessing: boolean
  transcripts: TranscriptSegment[]
  models: ModelInfo[]
  selectedModel: string
  defaultDevices: DefaultDevicesInfo
  meetingName: string | null
  meetingFolderPath: string | null
  asrModelStatus: 'idle' | 'loading' | 'loaded' | 'error'
  recordingDuration: number
  isMicMuted: boolean
  latestRecordingId: string | null
  audioSpectrum: number[]
  audioActive: boolean
  audioLevels: { mic: number; system: number }
  /** VAD 检测到人声但尚未断句（用于「正在识别」提示） */
  vadSpeaking: boolean
  /**
   * 识别链路终止原因（2026-09-29）：null = 正常（未录音 / 识别在跑）。
   * 远程流式 ASR 终止时由 Rust 事件 `transcription-status` 写入：
   * 'credits'（积分不足）| 'config'（鉴权/配置）| 'unavailable'（重连耗尽）| 'ended'（正常结束，不提示）。
   * 界面只用来把「● 实时转写中」改成「▲ 识别已停止」，不影响录音本身。
   */
  asrStopReason: 'credits' | 'config' | 'unavailable' | 'ended' | null
  /** seq_id → 译文（实时内嵌翻译，最终版） */
  translations: Map<number, string>
  /** seq_id → 流式中的部分译文快照（最终版到达后清除） */
  partialTranslations: Map<number, string>
  /** paragraph_id → 该段首个单元首次到达的墙钟时刻（Date.now()），段首时间显示/落库基准用 */
  paragraphStarts: Record<number, number>
  /** paragraph_id → 该段单元最近一次到达的墙钟时刻（落库 end_ms 用） */
  paragraphEnds: Record<number, number>
  translateEnabled: boolean
  translateTargetLang: TranslateTargetLang
  translationEngine: TranslationEngine

  setRecording: (v: boolean) => void
  setPaused: (v: boolean) => void
  setProcessing: (v: boolean) => void
  addTranscript: (seg: TranscriptSegment) => void
  clearTranscripts: () => void
  setModels: (m: ModelInfo[]) => void
  setSelectedModel: (m: string) => void
  setDefaultDevices: (d: DefaultDevicesInfo) => void
  setMeetingName: (n: string | null) => void
  setMeetingFolderPath: (p: string | null) => void
  setAsrModelStatus: (s: 'idle' | 'loading' | 'loaded' | 'error') => void
  setRecordingDuration: (s: number) => void
  setMicMuted: (v: boolean) => void
  setAsrStopReason: (r: 'credits' | 'config' | 'unavailable' | 'ended' | null) => void
  setLatestRecordingId: (id: string | null) => void
  setAudioSpectrum: (v: number[]) => void
  setAudioActive: (v: boolean) => void
  setAudioLevels: (v: { mic: number; system: number }) => void
  setVadSpeaking: (v: boolean) => void
  addTranslation: (seqId: number, text: string) => void
  addPartialTranslation: (seqId: number, text: string) => void
  setTranslateEnabled: (v: boolean) => void
  setTranslateTargetLang: (lang: TranslateTargetLang) => void
  setTranslationEngine: (engine: TranslationEngine) => void
  reset: () => void
}

const initialState = {
  isRecording: false,
  isPaused: false,
  isProcessing: false,
  transcripts: [] as TranscriptSegment[],
  models: [] as ModelInfo[],
  selectedModel: '',
  defaultDevices: { microphone: null, speaker: null } as DefaultDevicesInfo,
  meetingName: null as string | null,
  meetingFolderPath: null as string | null,
  asrModelStatus: 'idle' as const,
  recordingDuration: 0,
  isMicMuted: false,
  latestRecordingId: null as string | null,
  audioSpectrum: [] as number[],
  audioActive: false,
  audioLevels: { mic: 0, system: 0 },
  vadSpeaking: false,
  asrStopReason: null,
  translations: new Map<number, string>(),
  partialTranslations: new Map<number, string>(),
  paragraphStarts: {} as Record<number, number>,
  paragraphEnds: {} as Record<number, number>,
  translateEnabled: false,
  translateTargetLang: 'zh' as TranslateTargetLang,
  translationEngine: 'opus' as TranslationEngine,
}

export const useAppStore = create<AppState>()((set) => ({
  ...initialState,

  setRecording: (v) => set({ isRecording: v }),
  // 开始/停止录音时复位识别状态：新一场录音不该背着上一场的「识别已停止」
  setAsrStopReason: (r) => set({ asrStopReason: r }),
  setPaused: (v) => set({ isPaused: v }),
  setProcessing: (v) => set({ isProcessing: v }),

  addTranscript: (seg) =>
    set((state) => {
      // 相同 sequence_id 的段落原位替换（流式 partial → final），保留 React key
      const idx = state.transcripts.findIndex((t) => t.sequence_id === seg.sequence_id)
      let next: TranscriptSegment[]
      if (idx >= 0) {
        next = [...state.transcripts]
        next[idx] = { ...seg, id: state.transcripts[idx].id }
      } else {
        next = [...state.transcripts, seg]
      }
      next.sort((a, b) => a.sequence_id - b.sequence_id)
      // 流式段落：记录该段首次/最近到达的墙钟时刻（段首时间显示与落库时间基准）
      if (seg.paragraph_id == null) return { transcripts: next }
      const now = Date.now()
      const paragraphStarts = { ...state.paragraphStarts }
      if (paragraphStarts[seg.paragraph_id] == null) paragraphStarts[seg.paragraph_id] = now
      return {
        transcripts: next,
        paragraphStarts,
        paragraphEnds: { ...state.paragraphEnds, [seg.paragraph_id]: now },
      }
    }),

  clearTranscripts: () =>
    set({
      transcripts: [],
      translations: new Map(),
      partialTranslations: new Map(),
      paragraphStarts: {},
      paragraphEnds: {},
    }),
  // 防御：models 会被 RecorderPanel 直接 `.filter()`（无空值保护），
  // 一旦 IPC 返回 null（命令失败/字段缺失），首页就会整页崩掉。
  // 这里统一收敛成数组，别把 null 存进 store。
  setModels: (m) => set({ models: Array.isArray(m) ? m : [] }),
  setSelectedModel: (m) => set({ selectedModel: m }),
  setDefaultDevices: (d) => set({ defaultDevices: d }),
  setMeetingName: (n) => set({ meetingName: n }),
  setMeetingFolderPath: (p) => set({ meetingFolderPath: p }),
  setAsrModelStatus: (s) => set({ asrModelStatus: s }),
  setRecordingDuration: (s) => set({ recordingDuration: s }),
  setMicMuted: (v) => set({ isMicMuted: v }),
  setLatestRecordingId: (id) => set({ latestRecordingId: id }),
  setAudioSpectrum: (v) => set({ audioSpectrum: v }),
  setAudioActive: (v) => set({ audioActive: v }),
  setAudioLevels: (v) => set({ audioLevels: v }),
  setVadSpeaking: (v) => set({ vadSpeaking: v }),

  addTranslation: (seqId, text) =>
    set((state) => {
      const next = new Map(state.translations)
      next.set(seqId, text)
      // 最终版到达，清掉同 seq 的部分译文
      const nextPartial = new Map(state.partialTranslations)
      nextPartial.delete(seqId)
      return { translations: next, partialTranslations: nextPartial }
    }),
  addPartialTranslation: (seqId, text) =>
    set((state) => {
      const next = new Map(state.partialTranslations)
      next.set(seqId, text)
      return { partialTranslations: next }
    }),
  setTranslateEnabled: (v) => set({ translateEnabled: v }),
  setTranslateTargetLang: (lang) => set({ translateTargetLang: lang }),
  setTranslationEngine: (engine) => set({ translationEngine: engine }),

  reset: () => set(initialState),
}))
