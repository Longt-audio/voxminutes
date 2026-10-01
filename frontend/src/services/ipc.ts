// VoxMinutes MVP IPC 层 —— 只包含 MVP 后端命令
import { invoke } from '@tauri-apps/api/core'
import { listen as tauriListen, UnlistenFn } from '@tauri-apps/api/event'
import type {
  TranscriptSegment,
  TranscriptUpdate,
  AudioDevice,
  DefaultDevicesInfo,
  ModelInfo,
  DownloadableModelInfo,
  ModelDownloadProgress,
  RecordingListItem,
  RecordingDetails,
  RecordingSegment,
  PaginatedSegmentsResponse,
  SearchTranscriptResult,
  RecordingPreferences,
  AudioFileInfo,
  ImportProgress,
  ImportResult,
  ImportError,
  ImportWarning,
  RetranscriptionProgress,
  RetranscriptionResult,
  RetranscriptionError,
  RetranscriptionPartial,
  RemoteAsrConfig,
  AudioLevelUpdate,
  TranslateUpdate,
  TranslateTextStreamEvent,
  TranslationDirection,
  TranslateTargetLang,
  TranslationEngine,
  SummaryApiConfig,
  SummaryStreamEvent,
  SummaryLocalModelInfo,
  ModelLoadingEvent,
  ImportModelResult,
  TtsSynthesisResult,
  MergedRecordingResult,
} from '@/types'

// ── 事件监听安全包装 ──────────────────────────────────────────────────────────
// Tauri 的 listen 返回的 unlisten 是 async 函数，重复调用（React StrictMode 双
// 挂载 / 组件重渲染清理竞态）会抛 `listeners[eventId].handlerId` 未定义错误。
// 这里包一层：unlisten 只执行一次，且吞掉同步异常与异步 rejection。

import type { EventName, EventCallback, Options } from '@tauri-apps/api/event'

function listen<T>(
  event: EventName,
  handler: EventCallback<T>,
  options?: Options,
): Promise<UnlistenFn> {
  return tauriListen<T>(event, handler, options).then((raw) => {
    let called = false
    return () => {
      if (called) return
      called = true
      try {
        const r = raw() as unknown
        if (r && typeof (r as Promise<void>).catch === 'function') {
          ;(r as Promise<void>).catch(() => {})
        }
      } catch {
        // 重复 unlisten：忽略
      }
    }
  })
}

// ── 录音控制 ──────────────────────────────────────────────────────────────────

export async function startRecording(
  meetingName: string,
  micDeviceName?: string | null,
  systemDeviceName?: string | null
): Promise<void> {
  return invoke('start_recording', {
    micDeviceName: micDeviceName ?? null,
    systemDeviceName: systemDeviceName ?? null,
    meetingName,
  })
}

export async function stopRecording(savePath: string): Promise<void> {
  return invoke('stop_recording', { args: { save_path: savePath } })
}

export async function pauseRecording(): Promise<void> {
  return invoke('pause_recording')
}

export async function resumeRecording(): Promise<void> {
  return invoke('resume_recording')
}

export async function isRecording(): Promise<boolean> {
  return invoke<boolean>('is_recording')
}

export async function getRecordingState(): Promise<{
  is_recording: boolean
  is_paused: boolean
  is_active: boolean
  is_waiting_for_device?: boolean
  recording_duration: number | null
  active_duration: number | null
}> {
  return invoke('get_recording_state')
}

export async function getMeetingFolderPath(): Promise<string | null> {
  return invoke<string | null>('get_meeting_folder_path')
}

export async function getRecordingMeetingName(): Promise<string | null> {
  return invoke<string | null>('get_recording_meeting_name')
}

export async function setMicMute(enabled: boolean): Promise<boolean> {
  return invoke<boolean>('set_mic_mute', { enabled })
}

export async function getMicMute(): Promise<boolean> {
  return invoke<boolean>('get_mic_mute')
}

export async function toggleMicMute(): Promise<boolean> {
  return invoke<boolean>('toggle_mic_mute')
}

// ── 音频设备 ──────────────────────────────────────────────────────────────────

export async function listAudioDevices(): Promise<AudioDevice[]> {
  return invoke<AudioDevice[]>('get_audio_devices')
}

export async function getDefaultAudioDevices(): Promise<DefaultDevicesInfo> {
  return invoke<DefaultDevicesInfo>('get_default_audio_devices')
}

export async function openSystemSoundSettings(): Promise<void> {
  return invoke('open_system_sound_settings')
}

export async function triggerMicrophonePermission(): Promise<boolean> {
  return invoke<boolean>('trigger_microphone_permission')
}

// ── ASR 模型 ──────────────────────────────────────────────────────────────────

export async function sherpaOnnxGetModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>('sherpa_onnx_get_models')
}

export async function sherpaOnnxLoadModel(modelName: string): Promise<void> {
  return invoke('sherpa_onnx_load_model', { modelName })
}

export async function sherpaOnnxIsModelLoaded(): Promise<boolean> {
  return invoke<boolean>('sherpa_onnx_is_model_loaded')
}

export async function sherpaOnnxGetCurrentModel(): Promise<string | null> {
  return invoke<string | null>('sherpa_onnx_get_current_model')
}

export async function sherpaOnnxGetModelsDirectory(): Promise<string> {
  return invoke<string>('sherpa_onnx_get_models_directory')
}

/** 修改模型存放目录，立即生效并返回生效路径。 */
export async function setModelsDirectoryCustom(dir: string): Promise<string> {
  return invoke<string>('set_models_directory_custom', { dir })
}

// ── 模型下载 ──────────────────────────────────────────────────────────────────

export async function getDownloadableModels(): Promise<DownloadableModelInfo[]> {
  return invoke<DownloadableModelInfo[]>('get_downloadable_models')
}

export async function downloadModel(modelId: string, sourceIndex?: number): Promise<void> {
  return invoke('download_model', { modelId, sourceIndex: sourceIndex ?? null })
}

