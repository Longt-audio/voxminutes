// VoxMinutes MVP 类型定义 —— 与 Rust 后端 payload 对齐（snake_case）

// ── 实时转录 ──────────────────────────────────────────────────────────────────

export interface TranscriptSegment {
  id: string
  text: string
  timestamp: string
  sequence_id: number
  chunk_start_time: number
  is_partial: boolean
  confidence: number
  audio_start_time: number
  audio_end_time: number
  duration: number
  source: string
}

/** transcript-update 事件 payload（音频时间单位为秒） */
export interface TranscriptUpdate {
  text: string
  timestamp: string
  source: string
  sequence_id: number
  chunk_start_time: number
  is_partial: boolean
  confidence: number
  audio_start_time: number
  audio_end_time: number
  duration: number
}

// ── 音频设备 ──────────────────────────────────────────────────────────────────

export interface AudioDevice {
  name: string
  device_type: 'Input' | 'Output'
}

export interface DefaultDevicesInfo {
  microphone: string | null
  speaker: string | null
}

export interface RecordingPreferences {
  recordingsFolder: string
  autoSave: boolean
  defaultAsrModel?: string
}

// ── ASR 模型 ──────────────────────────────────────────────────────────────────

/** sherpa_onnx_get_models 返回的模型信息（serde_json 原样 snake_case） */
export interface ModelInfo {
  name: string
  status: string // 'Available' | 'Loaded' | 'Missing' | 'NotConfigured'
  size_mb?: number
  languages?: string[]
  architecture?: string
  description?: string
  has_punctuation?: boolean
  has_timestamps?: boolean
  has_hotwords?: boolean
  is_remote?: boolean
  hidden?: boolean
}

/** get_downloadable_models 返回的单个下载源 */
export interface ModelSourceInfo {
  label: string
  urls: string[]
}

/** get_downloadable_models 返回 */
export interface DownloadableModelInfo {
  id: string
  display_name: string
  installed: boolean
  downloading: boolean
  size_bytes: number
  /** 全部下载源（官方在前，镜像在后） */
  sources: ModelSourceInfo[]
}

/** model-download-progress 事件 payload */
export interface ModelDownloadProgress {
  modelId: string
  stage: 'downloading' | 'extracting' | 'verifying' | 'done' | 'error' | 'cancelled'
  downloadedBytes: number
  totalBytes: number
  percent: number
  message?: string | null
  /** 下载来源 URL（可选，后端新增字段） */
  sourceUrl?: string | null
}

/** import_model_file 命令返回 */
export interface ImportModelResult {
  status: 'done' | 'cancelled' | 'error'
  message?: string | null
}

// ── 历史记录（recordings） ────────────────────────────────────────────────────

export interface RecordingListItem {
  id: string
  title: string
  created_at: string
  updated_at: string
  folder_path?: string | null
}

export interface RecordingSegment {
  id: string
  text: string
  start_ms: number
  end_ms?: number | null
  speaker?: string | null
  source?: string | null
}

export interface RecordingDetails {
  id: string
  title: string
  created_at: string
  updated_at: string
  duration_ms?: number | null
  audio_path?: string | null
  folder_path?: string | null
  source?: string | null
  asr_engine?: string | null
  language?: string | null
  status?: string | null
  segments: RecordingSegment[]
}

export interface PaginatedSegmentsResponse {
  segments: RecordingSegment[]
  total_count: number
  has_more: boolean
}

export interface SearchTranscriptResult {
  id: string
  recording_id: string
  title: string
  text: string
  start_ms: number
}

// ── 导入 / 重新转写 ───────────────────────────────────────────────────────────

export interface AudioFileInfo {
  path: string
  filename: string
  duration_seconds: number
  size_bytes: number
  format: string
}

export interface ImportProgress {
  stage: string
  progress_percentage: number
  message: string
  elapsed_seconds?: number
  estimated_remaining_seconds?: number
  chunks_total?: number
  chunks_processed?: number
}

export interface ImportResult {
  meeting_id: string
  title: string
  segments_count: number
  duration_seconds: number
}

export interface ImportError {
  error: string
}

export interface ImportWarning {
  warning: string
  details?: string
}

export interface RetranscriptionProgress {
  meeting_id: string
  stage: string
  progress_percentage: number
  message: string
  elapsed_seconds?: number
  estimated_remaining_seconds?: number
  chunks_total?: number
  chunks_processed?: number
}

export interface RetranscriptionResult {
  meeting_id: string
  segments_count: number
  duration_seconds: number
  language?: string
  elapsed_seconds?: number
}

export interface RetranscriptionError {
  meeting_id: string
  error: string
}

/** retranscription-partial 事件 payload：单个音频块的增量识别结果 */
export interface RetranscriptionPartial {
  meeting_id: string
  chunk_index: number
  chunks_total: number
  text: string
  start_ms: number
  end_ms: number
}

// ── 远程 ASR（预留接口，MVP 不实现） ──────────────────────────────────────────

export interface RemoteAsrConfig {
  endpoint: string
  model: string
  configured: boolean
}

// ── 翻译 ──────────────────────────────────────────────────────────────────────

export type TranslationDirection = 'auto' | 'zh-en' | 'en-zh'

/**
 * 实时翻译目标语言（home⇄target 互译的 target 一侧，不含 auto）。
 * 合法取值由当前翻译引擎决定：opus 仅支持 zh/en；
 * hymt2 额外支持 ja ko fr de es ru pt zh-Hant yue th vi（见 lib/translateTargetLangs.ts）。
 */
export type TranslateTargetLang = string

/** 翻译引擎：opus = OPUS-MT（快速），hymt2 = Hy-MT2（高质量） */
export type TranslationEngine = 'opus' | 'hymt2'

/** translate-update 事件 payload */
export interface TranslateUpdate {
  sequence_id: number
  original_text: string
  translated_text: string
  source_lang: string
  target_lang: string
  is_partial: boolean
}

/** translate-text-stream 事件 payload（translate_text 带 requestId 时的流式增量） */
export interface TranslateTextStreamEvent {
  request_id: string
  delta: string
}

// ── 音频电平 / 频谱监听 ──────────────────────────────────────────────────────

export interface AudioLevelData {
  device_name: string
  device_type: string
  rms_level: number
  peak_level: number
  is_active: boolean
  spectrum: number[]
  samples: number[]
}

export interface AudioLevelUpdate {
  timestamp: number
  levels: AudioLevelData[]
}

// ── 会议总结 ──────────────────────────────────────────────────────────────────

/** summary_get_config / summary_save_config 使用的 API 配置 */
export interface SummaryApiConfig {
  protocol: string
  endpoint: string
  apiKey: string
  model: string
}

/** summary-stream 事件 payload（每个请求必有一个 done/error 终止事件） */
export interface SummaryStreamEvent {
  requestId: string
  kind: 'token' | 'done' | 'error'
  text: string
}

/** summary_local_models 返回的本地总结模型项 */
export interface SummaryLocalModelInfo {
  id: string
  displayName: string
  installed: boolean
}

// ── 模型加载 ──────────────────────────────────────────────────────────────────

/** model-loading 事件 payload：模型（ASR / OPUS-MT / Hy-MT2 / 总结 GGUF）实际加载的开始/完成/失败 */
export interface ModelLoadingEvent {
  /** 模型标识（如 x-asr-480ms、sense-voice、opus-mt-zh-en、GGUF 文件 stem） */
  model: string
  phase: 'start' | 'done' | 'error'
  /** phase 为 done 时的加载耗时（毫秒） */
  elapsed_ms?: number
  /** phase 为 error 时的错误信息 */
  message?: string
}
