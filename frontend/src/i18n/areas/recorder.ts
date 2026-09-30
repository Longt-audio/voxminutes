import type { Language } from '../languages'

/** 录音页文案：控制面板、录音设置弹窗、录音 hooks 的提示语。 */
export interface RecorderMessages {
  recStop: string
  recPreparing: string
  recStart: string
  recResume: string
  recPause: string
  recMuted: string
  recMute: string
  recTranslate: string
  recTranslating: string
  recTranslateTitle: string
  recSubtitle: string
  recSubtitleTitle: string
  recTargetLang: string
  recTranslateAuto: string
  recTranslateToEn: string
  recTranslateToZh: string
  recTranslateEngine: string
  recEngineOpus: string
  recEngineHymt2: string
  recEngineRemote: string
  recEngineCustomApi: string
  recRecommended: string
  recBadgeLocalFree: string
  recBadgeRemote: string
  recBadgeStreaming: string
  recBadgeBatch: string
  recStreamingHint: string
  recLangSupport: string
  recRemoteModel: string
  recRemoteModelDesc: string
  recRemoteModelSelect: string
  recRemoteNoModels: string
  recModelXAsr: string
  recModelSenseVoice: string
  recNoModel: string
  recLabelAsr: string
  recLabelLangs: string
  recLangsXAsr: string
  recLangsSenseVoice: string
  recLabelMic: string
  recLabelSpeaker: string
  recNoDevice: string
  recAudioDevices: string
  recOpenSoundTitle: string
  recSourceHint: string
  recNoModelTitle: string
  recNoModelPre: string
  recNoModelLink: string
  recNoModelPost: string
  recPaused: string
  recMicShort: string
  recSysShort: string
  recMicLevelTitle: string
  recSysLevelTitle: string
  recSetupTitle: string
  recSetupDesc: string
  recAsrModel: string
  recXAsrDesc: string
  recSenseVoiceDesc: string
  recNotDownloaded: string
  recMic: string
  recSystemDefault: string
  recSystemAudio: string
  recRecogLang: string
  recLangAuto: string
  recLangZh: string
  recLangEn: string
  recXAsrLangTitle: string
  /** 该 ASR 模型支持的语言列表（{list} 为语言名列表） */
  recLangSupported: string
  /** 该模型未声明支持语言 → 仅「自动检测」可选 */
  recLangAutoOnly: string
  /** 换模型后原语言不再受支持时的提示 */
  recLangResetByModel: string
  recMuteOnStart: string
  recTranslateCheck: string
  recCurrentDefault: string
  recSoundSettings: string
  /** 新录音默认标题前缀：语言无关常量 'Rec'（与录音文件夹 Rec_ 前缀一致），
   *  生成形如 Rec_2026-09-28_15-22；只影响新录音，历史记录标题不变 */
  recDefaultTitle: string
  recSavedToHistory: string
  recSaveFailed: string
  recDeviceSwitched: string
  recDeviceSwitchedDesc: string
  recNone: string
  recDeviceDisconnected: string
  recWaitingForDevice: string
  recStartFailed: string
  recStopFailed: string
  recActionFailed: string
  recRecognizing: string
  recRecognizingHint: string
  recAudioSelfTest: string
  recTestTitle: string
  recTestEntryHint: string
  recTabAsr: string
  recTabTranslate: string
  recTabAudio: string
  recInfoToggle: string
  recInfoRemoteLabel: string
  recInfoRemoteOnline: string
  recInfoRemoteOffline: string
  recInfoRemoteDisabled: string
  recInfoTranslateOff: string
  recTestDesc: string
  recTestReplay: string
  recTestPlaying: string
  recTestResultLabel: string
  recTestWaiting: string
  recTestLoadingModel: string
  recTestEnergy: string
  recTestErrorTitle: string
  recTestErrorHint: string
  recTestClose: string
  recAsrGroupLocal: string
  recAsrGroupRemote: string
  recRemoteBenefits: string
  recRemoteEnableGuide: string
  recRemoteNoStreaming: string
  recRemoteNoBatch: string
  recOfflineHint: string
  recTestModelLabel: string
  /** 音频测试：开始测试按钮 */
  recTestStart: string
  /** 音频测试：播放结束自动停止后的「重新测试」按钮 */
  recTestRetest: string
  /** 音频测试：待开始状态提示（结果区占位） */
  recTestIdleHint: string
  /** 音频测试：播放结束自动停止后的小字提示（远程模型按时长计费） */
  recTestAutoStopped: string
  /** 全局录音指示条：正在转录（任何页面可见） */
  recLiveTranscribing: string
  /** 全局录音指示条：点击回到实时转录页（tooltip） */
  recBackToLive: string
  /** 远程流式 ASR 断线自动重连中的 toast 提示 */
  recAsrReconnecting: string
  /** 远程流式 ASR 断线较久（≥30s）仍在重连的 toast 提示 */
  recAsrReconnectingLong: string
  /** 远程流式 ASR 上游无响应（网关↔上游假死，正在重建上游会话）的 toast 提示 */
  recAsrUpstreamStalled: string
  /**
   * 把后端带 code 的告警映射成界面文案（未知 code 回退 message）。
   * 后端只发 code（文案跟界面语言走），见 remote_asr_streaming_provider.rs::warn_outage。
   */
  recAsrWarningText: (code: string, fallback: string) => string
  /** 录音面板：录音中切换识别模型的下拉标题 */
  recAsrSwitchLabel: string
  /** 录音中切换识别模型：切换进行中小字 */
  recAsrSwitching: string
  /** 录音中切换识别模型：成功 toast */
  recAsrSwitched: string
  /** 录音中切换识别模型：失败 toast（{error} 占位） */
  recAsrSwitchFailed: string
  /** 当前是非流式模型时切换下拉的禁用提示 */
  recAsrSwitchStreamingOnly: string
  /** 模型选择 tips（录音设置弹窗模型选择最上方的小字弱化提示） */
  recModelPickTips: string
  /** 录音面板：翻译模型选择器的标题 */
  recTranslateModelLabel: string
  /** 模型卡片「支持语言」收起态的展开按钮 */
  recLangsExpand: string
  /** 模型卡片「支持语言」展开态的收起按钮 */
  recLangsCollapse: string
  /** 额度预估：余额充足时的一行（{credits} 余额、{duration} 预计可录时长） */
  recCreditEstimate: string
  /** 额度预估：低于 2 分钟预冻额度，连会话都建不起来（{min} 至少需要的积分） */
  recCreditTooLow: string
  /** 额度预估：口径说明小字（不含翻译/总结；静音也计费） */
  recCreditEstimateHint: string
  /** 时长格式：{h} 小时 {m} 分钟 */
  recDurHourMin: string
  /** 时长格式：{m} 分钟 */
  recDurMin: string
  /** 时长格式：{s} 秒 */
  recDurSec: string
  /** 识别已停止：通用（录音继续） */
  recAsrStopped: string
  /** 识别已停止：积分不足（录音继续，可稍后用离线识别补齐） */
  recAsrStoppedCredits: string
  /** 识别已停止：鉴权/配置类错误 */
  recAsrStoppedConfig: string
}