export async function cancelModelDownload(modelId: string): Promise<void> {
  return invoke('cancel_model_download', { modelId })
}

export async function deleteModel(modelId: string): Promise<void> {
  return invoke('delete_model', { modelId })
}

/** 一键清除所有模型后台（ASR/翻译/总结 LLM），释放内存。 */
export async function clearAllModelBackends(): Promise<void> {
  return invoke('clear_all_model_backends')
}

/** 从本地文件/文件夹导入模型；后端弹原生选择框，进度走 model-download-progress 事件。 */
export async function importModelFile(modelId: string): Promise<ImportModelResult> {
  return invoke<ImportModelResult>('import_model_file', { modelId })
}

/** 导入一个已解压好的模型文件夹（任意模型类型）。 */
export async function importModelFolder(modelId: string): Promise<ImportModelResult> {
  return invoke<ImportModelResult>('import_model_folder', { modelId })
}

export function onModelDownloadProgress(
  callback: (progress: ModelDownloadProgress) => void
): Promise<UnlistenFn> {
  return listen<ModelDownloadProgress>('model-download-progress', (event) => callback(event.payload))
}

/** 监听模型实际加载的开始/完成/失败（model-loading 事件）。 */
export function onModelLoading(callback: (e: ModelLoadingEvent) => void): Promise<UnlistenFn> {
  return listen<ModelLoadingEvent>('model-loading', (event) => callback(event.payload))
}

// ── 历史记录 ──────────────────────────────────────────────────────────────────

export async function apiGetRecordings(): Promise<RecordingListItem[]> {
  return invoke<RecordingListItem[]>('api_get_recordings', { authToken: null })
}

export async function apiGetRecording(recordingId: string): Promise<RecordingDetails> {
  return invoke<RecordingDetails>('api_get_recording', { recordingId, authToken: null })
}

export async function apiGetRecordingSegments(
  recordingId: string,
  limit = 500,
  offset = 0,
  source?: string
): Promise<PaginatedSegmentsResponse> {
  return invoke<PaginatedSegmentsResponse>('api_get_recording_segments', {
    recordingId,
    limit,
    offset,
    source: source ?? null,
  })
}

export async function apiDeleteRecording(recordingId: string): Promise<void> {
  return invoke('api_delete_recording', { recordingId, authToken: null })
}

/**
 * 合并多个录音工程：音频拼接 + 转写段按时间偏移衔接，生成一个新工程。
 * markerTemplate 为交界标记段模板（占位符 {n} {title} {time}），由 i18n 提供。
 */
export async function apiMergeRecordings(
  recordingIds: string[],
  title: string | null,
  deleteSources: boolean,
  markerTemplate?: string | null
): Promise<MergedRecordingResult> {
  return invoke<MergedRecordingResult>('api_merge_recordings', {
    recordingIds,
    title,
    deleteSources,
    markerTemplate: markerTemplate ?? null,
  })
}

export async function apiSaveRecordingTitle(recordingId: string, title: string): Promise<void> {
  return invoke('api_save_recording_title', { recordingId, title, authToken: null })
}

/** 保存一次录音的转录结果（停止录音后调用）。start_ms/end_ms 单位为毫秒。 */
export async function apiSaveTranscript(
  recordingTitle: string,
  segments: Array<{
    id: string
    text: string
    timestamp?: string
    start_ms?: number
    end_ms?: number
    duration?: number
    speaker?: string
    source?: string
    translation?: string
  }>,
  folderPath?: string | null
): Promise<{ status: string; message: string; recording_id: string }> {
  return invoke('api_save_transcript', {
    recordingTitle,
    segments,
    folderPath: folderPath ?? null,
    authToken: null,
  })
}

export async function apiSearchTranscripts(query: string): Promise<{ results: SearchTranscriptResult[] }> {
  return invoke('api_search_transcripts', { query })
}

export async function apiExportRecording(
  recordingId: string,
  format: 'txt' | 'srt' | 'markdown',
  source?: 'realtime' | 'offline_asr' | null,
  outputDir?: string | null
): Promise<{ status: string; path: string }> {
  return invoke('api_export_recording', { recordingId, format, source: source ?? null, outputDir: outputDir ?? null })
}

export async function apiUpdateSegmentText(segmentId: string, text: string): Promise<void> {
  return invoke('api_update_segment_text', { segmentId, text })
}

/** 读会议文件夹的说话人命名表（{speakerId: 自定义名}）。 */
export async function getSpeakerNames(folderPath: string): Promise<Record<string, string>> {
  return invoke('api_get_speaker_names', { folderPath })
}

/** 读会议文件夹 metadata.json 里记录的离线识别模型。 */
export async function getRetranscribedModel(folderPath: string): Promise<string> {
  return invoke('api_get_retranscribed_model', { folderPath })
}

/** 读会议文件夹 metadata.json 里的离线识别摘要（模型 / 音频时长 / 识别耗时）。 */
export async function getOfflineRecognitionInfo(folderPath: string): Promise<{
  model?: string
  duration_seconds?: number
  elapsed_seconds?: number
  retranscribed_at?: string
  /** 上游告警（内容风控部分拦截 / 档位降级 / 部分分片失败） */
  warnings?: string[]
}> {
  return invoke('api_get_offline_recognition_info', { folderPath })
}

/** 保存某个说话人的自定义名（空名 = 恢复默认「说话人N」）。 */
export async function setSpeakerName(folderPath: string, speakerId: string, name: string): Promise<void> {
  return invoke('api_set_speaker_name', { folderPath, speakerId, name })
}

export async function openRecordingFolder(recordingId: string): Promise<void> {
  return invoke('open_recording_folder', { recordingId })
}

// ── 设置 ──────────────────────────────────────────────────────────────────────

export async function apiGetSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('api_get_settings')
}

export async function apiSaveSetting(key: string, value: string | null): Promise<void> {
  return invoke('api_save_setting', { key, value })
}

