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
  /** seq_id → 译文（实时内嵌翻译，最终版） */
  translations: Map<number, string>
  /** seq_id → 流式中的部分译文快照（最终版到达后清除） */
  partialTranslations: Map<number, string>
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
  setLatestRecordingId: (id: string | null) => void
  setAudioSpectrum: (v: number[]) => void
  setAudioActive: (v: boolean) => void
  setAudioLevels: (v: { mic: number; system: number }) => void
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
  translations: new Map<number, string>(),
  partialTranslations: new Map<number, string>(),
  translateEnabled: false,
  translateTargetLang: 'zh' as TranslateTargetLang,
  translationEngine: 'opus' as TranslationEngine,
}

export const useAppStore = create<AppState>()((set) => ({
  ...initialState,

  setRecording: (v) => set({ isRecording: v }),
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
      return { transcripts: next }
    }),

  clearTranscripts: () => set({ transcripts: [], translations: new Map(), partialTranslations: new Map() }),
  setModels: (m) => set({ models: m }),
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
