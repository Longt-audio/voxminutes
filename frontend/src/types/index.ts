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
  /** 流式引擎按连续静音分段的段落 id；同段内所有单元相同，VAD 引擎为 null/undefined */
  paragraph_id?: number | null
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
  paragraph_id?: number | null
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
  /** 段落最终译文（实时内嵌翻译；空串/缺省 = 无译文） */
  translation?: string
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

/** api_merge_recordings 返回的新合并工程元信息 */
export interface MergedRecordingResult {
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
  /** 用户可见告警（上游内容风控部分拦截 / 档位降级 / 部分分片失败…） */
  warnings?: string[]
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
  /** 生效地址（用户未自定义时回落内置默认 https://api.voxmin.top） */
  serverUrl: string
  /** 用户自定义地址（空串 = 未自定义 = 用默认） */
  customServerUrl: string
  /** 当前是否在用内置默认地址 */
  isDefault: boolean
  license: string
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

/** 翻译引擎：opus = OPUS-MT（快速），hymt2 = Hy-MT2（高质量），remote = 远程网关，custom-api = 自定义 LLM API（与会议总结共享 summary.api_config） */
export type TranslationEngine = 'opus' | 'hymt2' | 'remote' | 'custom-api'

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

/** tts_synthesize 命令返回：合成出的音频（base64）+ MIME 类型 */
export interface TtsSynthesisResult {
  audio_base64: string
  content_type: string
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
  /**
   * token = 正文增量；thinking = 思维链增量；done/error = 终止事件。
   *
   * thinking 为什么单独一类（2026-09-23）：开启「思考模式」的上游（DeepSeek / MiMo）
   * 会先吐十几秒思维链、然后才出正文。以前客户端**只读 content、把思维链整个丢掉**，
   * 于是那十几秒界面毫无输出，用户以为卡死。现在把思维链也上报，界面可显示"正在思考"。
   */
  kind: 'token' | 'thinking' | 'done' | 'error'
  text: string
  /** 仅 kind='done' 时有意义：true = 输出顶到 max_tokens（finish_reason=length）被截断，
   *  总结正文不完整（2026-09-27 新增；旧版后端不下发该字段，按 false 处理） */
  truncated?: boolean
}

/** summary_local_models 返回的本地总结模型项 */
export interface SummaryLocalModelInfo {
  id: string
  displayName: string
  installed: boolean
}

// ── 模型加载 ──────────────────────────────────────────────────────────────────

/** model-loading 事件 payload：模型（ASR / OPUS-MT / Hy-MT2 / 总结 GGUF）实际加载的开始/完成/失败/卸载 */
export interface ModelLoadingEvent {
  /** 模型标识（如 x-asr-480ms、sense-voice、opus-mt、GGUF 文件 stem） */
  model: string
  phase: 'start' | 'done' | 'error' | 'unloaded'
  /** phase 为 done 时的加载耗时（毫秒） */
  elapsed_ms?: number
  /** phase 为 error 时的错误信息；phase 为 unloaded 时的原因（manual/swap/idle） */
  message?: string
}