export async function apiGetTranscriptConfig(): Promise<{ provider: string; model: string; api_key?: string | null } | null> {
  return invoke('api_get_transcript_config', { authToken: null })
}

/** 保存转写配置。远程（provider=remote-qwen3-asr）时必须把当前真实远程模型 id
 *  一并传给后端（remoteAsrModel）——后端以此为权威同步 REMOTE_ASR_MODEL 并落盘，
 *  不依赖模型选择器先前的异步持久化（2026-09-28 竞态修复：刚切完模型就点开始录音，
 *  异步落盘未完成时引擎会拿旧模型跑整场录音）。 */
export async function apiSaveTranscriptConfig(provider: string, model: string, apiKey: string | null, remoteAsrModel?: string | null): Promise<void> {
  return invoke('api_save_transcript_config', { provider, model, apiKey, authToken: null, remoteAsrModel: remoteAsrModel ?? null })
}

// ── 录音偏好 ──────────────────────────────────────────────────────────────────

export async function getRecordingPreferences(): Promise<RecordingPreferences> {
  return invoke<RecordingPreferences>('get_recording_preferences')
}

export async function setRecordingPreferences(preferences: {
  recordingsFolder: string
  autoSave: boolean
  defaultAsrModel?: string
}): Promise<void> {
  return invoke('set_recording_preferences', { preferences })
}

export async function getDefaultRecordingsFolderPath(): Promise<string> {
  return invoke<string>('get_default_recordings_folder_path')
}

export async function openRecordingsFolder(): Promise<void> {
  return invoke('open_recordings_folder')
}

export async function selectRecordingFolder(): Promise<string | null> {
  return invoke<string | null>('select_recording_folder')
}

// ── 远程服务（网关：ASR / 翻译 / TTS 共用） ─────────────────────────────────────

export async function setRemoteConfig(serverUrl: string, license: string, modelName?: string): Promise<void> {
  return invoke('set_remote_config', { serverUrl, license, modelName: modelName ?? null })
}

/** 只更新远程服务器地址（空串 = 恢复内置默认 https://api.voxmin.top），不动授权码/模型选择。 */
export async function setRemoteEndpoint(endpoint: string): Promise<void> {
  return invoke('set_remote_endpoint', { endpoint })
}

/** 远程连通性检查结果（两段式）。
 *  - ok：HTTP /health（服务器/域名/证书可达）
 *  - streamingOk：流式识别通道（wss 握手）是否可用；null = 不适用/未检测
 *    （未选流式模型、无授权码，或服务器本身不可达）
 *  - streamingError：streamingOk === false 时的原因（可直接展示） */
export interface RemoteHealthResult {
  ok: boolean
  streamingOk: boolean | null
  streamingError: string | null
}

export async function checkRemoteAsrHealth(endpoint: string): Promise<RemoteHealthResult> {
  return invoke<RemoteHealthResult>('check_remote_asr_health_cmd', { endpoint })
}

/** 提前预热流式识别通道（fire-and-forget）：App 启动 / 打开录音弹窗 / 切换识别模型时调用。
 *  预检成功有 5 分钟缓存，点「开始录音」时命中即免掉 ~3.5s 跨境握手。
 *  未配置远程/探测失败都被后端吞掉（只记日志），调用方无需处理错误。 */
export function warmRemoteStreaming(): void {
  invoke('warm_remote_streaming').catch(() => {})
}

export async function getRemoteConfig(): Promise<RemoteAsrConfig> {
  return invoke<RemoteAsrConfig>('get_remote_config')
}

export async function setRemoteEnabled(enabled: boolean): Promise<void> {
  return invoke('set_remote_enabled', { enabled })
}

export async function getRemoteEnabled(): Promise<boolean> {
  return invoke<boolean>('get_remote_enabled')
}

/** 网关当前启用的远程模型（asr/translate/tts 各自的模型 id） */
export async function getRemoteModels(): Promise<{ asr: string; translate: string; tts: string }> {
  return invoke('get_remote_models')
}

export interface RemoteModelItem {
  id: string
  object: string
  owned_by: string
  kind: 'asr' | 'translate' | 'tts'
  /** 前端显示名（可空串，空则回退 id） */
  display_name?: string
  /** 支持语言（可空串） */
  languages?: string
  /** ASR 专用：机器可读的支持语言码（规范码 ISO-639-1 + yue，由网关随目录下发）。
   *  用于在 ASR 模型旁渲染「识别语言」下拉框；客户端只发规范码，
   *  到各上游实际代码（豆包 zh-CN / Deepgram 粤语 zh-HK 等）的转换在网关完成。 */
  language_codes?: string[]
  /** 推荐模型（排序置顶 + 推荐标记） */
  recommended?: boolean
  price?: number
  price_unit?: string
  /** 流式(streaming) / 非流式(batch) */
  mode?: 'streaming' | 'batch'
  /** 推理模型（先输出思维链）：**不适合实时逐句翻译**（长句会把 max_tokens 全花在
   *  推理上 → 正文为空）。客户端据此把它们从实时翻译选择器里排除；会议总结不受影响。 */
  reasoning?: boolean
  /** LLM 用途（网关后台可配，随 /v1/models 下发）：
   *  'both' 翻译与会议总结都能选；'translate' 只翻译（如豆包机器翻译）；
   *  'summary' 只总结（如 deepseek-flash：输出价是输入的 4 倍，翻译成本是 qwen-flash 的 15 倍）。
   *  客户端据此过滤选择器，与网关侧强制校验（routing.ts）保持一致。
   *  旧版网关不下发此字段 → 一律按 'both' 处理（向后兼容）。非 LLM 恒为 'both'。 */
  usage?: 'both' | 'translate' | 'summary'
  /** 调用协议/端点 */
  protocol?: string
  /** TTS 专用：音色清单（由网关随目录下发，客户端不再硬编码）。
   *  lang='*' 表示与语言无关（Supertonic 的 sid 只决定音色，发音由 language 决定）。 */
  voices?: TtsVoiceOption[]
}