/**
 * 远程流式 ASR 告警文案（每个 code 一句话，四语言各一份）。
 * 后端只下发 code（文案跟界面语言走），见 remote_asr_streaming_provider.rs::warn_outage
 * 与 gateway/src/index.ts 的 up_ok 心跳：
 *   reconnecting       断线重连中（首次）
 *   reconnecting_long  断线较久（≥30s）仍在重连
 *   upstream_stalled   网关↔上游假死，正在重建上游会话
 */
const ASR_WARN_EN = {
  reconnecting: 'Remote recognition disconnected — reconnecting…',
  reconnectingLong: 'Remote recognition has been offline for a while — still reconnecting (recording continues)…',
  upstreamStalled: 'The recognition service stopped responding — rebuilding the connection…',
}
const ASR_WARN_ZH = {
  reconnecting: '远程识别连接中断，正在自动重连…',
  reconnectingLong: '远程识别已断线较久，仍在自动重连（录音未中断）…',
  upstreamStalled: '识别服务长时间无响应，正在重建连接…',
}
const ASR_WARN_KO = {
  reconnecting: '원격 인식 연결이 끊겨 자동으로 다시 연결합니다…',
  reconnectingLong: '원격 인식이 끊긴 지 오래되었습니다. 계속 다시 연결하는 중입니다(녹음은 계속됩니다)…',
  upstreamStalled: '인식 서비스가 응답하지 않습니다. 연결을 다시 만드는 중입니다…',
}
const ASR_WARN_JA = {
  reconnecting: 'リモート認識が切断されました。自動で再接続しています…',
  reconnectingLong: 'リモート認識が長時間切断されています。引き続き再接続中です（録音は継続）…',
  upstreamStalled: '認識サービスが応答していません。接続を再構築しています…',
}

/** code → 文案；未知 code 回退后端给的 message（网关直接下发的中文错误等）。 */
function asrWarningText(
  code: string,
  fallback: string,
  m: { reconnecting: string; reconnectingLong: string; upstreamStalled: string },
): string {
  if (code === 'reconnecting') return m.reconnecting
  if (code === 'reconnecting_long') return m.reconnectingLong
  if (code === 'upstream_stalled') return m.upstreamStalled
  return fallback
}

