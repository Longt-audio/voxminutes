// VoxMinutes MVP IPC 层 —— 只包含 MVP 后端命令
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
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
} from '@/types'

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

// ── 模型下载 ──────────────────────────────────────────────────────────────────

export async function getDownloadableModels(): Promise<DownloadableModelInfo[]> {
  return invoke<DownloadableModelInfo[]>('get_downloadable_models')
}

export async function downloadModel(modelId: string): Promise<void> {
  return invoke('download_model', { modelId })
}

export async function cancelModelDownload(modelId: string): Promise<void> {
  return invoke('cancel_model_download', { modelId })
}

export async function deleteModel(modelId: string): Promise<void> {
  return invoke('delete_model', { modelId })
}

/** 从本地文件/文件夹导入模型；后端弹原生选择框，进度走 model-download-progress 事件。 */
export async function importModelFile(modelId: string): Promise<ImportModelResult> {
  return invoke<ImportModelResult>('import_model_file', { modelId })
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

export async function apiSaveTranscriptConfig(provider: string, model: string, apiKey: string | null): Promise<void> {
  return invoke('api_save_transcript_config', { provider, model, apiKey, authToken: null })
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

// ── 远程 ASR（预留接口） ──────────────────────────────────────────────────────

export async function setRemoteAsrEndpoint(endpoint: string, modelName?: string): Promise<void> {
  return invoke('set_remote_asr_endpoint', { endpoint, modelName: modelName ?? null })
}

export async function checkRemoteAsrHealth(endpoint: string): Promise<boolean> {
  return invoke<boolean>('check_remote_asr_health_cmd', { endpoint })
}

export async function getRemoteAsrConfig(): Promise<RemoteAsrConfig> {
  return invoke<RemoteAsrConfig>('get_remote_asr_config')
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
  provider?: string | null
): Promise<{ meeting_id: string; message: string }> {
  return invoke('start_retranscription_command', {
    meetingId,
    meetingFolderPath,
    language: null,
    model: model ?? null,
    provider: provider ?? null,
    estimatedRtf: null,
  })
}

export async function cancelRetranscription(): Promise<void> {
  return invoke('cancel_retranscription_command')
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

export function onRetranscriptionPartial(callback: (p: RetranscriptionPartial) => void): Promise<UnlistenFn> {
  return listen<RetranscriptionPartial>('retranscription-partial', (e) => callback(e.payload))
}

// ── 音频测试（模型验证） ──────────────────────────────────────────────────────

export async function startAudioTest(modelName: string): Promise<number> {
  return invoke<number>('start_audio_test', { modelName })
}

export async function stopAudioTest(): Promise<void> {
  return invoke('stop_audio_test')
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