/** 网关下发的 TTS 音色项。 */
export interface TtsVoiceOption {
  /** 传给上游的 voice 取值（MiMo 是中文音色名，Supertonic 是 sid 数字字符串） */
  id: string
  /** 适用语言码；'*' = 与语言无关 */
  lang?: string
  gender?: string
}

/** list_remote_models 的返回负载：新版为 { data, updated_at }，旧版直接是数组（兼容） */
type RemoteModelsPayload = RemoteModelItem[] | { data?: RemoteModelItem[]; updated_at?: string }

function parseRemoteModelsPayload(payload: RemoteModelsPayload): { models: RemoteModelItem[]; updatedAt: string } {
  if (Array.isArray(payload)) return { models: payload, updatedAt: '' }
  return { models: payload?.data ?? [], updatedAt: payload?.updated_at ?? '' }
}

/** 网关 /v1/models 全量列表（供模型选择器下拉） */
export async function listRemoteModels(): Promise<RemoteModelItem[]> {
  const payload = await invoke<RemoteModelsPayload>('list_remote_models')
  return parseRemoteModelsPayload(payload).models
}

/** 网关 /v1/models 完整目录（含 updated_at，用于轮询变更检测） */
export async function listRemoteCatalog(): Promise<{ models: RemoteModelItem[]; updatedAt: string }> {
  const payload = await invoke<RemoteModelsPayload>('list_remote_models')
  return parseRemoteModelsPayload(payload)
}

/** 读回各能力的远程模型选择（asr=实时转录用流式模型；asr_offline=历史离线重识别用非流式模型） */
export async function getRemoteModelChoice(): Promise<{ asr: string; asr_offline: string; translate: string; summary: string; tts: string }> {
  return invoke('get_remote_model_choice')
}

/** 设置各能力的远程模型选择（asr_mode 透传网关 mode，用于决定流式/非流式） */
export async function setRemoteModelChoice(choice: { asr?: string; asr_mode?: string; asr_offline?: string; translate?: string; summary?: string; tts?: string }): Promise<void> {
  // ⚠️ Tauri v2 命令参数按 camelCase 匹配 Rust 参数名（snake_case）。
  // 此前一直用 snake_case 传 asr_mode/asr_offline → Rust 侧永远收到 None：
  // 「流式/非流式判断」与「离线重识别模型」两个字段从未生效（2026-09-20
  // 音频测试豆包 400 / 会议总结离线识别失败 的共同根因）。
  return invoke('set_remote_model_choice', {
    asr: choice.asr ?? null,
    asrMode: choice.asr_mode ?? null,
    asrOffline: choice.asr_offline ?? null,
    translate: choice.translate ?? null,
    summary: choice.summary ?? null,
    tts: choice.tts ?? null,
  })
}

/** 录音中热切换流式 ASR 引擎（调用前需先持久化新选择：apiSaveTranscriptConfig + 远程模型选择）。
 *  切到远程时把当前真实远程模型 id 一并传给后端（2026-09-28 竞态修复：以显式指定为准）。 */
export async function switchAsrModel(remoteAsrModel?: string | null): Promise<void> {
  return invoke('switch_asr_model', { remoteAsrModel: remoteAsrModel ?? null })
}

// ── 远程 TTS（网关 /v1/audio/speech） ──────────────────────────────────────────

/** 合成一段文本为语音，返回 base64 音频 + MIME 类型。
 *  voice: 音色（可选；缺省走供应商默认音色）
 *  model: 指定远程 TTS 模型（可选；缺省用「远程服务」里选择的 TTS 模型）
 *  instructions: 自然语言指令（可选；MiMo 的风格/情感/语速，或音色设计的音色描述）
 *  language: 文本语言码（可选；**强烈建议传**）—— 网关据此路由到合适的 TTS 模型
 *            （中文→MiMo，其余 31 语种→自建 Supertonic），且 Supertonic 必须靠它才能正确发音 */
export async function ttsSynthesize(
  text: string,
  voice?: string,
  model?: string,
  instructions?: string,
  language?: string,
): Promise<TtsSynthesisResult> {
  return invoke<TtsSynthesisResult>('tts_synthesize', {
    text,
    voice: voice ?? null,
    model: model ?? null,
    instructions: instructions ?? null,
    language: language ?? null,
  })
}

/** 弹出保存对话框，把 TTS 音频保存到用户选择的位置。返回写入路径（取消时 null）。 */
export async function saveTtsAudio(
  audioBase64: string,
  suggestedName: string,
  mime: string,
): Promise<string | null> {
  return invoke<string | null>('save_tts_audio', {
    audioBase64,
    suggestedName,
    mime,
  })
}

// ── 文件导入 ──────────────────────────────────────────────────────────────────

export async function selectAndValidateAudio(): Promise<AudioFileInfo | null> {
  return invoke<AudioFileInfo | null>('select_and_validate_audio_command')
}

export async function startImportAudio(
  sourcePath: string,
  title: string,
  language?: string,
  model?: string | null,
  provider?: string | null
): Promise<{ message: string }> {
  return invoke('start_import_audio_command', {
    sourcePath,
    title,
    language: language ?? null,
    model: model ?? null,
    provider: provider ?? null,
  })
}

export async function cancelImportAudio(): Promise<void> {
  return invoke('cancel_import_command')
}

export function onImportProgress(callback: (p: ImportProgress) => void): Promise<UnlistenFn> {
  return listen<ImportProgress>('import-progress', (e) => callback(e.payload))
}

export function onImportComplete(callback: (r: ImportResult) => void): Promise<UnlistenFn> {
  return listen<ImportResult>('import-complete', (e) => callback(e.payload))
}

export function onImportError(callback: (e: ImportError) => void): Promise<UnlistenFn> {
  return listen<ImportError>('import-error', (e) => callback(e.payload))
}