export const RECORDER_MESSAGES: Record<Language, RecorderMessages> = {
  en: {
    recStop: 'Stop Recording',
    recPreparing: 'Preparing…',
    recStart: 'Start Recording',
    recResume: 'Resume',
    recPause: 'Pause',
    recMuted: 'Muted',
    recMute: 'Mute',
    recTranslate: 'Translate',
    recTranslating: 'Translating',
    recTranslateTitle: 'Real-time translation: show a translation under each recognized segment',
    recSubtitle: 'Subtitle',
    recSubtitleTitle: 'Show desktop subtitle overlay',
    recTargetLang: 'Target language',
    recTranslateAuto: 'Auto both ways',
    recTranslateToEn: 'Into English',
    recTranslateToZh: 'Into Chinese',
    recTranslateEngine: 'Engine',
    recEngineOpus: 'OPUS-MT (fast)',
    recEngineHymt2: 'Hy-MT2 (high quality)',
    recEngineRemote: 'Remote',
    recEngineCustomApi: 'Custom API',
    recRecommended: ' [Recommended]',
    recBadgeLocalFree: 'Offline · Free',
    recBadgeRemote: 'Cloud',
    recBadgeStreaming: 'Streaming',
    recBadgeBatch: 'Batch',
    recStreamingHint: 'Streaming: words appear as you speak; Batch: text appears after each sentence.',
    recLangSupport: 'Languages',
    recRemoteModel: 'Remote Model',
    recRemoteModelDesc: 'Cloud speech recognition via the remote gateway',
    recRemoteModelSelect: 'Select remote model',
    recRemoteNoModels: 'No remote models published in the admin console yet',
    recModelXAsr: 'X-ASR Streaming (ZH/EN)',
    recModelSenseVoice: 'SenseVoice Multilingual',
    recNoModel: 'Not selected',
    recLabelAsr: 'Speech recognition: ',
    recLabelLangs: 'Languages: ',
    recLangsXAsr: 'Chinese / English',
    recLangsSenseVoice: 'ZH / EN / JA / KO / Cantonese',
    recLabelMic: 'Mic: ',
    recLabelSpeaker: 'Speaker: ',
    recNoDevice: 'Not detected',
    recAudioDevices: 'Audio Devices',
    recOpenSoundTitle: 'Open Windows Sound settings (audio devices page)',
    recSourceHint: 'Records both system playback and microphone input',
    recNoModelTitle: 'No speech recognition model installed',
    recNoModelPre: 'Go to',
    recNoModelLink: 'Settings',
    recNoModelPost: 'to download a model (about 600–900 MB). You can start recording and transcribing once it finishes.',
    recPaused: 'Paused',
    recMicShort: 'Mic',
    recSysShort: 'Sys',
    recMicLevelTitle: 'Microphone level (should fluctuate when you speak; a constantly empty bar means no audio is reaching the app)',
    recSysLevelTitle: 'System audio level (should fluctuate while sound is playing)',
    recSetupTitle: 'Before You Start Recording',
    recSetupDesc: 'Choose the model and audio devices for this recording.',
    recAsrModel: 'Speech Recognition Model',
    recXAsrDesc: 'Streaming recognition, Chinese / English',
    recSenseVoiceDesc: 'Multilingual recognition (ZH/EN/JA/KO/Cantonese)',
    recNotDownloaded: 'Not downloaded',
    recMic: 'Microphone',
    recSystemDefault: 'System default (auto-follow)',
    recSystemAudio: 'System Audio',
    recRecogLang: 'Recognition Language',
    recLangAuto: 'Auto Detect',
    recLangZh: '中文',
    recLangEn: 'English',
    recXAsrLangTitle: 'X-ASR is a bilingual Chinese-English model; no language selection needed',
    recLangSupported: 'Supported: {list}',
    recLangAutoOnly: 'This model publishes no language list — only auto-detect is available.',
    recLangResetByModel: 'The new model does not support the previously selected language; reset to auto-detect.',
    recMuteOnStart: 'Mute microphone when recording starts',
    recTranslateCheck: 'Real-time translation (show translation under each segment)',
    recCurrentDefault: 'Current system default: {mic} / {speaker}',
    recSoundSettings: 'Sound Settings',
    recDefaultTitle: 'Rec',
    recSavedToHistory: 'Saved to history',
    recSaveFailed: 'Failed to save transcript',
    recDeviceSwitched: 'Audio device switched',
    recDeviceSwitchedDesc: 'Microphone: {mic} / System audio: {sys}',
    recNone: 'None',
    recDeviceDisconnected: 'Audio device disconnected',
    recWaitingForDevice: 'Waiting for the device to reconnect — recording will not stop',
    recStartFailed: 'Failed to start recording',
    recStopFailed: 'Failed to stop recording',
    recActionFailed: 'Action failed',
    recRecognizing: 'Recognizing speech…',
    recRecognizingHint: 'Text appears here after you pause briefly',
    recAudioSelfTest: 'Voice Model Test',
    recTestTitle: 'Voice Model Test',
    recTestEntryHint: 'Play sample audio to verify capture and recognition',
    recTabAsr: 'Speech Model',
    recTabTranslate: 'Translation',
    recTabAudio: 'Audio Path',
    recInfoToggle: 'Info',
    recInfoRemoteLabel: 'Remote service',
    recInfoRemoteOnline: 'Online',
    recInfoRemoteOffline: 'Offline',
    recInfoRemoteDisabled: 'Disabled',
    recInfoTranslateOff: 'Off',
    recTestDesc: 'The test audio is played through the system speaker and recognized via system audio capture. You can also speak into the microphone.',
    recTestReplay: 'Replay',
    recTestPlaying: 'Playing',
    recTestResultLabel: 'Recognition Result',
    recTestWaiting: 'Waiting for recognition results…',
    recTestLoadingModel: 'Loading recognition model…',
    recTestEnergy: 'Audio Energy',
    recTestErrorTitle: 'Self-test could not start',
    recTestErrorHint: 'Go back, switch to another model, and try again.',
    recTestClose: 'Done',
    recAsrGroupLocal: 'Local models',
    recAsrGroupRemote: 'Cloud models',
    recRemoteBenefits: 'Recommended: higher accuracy, more languages, no download or setup needed. Billed by usage in credits.',
    recRemoteEnableGuide: 'Cloud models are off. Enable them in Account → Remote service for higher accuracy and more languages.',
    recRemoteNoStreaming: 'No streaming cloud models available',
    recRemoteNoBatch: 'No batch cloud models available',
    recOfflineHint: 'Offline re-recognition uses batch models only: whole-file recognition is more accurate.',
    recTestModelLabel: 'Test model',
    recTestStart: 'Start Test',
    recTestRetest: 'Retest',
    recTestIdleHint: 'Pick a model, then click "Start Test"',
    recTestAutoStopped: 'Playback finished; the test stopped automatically. Cloud models are billed by audio duration.',
    recLiveTranscribing: 'Transcribing live…',
    recBackToLive: 'Back to live transcription',
    recAsrReconnecting: ASR_WARN_EN.reconnecting,
    recAsrReconnectingLong: ASR_WARN_EN.reconnectingLong,
    recAsrUpstreamStalled: ASR_WARN_EN.upstreamStalled,
    recAsrWarningText: (code, fallback) => asrWarningText(code, fallback, ASR_WARN_EN),
    recAsrSwitchLabel: 'Speech Recognition Model',
    recAsrSwitching: 'Switching…',
    recAsrSwitched: 'Recognition model switched',
    recAsrSwitchFailed: 'Failed to switch model: {error}',
    recAsrSwitchStreamingOnly: 'Only streaming models can be switched while recording. This model applies to the next recording.',
    recModelPickTips: '💡 Local models run offline for free and nothing is uploaded. Qwen is a multilingual all-rounder — the top pick for mixed Chinese-English meetings. Doubao has the strongest Chinese and dialect recognition. Deepgram is more accurate for English and Western languages but weaker in Chinese (streaming mode requires selecting the language manually).',
    recTranslateModelLabel: 'Translation Model',
    recLangsExpand: 'More',
    recLangsCollapse: 'Less',
    recCreditEstimate: 'Balance {credits} credits · about {duration} of live recognition left',
    recCreditTooLow: 'Not enough credits to start live recognition ({min} credits needed). Recording still works — use offline recognition later.',
    recCreditEstimateHint: 'Estimated from the selected model\u2019s per-minute rate. Silence is billed too, and translation/summary are extra.',
    recDurHourMin: '{h} h {m} min',
    recDurMin: '{m} min',
    recDurSec: '{s} s',
    recAsrStopped: 'Recognition stopped (recording continues)',
    recAsrStoppedCredits: 'Recognition stopped — out of credits. Recording continues; use offline recognition later.',
    recAsrStoppedConfig: 'Recognition stopped — service unavailable. Recording continues.',
  },
  zh: {
    recStop: '停止录音',
    recPreparing: '准备中…',
    recStart: '开始录音',
    recResume: '继续',
    recPause: '暂停',
    recMuted: '已静音',
    recMute: '静音',
    recTranslate: '翻译',
    recTranslating: '翻译中',
    recTranslateTitle: '实时翻译：每段识别结果下方显示译文',
    recSubtitle: '字幕',
    recSubtitleTitle: '显示桌面字幕悬浮窗',
    recTargetLang: '目标语言',
    recTranslateAuto: '自动互译',
    recTranslateToEn: '译成英文',
    recTranslateToZh: '译成中文',
    recTranslateEngine: '翻译引擎',
    recEngineOpus: 'OPUS-MT（快速）',
    recEngineHymt2: 'Hy-MT2（高质量）',
    recEngineRemote: '远程',
    recEngineCustomApi: '自定义 API',
    recRecommended: '【推荐】',
    recBadgeLocalFree: '离线免费',
    recBadgeRemote: '在线远程',
    recBadgeStreaming: '流式',
    recBadgeBatch: '非流式',
    recStreamingHint: '流式：边说边出字；非流式：说完一句话后出字',
    recLangSupport: '支持语言',
    recRemoteModel: '远程模型',
    recRemoteModelDesc: '走远程网关的高精度语音识别',
    recRemoteModelSelect: '选择远程模型',
    recRemoteNoModels: '后台尚未上架可用的远程模型',
    recModelXAsr: 'X-ASR 流式（中英）',
    recModelSenseVoice: 'SenseVoice 多语言',
    recNoModel: '未选择',
    recLabelAsr: '语音识别：',
    recLabelLangs: '语言：',
    recLangsXAsr: '中文 / English',
    recLangsSenseVoice: '中 / 英 / 日 / 韩 / 粤',
    recLabelMic: '录制：',
    recLabelSpeaker: '播放：',
    recNoDevice: '未检测到',
    recAudioDevices: '音频设备',
    recOpenSoundTitle: '打开 Windows 声音设置（音频设备页面）',
    recSourceHint: '同时录制系统播放和麦克风输入的音频',
    recNoModelTitle: '尚未安装语音识别模型',
    recNoModelPre: '请先到',
    recNoModelLink: '设置页',
    recNoModelPost: '下载模型（约 600MB–900MB），下载完成后即可开始录音转写。',
    recPaused: '已暂停',
    recMicShort: '麦',
    recSysShort: '系',
    recMicLevelTitle: '麦克风电平（说话时应有波动；恒为空说明系统未送到声音）',
    recSysLevelTitle: '系统音频电平（播放声音时应有波动）',
    recSetupTitle: '开始录音前确认',
    recSetupDesc: '选择本次录音使用的模型与音频设备。',
    recAsrModel: '语音识别模型',
    recXAsrDesc: '流式识别，中文 / English',
    recSenseVoiceDesc: '多语言识别（中/英/日/韩/粤）',
    recNotDownloaded: '未下载',
    recMic: '麦克风',
    recSystemDefault: '系统默认（自动跟随）',
    recSystemAudio: '系统音频',
    recRecogLang: '识别语言',
    recLangAuto: '自动检测',
    recLangZh: '中文',
    recLangEn: 'English',
    recXAsrLangTitle: 'X-ASR 为中英双语模型，无需选择语言',
    recLangSupported: '支持：{list}',
    recLangAutoOnly: '该模型未提供语言列表，仅支持自动检测。',
    recLangResetByModel: '新模型不支持原先选择的语言，已重置为自动检测。',
    recMuteOnStart: '开始录音时麦克风静音',
    recTranslateCheck: '实时翻译（每段下方显示译文）',
    recCurrentDefault: '当前系统默认：{mic} / {speaker}',
    recSoundSettings: '声音设置',
    recDefaultTitle: 'Rec',
    recSavedToHistory: '已保存到历史记录',
    recSaveFailed: '保存转录记录失败',
    recDeviceSwitched: '音频设备已切换',
    recDeviceSwitchedDesc: '麦克风: {mic} / 系统音频: {sys}',
    recNone: '无',
    recDeviceDisconnected: '音频设备已断开',
    recWaitingForDevice: '正在等待设备恢复，录制不会中断',
    recStartFailed: '启动录音失败',
    recStopFailed: '停止录音失败',
    recActionFailed: '操作失败',
    recRecognizing: '正在识别…',
    recRecognizingHint: '说话稍作停顿，文字会显示在这里',
    recAudioSelfTest: '语音模型测试',
    recTestTitle: '语音模型测试',
    recTestEntryHint: '播放示例音频，测试音频采集与识别是否正常',
    recTabAsr: '语音识别模型',
    recTabTranslate: '翻译模型',
    recTabAudio: '音频通路',
    recInfoToggle: '信息展示',
    recInfoRemoteLabel: '远程服务',
    recInfoRemoteOnline: '在线',
    recInfoRemoteOffline: '离线',
    recInfoRemoteDisabled: '未启用',
    recInfoTranslateOff: '关',
    recTestDesc: '测试音频将通过系统扬声器播放并被系统音频捕获识别，也可以对着麦克风说话。',
    recTestReplay: '重新播放',
    recTestPlaying: '播放中',
    recTestResultLabel: '识别结果',
    recTestWaiting: '等待识别结果…',
    recTestLoadingModel: '正在加载识别模型…',
    recTestEnergy: '音频能量',
    recTestErrorTitle: '自检无法开始',
    recTestErrorHint: '请返回更换模型后再试。',
    recTestClose: '完成',
    recAsrGroupLocal: '本地模型',
    recAsrGroupRemote: '远程模型',
    recRemoteBenefits: '推荐：识别更准、支持更多语言、免下载免配置；按使用量扣积分。',
    recRemoteEnableGuide: '远程模型未开启。可在「用户中心 → 远程服务」开启后使用（识别更准、语言更多）。',
    recRemoteNoStreaming: '暂无可用的流式远程模型',
    recRemoteNoBatch: '暂无可用的非流式远程模型',
    recOfflineHint: '离线重识别只使用非流式模型：整段识别更准确。',
    recTestModelLabel: '测试模型',
    recTestStart: '开始测试',
    recTestRetest: '重新测试',
    recTestIdleHint: '选好模型后点击「开始测试」',
    recTestAutoStopped: '播放结束，测试已自动停止。远程模型按音频时长计费。',
    recLiveTranscribing: '实时转写中…',
    recBackToLive: '回到实时转录页',
    recAsrReconnecting: ASR_WARN_ZH.reconnecting,
    recAsrReconnectingLong: ASR_WARN_ZH.reconnectingLong,
    recAsrUpstreamStalled: ASR_WARN_ZH.upstreamStalled,
    recAsrWarningText: (code, fallback) => asrWarningText(code, fallback, ASR_WARN_ZH),
    recAsrSwitchLabel: '语音识别模型',
    recAsrSwitching: '切换中…',
    recAsrSwitched: '已切换识别模型',
    recAsrSwitchFailed: '切换模型失败：{error}',
    recAsrSwitchStreamingOnly: '录音中仅支持切换流式模型；该模型将在下次录音生效。',
    recModelPickTips: '💡 本地模型离线免费、隐私不上传；Qwen（千问）多语言全能，中英混说会议首选；Doubao（豆包）中文与方言识别最强；Deepgram 英文及西方语言更准，但中文支持较弱（流式模式需手动指定语言）。',
    recTranslateModelLabel: '翻译模型',
    recLangsExpand: '展开',
    recLangsCollapse: '收起',
    recCreditEstimate: '余额 {credits} 积分 · 约可录 {duration}',
    recCreditTooLow: '余额不足，无法开始实时识别（至少需要 {min} 积分）。录音不受影响，可稍后用离线识别补齐。',
    recCreditEstimateHint: '按所选识别模型的单价估算；静音同样计费，实时翻译与会议总结另计。',
    recDurHourMin: '{h} 小时 {m} 分钟',
    recDurMin: '{m} 分钟',
    recDurSec: '{s} 秒',
    recAsrStopped: '识别已停止（录音继续）',
    recAsrStoppedCredits: '识别已停止：积分不足。录音继续，可稍后用离线识别补齐。',
    recAsrStoppedConfig: '识别已停止：服务不可用。录音继续。',
  },
  ko: {
    recStop: '녹음 중지',
    recPreparing: '준비 중…',
    recStart: '녹음 시작',
    recResume: '계속',
    recPause: '일시 정지',
    recMuted: '음소거됨',
    recMute: '음소거',
    recTranslate: '번역',
    recTranslating: '번역 중',
    recTranslateTitle: '실시간 번역: 인식된 각 문장 아래에 번역을 표시합니다',
    recSubtitle: '자막',
    recSubtitleTitle: '데스크톱 자막 오버레이 표시',
    recTargetLang: '대상 언어',
    recTranslateAuto: '자동 양방향',
    recTranslateToEn: '영어로 번역',
    recTranslateToZh: '중국어로 번역',
    recTranslateEngine: '번역 엔진',
    recEngineOpus: 'OPUS-MT (빠름)',
    recEngineHymt2: 'Hy-MT2 (고품질)',
    recEngineRemote: '원격',
    recEngineCustomApi: '사용자 지정 API',
    recRecommended: '【추천】',
    recBadgeLocalFree: '오프라인 무료',
    recBadgeRemote: '온라인 원격',
    recBadgeStreaming: '스트리밍',
    recBadgeBatch: '배치',
    recStreamingHint: '스트리밍: 말하는 동안 실시간 표시, 배치: 문장이 끝난 후 표시',
    recLangSupport: '지원 언어',
    recRemoteModel: '원격 모델',
    recRemoteModelDesc: '원격 게이트웨이의 고정밀 음성 인식',
    recRemoteModelSelect: '원격 모델 선택',
    recRemoteNoModels: '관리 콘솔에 공개된 원격 모델이 없습니다',
    recModelXAsr: 'X-ASR 스트리밍(중/영)',
    recModelSenseVoice: 'SenseVoice 다국어',
    recNoModel: '선택되지 않음',
    recLabelAsr: '음성 인식: ',
    recLabelLangs: '언어: ',
    recLangsXAsr: '중국어 / 영어',
    recLangsSenseVoice: '중 / 영 / 일 / 한 / 광둥어',
    recLabelMic: '마이크: ',
    recLabelSpeaker: '스피커: ',
    recNoDevice: '감지되지 않음',
    recAudioDevices: '오디오 기기',
    recOpenSoundTitle: 'Windows 소리 설정 열기(오디오 기기 페이지)',
    recSourceHint: '시스템 재생 음성과 마이크 입력을 함께 녹음합니다',
    recNoModelTitle: '음성 인식 모델이 설치되어 있지 않습니다',
    recNoModelPre: '먼저',
    recNoModelLink: '설정',
    recNoModelPost: '에서 모델을 다운로드하세요(약 600~900MB). 다운로드가 완료되면 녹음 받아쓰기를 시작할 수 있습니다.',
    recPaused: '일시 정지됨',
    recMicShort: 'Mic',
    recSysShort: 'Sys',
    recMicLevelTitle: '마이크 레벨(말할 때 움직여야 합니다. 계속 비어 있으면 시스템에서 소리가 전달되지 않는 것입니다)',
    recSysLevelTitle: '시스템 오디오 레벨(소리 재생 시 움직여야 합니다)',
    recSetupTitle: '녹음 시작 전 확인',
    recSetupDesc: '이번 녹음에 사용할 모델과 오디오 기기를 선택하세요.',
    recAsrModel: '음성 인식 모델',
    recXAsrDesc: '스트리밍 인식, 중국어 / 영어',
    recSenseVoiceDesc: '다국어 인식(중/영/일/한/광둥어)',
    recNotDownloaded: '다운로드되지 않음',
    recMic: '마이크',
    recSystemDefault: '시스템 기본값(자동 추적)',
    recSystemAudio: '시스템 오디오',
    recRecogLang: '인식 언어',
    recLangAuto: '자동 감지',
    recLangZh: '中文',
    recLangEn: 'English',
    recXAsrLangTitle: 'X-ASR는 중국어·영어 이중 언어 모델이므로 언어를 선택할 필요가 없습니다',
    recLangSupported: '지원: {list}',
    recLangAutoOnly: '이 모델은 언어 목록을 제공하지 않아 자동 감지만 지원합니다.',
    recLangResetByModel: '새 모델이 이전에 선택한 언어를 지원하지 않아 자동 감지로 재설정했습니다.',
    recMuteOnStart: '녹음 시작 시 마이크 음소거',
    recTranslateCheck: '실시간 번역(각 문장 아래에 번역 표시)',
    recCurrentDefault: '현재 시스템 기본값: {mic} / {speaker}',
    recSoundSettings: '소리 설정',
    recDefaultTitle: 'Rec',
    recSavedToHistory: '기록에 저장했습니다',
    recSaveFailed: '받아쓰기 기록 저장에 실패했습니다',
    recDeviceSwitched: '오디오 기기가 전환되었습니다',
    recDeviceSwitchedDesc: '마이크: {mic} / 시스템 오디오: {sys}',
    recNone: '없음',
    recDeviceDisconnected: '오디오 기기 연결이 끊겼습니다',
    recWaitingForDevice: '기기가 복구되기를 기다리는 중입니다. 녹음은 중단되지 않습니다',
    recStartFailed: '녹음 시작에 실패했습니다',
    recStopFailed: '녹음 중지에 실패했습니다',
    recActionFailed: '작업에 실패했습니다',
    recRecognizing: '음성 인식 중…',
    recRecognizingHint: '잠시 말을 멈추면 여기에 텍스트가 표시됩니다',
    recAudioSelfTest: '음성 모델 테스트',
    recTestTitle: '음성 모델 테스트',
    recTestEntryHint: '샘플 오디오를 재생하여 오디오 캡처와 인식이 정상인지 테스트합니다',
    recTabAsr: '음성 인식 모델',
    recTabTranslate: '번역 모델',
    recTabAudio: '오디오 경로',
    recInfoToggle: '정보',
    recInfoRemoteLabel: '원격 서비스',
    recInfoRemoteOnline: '온라인',
    recInfoRemoteOffline: '오프라인',
    recInfoRemoteDisabled: '비활성화됨',
    recInfoTranslateOff: '끔',
    recTestDesc: '테스트 오디오가 시스템 스피커로 재생되어 시스템 오디오 캡처를 통해 인식됩니다. 마이크에 대고 말할 수도 있습니다.',
    recTestReplay: '다시 재생',
    recTestPlaying: '재생 중',
    recTestResultLabel: '인식 결과',
    recTestWaiting: '인식 결과를 기다리는 중…',
    recTestLoadingModel: '인식 모델을 불러오는 중…',
    recTestEnergy: '오디오 에너지',
    recTestErrorTitle: '자체 테스트를 시작할 수 없습니다',
    recTestErrorHint: '돌아가서 다른 모델로 변경한 후 다시 시도하세요.',
    recTestClose: '완료',
    recAsrGroupLocal: '로컬 모델',
    recAsrGroupRemote: '클라우드 모델',
    recRemoteBenefits: '추천: 더 높은 정확도, 더 많은 언어 지원, 다운로드·설정 불필요. 사용량에 따라 크레딧 차감.',
    recRemoteEnableGuide: '클라우드 모델이 꺼져 있습니다. 계정 → 원격 서비스에서 활성화하면 더 정확하고 많은 언어를 사용할 수 있습니다.',
    recRemoteNoStreaming: '사용 가능한 스트리밍 클라우드 모델이 없습니다',
    recRemoteNoBatch: '사용 가능한 배치 클라우드 모델이 없습니다',
    recOfflineHint: '오프라인 재인식은 배치 모델만 사용합니다: 전체 파일 인식이 더 정확합니다.',
    recTestModelLabel: '테스트 모델',
    recTestStart: '테스트 시작',
    recTestRetest: '다시 테스트',
    recTestIdleHint: '모델을 선택한 후 "테스트 시작"을 누르세요',
    recTestAutoStopped: '재생이 끝나 테스트가 자동으로 중지되었습니다. 원격 모델은 오디오 길이 기준으로 과금됩니다.',
    recLiveTranscribing: '실시간 전사 중…',
    recBackToLive: '실시간 전사 화면으로 돌아가기',
    recAsrReconnecting: ASR_WARN_KO.reconnecting,
    recAsrReconnectingLong: ASR_WARN_KO.reconnectingLong,
    recAsrUpstreamStalled: ASR_WARN_KO.upstreamStalled,
    recAsrWarningText: (code, fallback) => asrWarningText(code, fallback, ASR_WARN_KO),
    recAsrSwitchLabel: '음성 인식 모델',
    recAsrSwitching: '전환 중…',
    recAsrSwitched: '인식 모델을 전환했습니다',
    recAsrSwitchFailed: '모델 전환 실패: {error}',
    recAsrSwitchStreamingOnly: '녹음 중에는 스트리밍 모델만 전환할 수 있습니다. 이 모델은 다음 녹음에 적용됩니다.',
    recModelPickTips: '💡 로컬 모델은 오프라인 무료이며 음성이 업로드되지 않습니다. Qwen(첸원)은 다국어 만능으로 중·영 혼합 회의에 가장 적합하고, Doubao(더우바오)는 중국어와 방언 인식이 가장 뛰어나며, Deepgram은 영어 등 서양 언어에 더 정확하지만 중국어 지원은 약합니다(스트리밍 모드에서는 언어를 수동으로 지정해야 합니다).',
    recTranslateModelLabel: '번역 모델',
    recLangsExpand: '더 보기',
    recLangsCollapse: '접기',
    recCreditEstimate: '잔액 {credits} 크레딧 · 약 {duration} 실시간 인식 가능',
    recCreditTooLow: '크레딧이 부족하여 실시간 인식을 시작할 수 없습니다({min} 크레딧 필요). 녹음은 계속되며 나중에 오프라인 인식을 사용할 수 있습니다.',
    recCreditEstimateHint: '선택한 인식 모델의 분당 단가 기준입니다. 무음도 과금되며 번역·요약은 별도입니다.',
    recDurHourMin: '{h}시간 {m}분',
    recDurMin: '{m}분',
    recDurSec: '{s}초',
    recAsrStopped: '인식 중지됨(녹음은 계속)',
    recAsrStoppedCredits: '인식 중지: 크레딧 부족. 녹음은 계속되며 나중에 오프라인 인식을 사용할 수 있습니다.',
    recAsrStoppedConfig: '인식 중지: 서비스를 사용할 수 없습니다. 녹음은 계속됩니다.',
  },
  ja: {
    recStop: '録音停止',
    recPreparing: '準備中…',
    recStart: '録音開始',
    recResume: '再開',
    recPause: '一時停止',
    recMuted: 'ミュート中',
    recMute: 'ミュート',
    recTranslate: '翻訳',
    recTranslating: '翻訳中',
    recTranslateTitle: 'リアルタイム翻訳：認識された各セグメントの下に訳文を表示します',
    recSubtitle: '字幕',
    recSubtitleTitle: 'デスクトップ字幕オーバーレイを表示',
    recTargetLang: '対象言語',
    recTranslateAuto: '自動で双方向',
    recTranslateToEn: '英語に翻訳',
    recTranslateToZh: '中国語に翻訳',
    recTranslateEngine: '翻訳エンジン',
    recEngineOpus: 'OPUS-MT（高速）',
    recEngineHymt2: 'Hy-MT2（高品質）',
    recEngineRemote: 'リモート',
    recEngineCustomApi: 'カスタム API',
    recRecommended: '【おすすめ】',
    recBadgeLocalFree: 'オフライン無料',
    recBadgeRemote: 'オンライン',
    recBadgeStreaming: 'ストリーミング',
    recBadgeBatch: 'バッチ',
    recStreamingHint: 'ストリーミング：話しながら逐字表示、バッチ：一文話し終わってから表示',
    recLangSupport: '対応言語',
    recRemoteModel: 'リモートモデル',
    recRemoteModelDesc: 'リモートゲートウェイの高精度音声認識',
    recRemoteModelSelect: 'リモートモデルを選択',
    recRemoteNoModels: '管理コンソールで利用可能なリモートモデルがまだありません',
    recModelXAsr: 'X-ASR ストリーミング(中/英)',
    recModelSenseVoice: 'SenseVoice 多言語',
    recNoModel: '未選択',
    recLabelAsr: '音声認識: ',
    recLabelLangs: '言語: ',
    recLangsXAsr: '中国語 / 英語',
    recLangsSenseVoice: '中 / 英 / 日 / 韓 / 広東語',
    recLabelMic: 'マイク: ',
    recLabelSpeaker: 'スピーカー: ',
    recNoDevice: '未検出',
    recAudioDevices: 'オーディオデバイス',
    recOpenSoundTitle: 'Windows のサウンド設定（オーディオデバイスページ）を開きます',
    recSourceHint: 'システム再生音とマイク入力を同時に録音します',
    recNoModelTitle: '音声認識モデルがインストールされていません',
    recNoModelPre: 'まず',
    recNoModelLink: '設定ページ',
    recNoModelPost: 'でモデルをダウンロードしてください（約 600〜900MB）。完了後に録音と文字起こしを開始できます。',
    recPaused: '一時停止中',
    recMicShort: 'Mic',
    recSysShort: 'Sys',
    recMicLevelTitle: 'マイクレベル（話すと変動します。常に空の場合は音声が届いていません）',
    recSysLevelTitle: 'システム音声レベル（音の再生中に変動します）',
    recSetupTitle: '録音開始前の確認',
    recSetupDesc: 'この録音で使用するモデルとオーディオデバイスを選択してください。',
    recAsrModel: '音声認識モデル',
    recXAsrDesc: 'ストリーミング認識、中国語 / 英語',
    recSenseVoiceDesc: '多言語認識（中/英/日/韓/広東語）',
    recNotDownloaded: '未ダウンロード',
    recMic: 'マイク',
    recSystemDefault: 'システムデフォルト（自動追従）',
    recSystemAudio: 'システム音声',
    recRecogLang: '認識言語',
    recLangAuto: '自動検出',
    recLangZh: '中文',
    recLangEn: 'English',
    recXAsrLangTitle: 'X-ASR は中国語・英語のバイリンガルモデルのため、言語選択は不要です',
    recLangSupported: '対応: {list}',
    recLangAutoOnly: 'このモデルは言語リストを公開していないため、自動検出のみ利用できます。',
    recLangResetByModel: '新しいモデルが以前の言語に対応していないため、自動検出に戻しました。',
    recMuteOnStart: '録音開始時にマイクをミュート',
    recTranslateCheck: 'リアルタイム翻訳（各セグメントの下に訳文を表示）',
    recCurrentDefault: '現在のシステムデフォルト：{mic} / {speaker}',
    recSoundSettings: 'サウンド設定',
    recDefaultTitle: 'Rec',
    recSavedToHistory: '履歴に保存しました',
    recSaveFailed: '文字起こしの保存に失敗しました',
    recDeviceSwitched: 'オーディオデバイスが切り替わりました',
    recDeviceSwitchedDesc: 'マイク: {mic} / システム音声: {sys}',
    recNone: 'なし',
    recDeviceDisconnected: 'オーディオデバイスが切断されました',
    recWaitingForDevice: 'デバイスの復旧を待っています。録音は中断されません',
    recStartFailed: '録音の開始に失敗しました',
    recStopFailed: '録音の停止に失敗しました',
    recActionFailed: '操作に失敗しました',
    recRecognizing: '音声認識中…',
    recRecognizingHint: '少し間を置くと、ここに文字が表示されます',
    recAudioSelfTest: '音声モデルテスト',
    recTestTitle: '音声モデルテスト',
    recTestEntryHint: 'サンプル音声を再生して、音声の収録と認識が正常かテストします',
    recTabAsr: '音声認識モデル',
    recTabTranslate: '翻訳モデル',
    recTabAudio: 'オーディオ経路',
    recInfoToggle: '情報',
    recInfoRemoteLabel: 'リモートサービス',
    recInfoRemoteOnline: 'オンライン',
    recInfoRemoteOffline: 'オフライン',
    recInfoRemoteDisabled: '無効',
    recInfoTranslateOff: 'オフ',
    recTestDesc: 'テスト音声はシステムスピーカーから再生され、システム音声キャプチャで認識されます。マイクに向かって話すこともできます。',
    recTestReplay: '再再生',
    recTestPlaying: '再生中',
    recTestResultLabel: '認識結果',
    recTestWaiting: '認識結果を待っています…',
    recTestLoadingModel: '認識モデルを読み込んでいます…',
    recTestEnergy: 'オーディオエネルギー',
    recTestErrorTitle: '自己テストを開始できません',
    recTestErrorHint: '戻って別のモデルに変更してから再試行してください。',
    recTestClose: '完了',
    recAsrGroupLocal: 'ローカルモデル',
    recAsrGroupRemote: 'クラウドモデル',
    recRemoteBenefits: 'おすすめ：高精度・多言語対応・ダウンロードや設定不要。使用量に応じてクレジット消費。',
    recRemoteEnableGuide: 'クラウドモデルが無効です。「アカウント → リモートサービス」で有効にすると、より高精度で多くの言語が使えます。',
    recRemoteNoStreaming: '利用可能なストリーミングクラウドモデルがありません',
    recRemoteNoBatch: '利用可能なバッチクラウドモデルがありません',
    recOfflineHint: 'オフライン再認識はバッチモデルのみ使用します：ファイル全体の認識がより正確です。',
    recTestModelLabel: 'テストモデル',
    recTestStart: 'テスト開始',
    recTestRetest: '再テスト',
    recTestIdleHint: 'モデルを選んで「テスト開始」をクリック',
    recTestAutoStopped: '再生が終了し、テストは自動停止しました。リモートモデルは音声の長さで課金されます。',
    recLiveTranscribing: 'リアルタイム文字起こし中…',
    recBackToLive: 'リアルタイム文字起こし画面へ戻る',
    recAsrReconnecting: ASR_WARN_JA.reconnecting,
    recAsrReconnectingLong: ASR_WARN_JA.reconnectingLong,
    recAsrUpstreamStalled: ASR_WARN_JA.upstreamStalled,
    recAsrWarningText: (code, fallback) => asrWarningText(code, fallback, ASR_WARN_JA),
    recAsrSwitchLabel: '音声認識モデル',
    recAsrSwitching: '切り替え中…',
    recAsrSwitched: '認識モデルを切り替えました',
    recAsrSwitchFailed: 'モデルの切り替えに失敗：{error}',
    recAsrSwitchStreamingOnly: '録音中はストリーミングモデルのみ切り替え可能です。このモデルは次回の録音に適用されます。',
    recModelPickTips: '💡 ローカルモデルはオフライン無料で、音声はアップロードされません。Qwen（千問）は多言語オールラウンダーで中英混在の会議に最適、Doubao（豆包）は中国語・方言の認識が最強、Deepgramは英語など欧米言語は高精度ですが中国語対応は弱めです（ストリーミングモードでは言語の手動指定が必要）。',
    recTranslateModelLabel: '翻訳モデル',
    recLangsExpand: 'すべて表示',
    recLangsCollapse: '折りたたむ',
    recCreditEstimate: '残高 {credits} クレジット · 約 {duration} 認識可能',
    recCreditTooLow: 'クレジット不足のためリアルタイム認識を開始できません（{min} クレジット必要）。録音は継続され、後でオフライン認識を利用できます。',
    recCreditEstimateHint: '選択した認識モデルの分単価による概算です。無音も課金され、翻訳・要約は別途です。',
    recDurHourMin: '{h}時間{m}分',
    recDurMin: '{m}分',
    recDurSec: '{s}秒',
    recAsrStopped: '認識が停止しました（録音は継続）',
    recAsrStoppedCredits: '認識が停止しました：クレジット不足。録音は継続され、後でオフライン認識を利用できます。',
    recAsrStoppedConfig: '認識が停止しました：サービス利用不可。録音は継続します。',
  },
}