export function onImportWarning(callback: (w: ImportWarning) => void): Promise<UnlistenFn> {
  return listen<ImportWarning>('import-warning', (e) => callback(e.payload))
}

// ── 重新转写（离线转写已导入的音频） ─────────────────────────────────────────

export async function startRetranscription(
  meetingId: string,
  meetingFolderPath: string,
  model?: string | null,
  provider?: string | null,
  /** 识别语言（规范码）。'auto'/空 = 交给上游自动检测。 */
  language?: string | null
): Promise<{ meeting_id: string; message: string }> {
  return invoke('start_retranscription_command', {
    meetingId,
    meetingFolderPath,
    // 传 'auto' 而不是 null：null 会让后端回落到「上一次录音的全局语言偏好」，
    // 而这里用户明确选了自动检测（见 retranscription.rs 的语言语义注释）。
    language: language && language !== '' ? language : 'auto',
    model: model ?? null,
    provider: provider ?? null,
    estimatedRtf: null,
  })
}

export async function cancelRetranscription(): Promise<void> {
  return invoke('cancel_retranscription_command')
}

/** 翻译链路告警（如「模型返回空译文」）——后端 60s 节流后下发，前端 toast 提示。 */
export function onTranslationWarning(callback: (e: { message: string }) => void): Promise<UnlistenFn> {
  return listen<{ message: string }>('translation-warning', (e) => callback(e.payload))
}

export function onRetranscriptionProgress(callback: (p: RetranscriptionProgress) => void): Promise<UnlistenFn> {
  return listen<RetranscriptionProgress>('retranscription-progress', (e) => callback(e.payload))
}

export function onRetranscriptionComplete(callback: (r: RetranscriptionResult) => void): Promise<UnlistenFn> {
  return listen<RetranscriptionResult>('retranscription-complete', (e) => callback(e.payload))
}

export function onRetranscriptionError(callback: (e: RetranscriptionError) => void): Promise<UnlistenFn> {
  return listen<RetranscriptionError>('retranscription-error', (e) => callback(e.payload))
}

/** 用户主动「停止识别」成功中断（不是故障）：前端提示「已停止」并复位进行中状态 */
export function onRetranscriptionCancelled(callback: (e: { meeting_id: string }) => void): Promise<UnlistenFn> {
  return listen<{ meeting_id: string }>('retranscription-cancelled', (e) => callback(e.payload))
}

export function onRetranscriptionPartial(callback: (p: RetranscriptionPartial) => void): Promise<UnlistenFn> {
  return listen<RetranscriptionPartial>('retranscription-partial', (e) => callback(e.payload))
}

// ── 音频测试（模型验证） ──────────────────────────────────────────────────────

export async function startAudioTest(
  modelName: string,
  micDeviceName?: string | null,
  systemDeviceName?: string | null,
  remoteAsrModel?: string | null,
): Promise<number> {
  return invoke<number>('start_audio_test', {
    modelName,
    micDeviceName: micDeviceName ?? null,
    systemDeviceName: systemDeviceName ?? null,
    remoteAsrModel: remoteAsrModel ?? null,
  })
}

export async function stopAudioTest(): Promise<void> {
  return invoke('stop_audio_test')
}

/** 重新播放示例音频（自检进行中再次触发），返回 WAV 时长（秒）。 */
export async function replayAudioTest(): Promise<number> {
  return invoke<number>('replay_audio_test')
}

/** 自检转写事件（payload 与 TranscriptUpdate 一致，来自独立测试链路）。 */
export function onAudioTestTranscript(callback: (update: TranscriptUpdate) => void): Promise<UnlistenFn> {
  return listen<TranscriptUpdate>('audio-test-transcript', (event) => callback(event.payload))
}

/** 自检示例音频开始播放（带 duration，驱动前端进度条）。 */
export function onAudioTestPlaybackStarted(
  callback: (payload: { duration: number }) => void
): Promise<UnlistenFn> {
  return listen<{ duration: number }>('audio-test-playback-started', (event) => callback(event.payload))
}

/** 转写错误事件 payload（录音与音频自检共用同一转写链路，后端两处都会发）。 */
export interface TranscriptionErrorPayload {
  error: string
  userMessage?: string
  actionable?: boolean
}

/** 转写致命错误（如模型初始化失败 / 远程连接失败）。 */
export function onTranscriptionError(callback: (e: TranscriptionErrorPayload) => void): Promise<UnlistenFn> {
  return listen<TranscriptionErrorPayload>('transcription-error', (e) => callback(e.payload))
}

/** 转写非致命警告（payload 为纯文本消息，如远程流式会话内错误）。 */
export function onTranscriptionWarning(callback: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>('transcription-warning', (e) => callback(e.payload))
}

/**
 * 识别链路状态（2026-09-29）：远程流式 ASR 连上时 running=true，终止时 running=false
 * + reason（'credits' 积分不足 | 'config' 鉴权/配置 | 'unavailable' 重连耗尽
 * | 'ended' 正常结束）。只用于把底部「● 实时转写中」换成「▲ 识别已停止」——
 * 录音与落盘完全不受影响。
 */
export interface TranscriptionStatusPayload {
  running: boolean
  reason?: string
}

export function onTranscriptionStatus(
  callback: (s: TranscriptionStatusPayload) => void
): Promise<UnlistenFn> {
  return listen<TranscriptionStatusPayload>('transcription-status', (e) => callback(e.payload))
}

// ── 语言偏好 ──────────────────────────────────────────────────────────────────

export async function setLanguagePreference(language: string): Promise<void> {
  return invoke('set_language_preference', { language })
}

// ── 录音事件 ──────────────────────────────────────────────────────────────────

export function onTranscriptUpdate(callback: (update: TranscriptUpdate) => void): Promise<UnlistenFn> {
  return listen<TranscriptUpdate>('transcript-update', (event) => callback(event.payload))
}

export function onRecordingStarted(callback: () => void): Promise<UnlistenFn> {
  return listen('recording-started', callback)
}

export function onRecordingStopped(
  callback: (payload: { message: string; folder_path?: string; meeting_name?: string }) => void
): Promise<UnlistenFn> {
  return listen<{ message: string; folder_path?: string; meeting_name?: string }>(
    'recording-stopped',
    (event) => callback(event.payload)
  )
}

export function onRecordingPaused(callback: () => void): Promise<UnlistenFn> {
  return listen('recording-paused', callback)
}

export function onRecordingResumed(callback: () => void): Promise<UnlistenFn> {
  return listen('recording-resumed', callback)
}

export function onSpeechDetected(callback: () => void): Promise<UnlistenFn> {
  return listen('speech-detected', callback)
}

/** VAD 检测到人声（active=true）/ 结束（active=false），用于「正在识别」提示 */
export function onVadSpeechActivity(callback: (payload: { active: boolean }) => void): Promise<UnlistenFn> {
  return listen<{ active: boolean }>('vad-speech-activity', (event) => callback(event.payload))
}

export function onMicMuteChanged(callback: (payload: { muted: boolean }) => void): Promise<UnlistenFn> {
  return listen<{ muted: boolean }>('mic-mute-changed', (event) => callback(event.payload))
}

export function onDefaultDeviceChanged(
  callback: (payload: { microphone?: string | null; system_audio?: string | null }) => void
): Promise<UnlistenFn> {
  return listen<{ microphone?: string | null; system_audio?: string | null }>(
    'default-device-changed',
    (event) => callback(event.payload)
  )
}

export function onWaitingForAudioDevice(
  callback: (payload: { microphone?: string | null; system_audio?: string | null }) => void
): Promise<UnlistenFn> {
  return listen<{ microphone?: string | null; system_audio?: string | null }>(
    'waiting-for-audio-device',
    (event) => callback(event.payload)
  )
}

// ── 翻译 ──────────────────────────────────────────────────────────────────────

export async function translateText(
  text: string,
  direction: TranslationDirection,
  target?: TranslateTargetLang,
  requestId?: string
): Promise<string> {
  return invoke<string>('translate_text', {
    text,
    direction,
    target: target ?? null,
    requestId: requestId ?? null,
  })
}

export async function setTranslationEnabled(enabled: boolean): Promise<void> {
  return invoke('set_translation_enabled', { enabled })
}

export async function getTranslationEnabled(): Promise<boolean> {
  return invoke<boolean>('get_translation_enabled')
}

export async function setTranslationTargetLang(lang: TranslateTargetLang): Promise<void> {
  return invoke('set_translation_target_lang', { lang })
}

export async function getTranslationTargetLang(): Promise<TranslateTargetLang> {
  return invoke<TranslateTargetLang>('get_translation_target_lang')
}

/** 流式分段停顿（秒）：连续静音超过该时长后转写另起一段（仅流式引擎） */
export async function getFlowPauseSecs(): Promise<number> {
  return invoke<number>('get_flow_pause_secs')
}

export async function setFlowPauseSecs(secs: number): Promise<void> {
  return invoke('set_flow_pause_secs', { secs })
}

export async function setTranslationHomeLang(lang: string): Promise<void> {
  return invoke('set_translation_home_lang', { lang })
}

export async function getTranslationHomeLang(): Promise<string> {
  return invoke<string>('get_translation_home_lang')
}

export async function setTranslationEngine(engine: TranslationEngine): Promise<void> {
  return invoke('set_translation_engine', { engine })
}

export async function getTranslationEngine(): Promise<TranslationEngine> {
  return invoke<TranslationEngine>('get_translation_engine')
}

export function onTranslateUpdate(callback: (update: TranslateUpdate) => void): Promise<UnlistenFn> {
  return listen<TranslateUpdate>('translate-update', (event) => callback(event.payload))
}

/** translate_text 带 requestId 时的流式增量事件（hymt2 引擎；opus 不发） */
export function onTranslateTextStream(
  callback: (e: TranslateTextStreamEvent) => void
): Promise<UnlistenFn> {
  return listen<TranslateTextStreamEvent>('translate-text-stream', (event) => callback(event.payload))
}

// ── 其他 ──────────────────────────────────────────────────────────────────────

export async function getTranscriptHistory(): Promise<TranscriptSegment[]> {
  return invoke<TranscriptSegment[]>('get_transcript_history')
}

export async function openExternalUrl(url: string): Promise<void> {
  return invoke('open_external_url', { url })
}

export function onFirstLaunchDetected(callback: () => void): Promise<UnlistenFn> {
  return listen('first-launch-detected', callback)
}

export function onDatabaseInitialized(callback: () => void): Promise<UnlistenFn> {
  return listen('database-initialized', callback)
}

// ── 音频电平 / 频谱监听 ──────────────────────────────────────────────────────

export async function startAudioLevelMonitoring(deviceNames: string[]): Promise<void> {
  return invoke('start_audio_level_monitoring', { deviceNames })
}

export async function stopAudioLevelMonitoring(): Promise<void> {
  return invoke('stop_audio_level_monitoring')
}

export function onAudioLevels(callback: (update: AudioLevelUpdate) => void): Promise<UnlistenFn> {
  return listen<AudioLevelUpdate>('audio-levels', (event) => callback(event.payload))
}

// ── 会议总结 ──────────────────────────────────────────────────────────────────

export async function summaryGetConfig(): Promise<SummaryApiConfig | null> {
  return invoke<SummaryApiConfig | null>('summary_get_config')
}

export async function summarySaveConfig(config: SummaryApiConfig): Promise<void> {
  return invoke('summary_save_config', { config })
}

export async function summaryTestConnection(config: SummaryApiConfig): Promise<string> {
  return invoke<string>('summary_test_connection', { config })
}

export async function summaryListModels(config: SummaryApiConfig): Promise<string[]> {
  return invoke<string[]>('summary_list_models', { config })
}

/** 通过 API 生成总结；token 流通过 summary-stream 事件推送。 */
export async function summaryGenerate(
  requestId: string,
  config: SummaryApiConfig,
  prompt: string,
  options?: { maxTokens?: number; temperature?: number }
): Promise<void> {
  return invoke('summary_generate', {
    requestId,
    config,
    prompt,
    maxTokens: options?.maxTokens ?? null,
    temperature: options?.temperature ?? null,
  })
}

/** 已注册的本地总结模型列表（按后端优先级排序）。 */
export async function summaryLocalModels(): Promise<SummaryLocalModelInfo[]> {
  return invoke<SummaryLocalModelInfo[]>('summary_local_models')
}

/** 通过本地模型生成总结；modelId 省略时后端自动选第一个已安装的总结模型。 */
export async function summaryLocalGenerate(
  requestId: string,
  prompt: string,
  options?: { maxTokens?: number; modelId?: string }
): Promise<void> {
  return invoke('summary_local_generate', {
    requestId,
    prompt,
    maxTokens: options?.maxTokens ?? null,
    modelId: options?.modelId ?? null,
  })
}

export async function summaryCancel(requestId: string): Promise<void> {
  return invoke('summary_cancel', { requestId })
}

export async function summarySave(
  recordingId: string,
  source: 'realtime' | 'offline',
  content: string
): Promise<string> {
  return invoke<string>('summary_save', { recordingId, source, content })
}

export async function summaryLoad(
  recordingId: string,
  source: 'realtime' | 'offline'
): Promise<string | null> {
  return invoke<string | null>('summary_load', { recordingId, source })
}

export function onSummaryStream(callback: (e: SummaryStreamEvent) => void): Promise<UnlistenFn> {
  return listen<SummaryStreamEvent>('summary-stream', (e) => callback(e.payload))
}

/** 把总结内容导出为 Markdown 文件，返回导出文件路径。 */
export async function summaryExportMarkdown(
  recordingId: string,
  content: string,
  outputDir?: string | null
): Promise<string> {
  return invoke<string>('summary_export_markdown', {
    recordingId,
    content,
    outputDir: outputDir ?? null,
  })
}

// ── 桌面字幕悬浮窗 ─────────────────────────────────────────────────────────────

export async function showSubtitleWindow(): Promise<void> {
  return invoke('show_subtitle_window')
}

export async function hideSubtitleWindow(): Promise<void> {
  return invoke('hide_subtitle_window')
}

export async function toggleSubtitleWindow(): Promise<boolean> {
  return invoke<boolean>('toggle_subtitle_window')
}

export async function getSubtitleWindowState(): Promise<boolean> {
  return invoke<boolean>('get_subtitle_window_state')
}

export function onSubtitleWindowState(
  callback: (payload: { visible: boolean }) => void
): Promise<UnlistenFn> {
  return listen<{ visible: boolean }>('subtitle-window-state', (event) => callback(event.payload))
}

export interface SubtitleSegmentInput {
  sequence_id: number
  text: string
  is_partial: boolean
}

export interface SubtitleTranslationInput {
  sequence_id: number
  translated_text: string
}

export async function pushSubtitleSegment(update: SubtitleSegmentInput): Promise<void> {
  return invoke('push_subtitle_segment', { update })
}

export async function pushSubtitleTranslation(update: SubtitleTranslationInput): Promise<void> {
  return invoke('push_subtitle_translation', { update })
}

// ── 悬浮球 ───────────────────────────────────────────────────────────────────

export async function showFloatingBall(): Promise<void> {
  return invoke('show_floating_ball')
}

export async function hideFloatingBall(): Promise<void> {
  return invoke('hide_floating_ball')
}

export async function toggleFloatingBall(): Promise<boolean> {
  return invoke<boolean>('toggle_floating_ball')
}

export async function getFloatingBallState(): Promise<boolean> {
  return invoke<boolean>('get_floating_ball_state')
}

export function onFloatingBallState(
  callback: (payload: { visible: boolean }) => void
): Promise<UnlistenFn> {
  return listen<{ visible: boolean }>('floating-ball-state', (event) => callback(event.payload))
}

// ── 远程推送消息（tips / 公告 / 最新版本） ─────────────────────────────────────

export interface RemoteMessage {
  id: string
  type: string
  /** 展示渠道：footer = 页面底部短信息；startup = 启动时长信息弹窗 */
  channel?: 'footer' | 'startup'
  title: string
  body: string
  /** 可选图片（data URL 或 http 链接），仅 startup 渠道显示 */
  image?: string
  severity: string
  dismissible: boolean
  created_at: string
}

export interface RemoteLatestVersion {
  version: string
  release_notes: string
  download_url: string
  published_at: string
}

/** 启动文档（版本化多文档，本地缓存，离线可读）。 */
export interface NoticeDocument {
  version: number
  title: string
  body: string
  images: string[]
  updated_at: string
}

/** 拉取启动文档列表（最新在第一页）；离线时返回本地缓存。
 *  lang = 界面语言（zh/en/ko/ja），透传网关 ?lang= 取对应语言文档；缓存按语言分文件。 */
export async function fetchNoticeDocuments(lang?: string): Promise<NoticeDocument[]> {
  const r = await invoke<{ items: NoticeDocument[] }>('fetch_notice_documents', { lang: lang ?? null })
  return r.items || []
}

/** 欢迎弹窗的欢迎词（网关推送，无需鉴权）。 */
export interface WelcomeMessage {
  title: string
  body: string
  updated_at?: string
}

/** 拉取欢迎词（GET {server}/v1/welcome?lang=…）；网关未配置/不可达时 reject，前端回退默认欢迎词。 */
export async function fetchWelcome(lang?: string): Promise<WelcomeMessage> {
  return invoke<WelcomeMessage>('fetch_welcome', { lang: lang ?? null })
}

export async function fetchRemoteMessages(lang?: string): Promise<{
  announcements: RemoteMessage[]
  latestVersion: RemoteLatestVersion | null
}> {
  return invoke('fetch_remote_messages', { lang: lang ?? null })
}

/** 重要信息推送（顶部横幅）：text 为空串 = 不显示。 */
export interface ImportantNotice {
  text: string
  updated_at: string
  lang?: string | null
}

/** 拉取重要信息推送（GET {server}/v1/important-notice?lang=…）；网关未配置/不可达时 reject，前端静默不渲染。 */
export async function fetchImportantNotice(lang?: string): Promise<ImportantNotice> {
  return invoke<ImportantNotice>('fetch_important_notice', { lang: lang ?? null })
}

/** 拉取积分余额（网关 /v1/usage，含低余额预警）。 */
export async function getRemoteUsage(): Promise<{
  license: string
  name: string
  credits: number
  low_balance?: boolean
  threshold?: number
}> {
  return invoke('get_remote_usage')
}

/** 设备绑定自动注册（网关 /v1/register），返回 { api_key, credits, is_new }。 */
export async function registerDevice(): Promise<{ api_key: string; credits: number; is_new: boolean }> {
  return invoke('register_device')
}

/** 兑换码充值（网关 /v1/redeem），返回 { credits, added }。 */
export async function redeemCode(code: string): Promise<{ credits: number; added: number }> {
  return invoke('redeem_code', { code })
}

/** 用户积分流水（网关 /v1/ledger）。 */
export interface LedgerItem {
  id: number
  type: string
  amount: number
  balance_after: number
  remark: string
  created_at: string
}
export async function getRemoteLedger(): Promise<{ items: LedgerItem[] }> {
  return invoke('get_remote_ledger')
}

/** 用户按模型消耗汇总（网关 /v1/usage/by-model）。 */
export interface ModelUsageItem {
  provider: string
  model: string
  kind: string
  units: number
  unit: string
  credits: number
  cost_cny: number
  calls: number
}
export async function getRemoteUsageByModel(): Promise<{ items: ModelUsageItem[] }> {
  return invoke('get_remote_usage_by_model')
}

/** 任务消耗汇总（网关 /v1/usage/tasks）：一次录音 / 一次离线识别 / 一次会议总结各消耗多少。
 *
 *  与「按模型消耗」的区别：按模型看不出「这次录音花了多少」（一次录音有几百次翻译调用），
 *  任务口径把同一次录音的识别 + 翻译 + 总结合成一行，用户一眼能读懂。 */
export interface TaskUsageBreakdown {
  kind: string
  label: string
  credits: number
  units: number
  unit: string
  calls: number
  models: string[]
}
export interface TaskUsageItem {
  task_key: string
  session_id: string
  task: string
  started_at: string
  ended_at: string
  total_credits: number
  total_cost_cny: number
  breakdown: TaskUsageBreakdown[]
}
export async function getRemoteUsageTasks(): Promise<{ items: TaskUsageItem[] }> {
  return invoke('get_remote_usage_tasks')
}

/** 模型测速（仅测往返延迟，不扣积分）：kind = asr | llm | tts。 */
export interface SpeedTestResult {
  kind: string
  model: string | null
  ok: boolean
  ms: number
  detail?: string
}

export async function runSpeedTest(kind: 'asr' | 'llm' | 'tts', model?: string): Promise<SpeedTestResult> {
  return invoke<SpeedTestResult>('run_speed_test', { kind, model: model ?? null })
}

/** 提交用户反馈（文字 + 可选截图 base64 + 联系方式 + 可选诊断日志）。 */
export async function submitFeedback(
  text: string,
  screenshot?: string | null,
  contact?: string | null,
  diagLog?: string | null
): Promise<void> {
  return invoke('submit_feedback', {
    text,
    screenshot: screenshot ?? null,
    contact: contact ?? null,
    diagLog: diagLog ?? null,
  })
}

/**
 * 收集客户端诊断日志（脱敏后的日志尾部），供反馈一键附加。
 * 软件发布后我们拿不到用户机器上的日志文件，只能靠这里带上来。
 */
export async function collectDiagLog(): Promise<string> {
  return invoke('collect_diag_log')
}

/** 诊断日志附加范围（2026-09-28）：按文件数（最近 N 次运行）或按时间窗（最近一天）。 */
export interface DiagLogRange {
  /** 最近 N 个日志文件（≈ 最近 N 次运行），每文件 ≤320KB */
  maxFiles?: number
  /** 最近 N 小时内修改过的日志文件（优先于 maxFiles），每文件 ≤160KB */
  sinceHours?: number
}

/** 按用户选择的范围收集诊断日志；range 省略时等同 collectDiagLog（默认最近 2 个文件）。 */
export async function collectDiagLogRange(range?: DiagLogRange): Promise<string> {
  return invoke('collect_diag_log', {
    maxFiles: range?.maxFiles ?? null,
    sinceHours: range?.sinceHours ?? null,
  })
}

/** 应用日志目录绝对路径：「附加日志文件…」对话框的默认定位（macOS/Windows 通用，dev 模式指向 target 旁 logs/）。 */
export async function getLogDir(): Promise<string | null> {
  return invoke('get_log_dir')
}

/** 直接用系统文件管理器打开应用日志目录；返回打开后的绝对路径。
 *  反馈卡「打开日志文件夹」按钮用 —— 免得用户自己去找 %LOCALAPPDATA%\VoxMinutes\logs。 */
export async function openLogFolder(): Promise<string> {
  return invoke<string>('open_log_folder')
}

/** 构建/安装信息：版本号 + 可执行文件落地时间（用于分辨「装的是哪一版」）。 */
export async function getBuildInfo(): Promise<{ version: string; exeModifiedUnix: number | null }> {
  return invoke('get_build_info')
}

/** 读取用户手动挑选的日志文件（同样脱敏 + 单文件 ≤160KB），头部标注「用户手动附加」。 */
export async function collectManualLogs(paths: string[]): Promise<string> {
  return invoke('collect_manual_logs', { paths })
}
