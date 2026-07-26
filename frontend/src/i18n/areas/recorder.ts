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
  recTargetLang: string
  recTranslateAuto: string
  recTranslateToEn: string
  recTranslateToZh: string
  recTranslateEngine: string
  recEngineOpus: string
  recEngineHymt2: string
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
  recMuteOnStart: string
  recTranslateCheck: string
  recCurrentDefault: string
  recSoundSettings: string
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
    recTargetLang: 'Target language',
    recTranslateAuto: 'Auto both ways',
    recTranslateToEn: 'Into English',
    recTranslateToZh: 'Into Chinese',
    recTranslateEngine: 'Engine',
    recEngineOpus: 'OPUS-MT (fast)',
    recEngineHymt2: 'Hy-MT2 (high quality)',
    recModelXAsr: 'X-ASR Streaming (ZH/EN)',
    recModelSenseVoice: 'SenseVoice Multilingual',
    recNoModel: 'Not selected',
    recLabelAsr: 'ASR: ',
    recLabelLangs: 'Languages: ',
    recLangsXAsr: 'Chinese / English',
    recLangsSenseVoice: 'ZH / EN / JA / KO / Cantonese',
    recLabelMic: 'Mic: ',
    recLabelSpeaker: 'Speaker: ',
    recNoDevice: 'Not detected',
    recAudioDevices: 'Audio Devices',
    recOpenSoundTitle: 'Open Windows Sound settings (audio devices page)',
    recSourceHint: 'Records both system playback and microphone input',
    recNoModelTitle: 'No ASR model installed',
    recNoModelPre: 'Go to',
    recNoModelLink: 'Settings',
    recNoModelPost: 'to download a model (about 600–900 MB). You can start recording and transcribing once it finishes.',
    recPaused: 'Paused',
    recMicShort: 'Mic',
    recSysShort: 'Sys',
    recMicLevelTitle: 'Microphone level (should fluctuate when you speak; a constantly empty bar means no audio is reaching the app)',
    recSysLevelTitle: 'System audio level (should fluctuate while sound is playing)',
    recSetupTitle: 'Before You Start Recording',
    recSetupDesc: 'Choose the ASR model and audio devices for this recording. Audio is processed locally only.',
    recAsrModel: 'ASR Model',
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
    recMuteOnStart: 'Mute microphone when recording starts',
    recTranslateCheck: 'Real-time translation (show translation under each segment)',
    recCurrentDefault: 'Current system default: {mic} / {speaker}',
    recSoundSettings: 'Sound Settings',
    recDefaultTitle: 'Recording',
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
    recTargetLang: '目标语言',
    recTranslateAuto: '自动互译',
    recTranslateToEn: '译成英文',
    recTranslateToZh: '译成中文',
    recTranslateEngine: '翻译引擎',
    recEngineOpus: 'OPUS-MT（快速）',
    recEngineHymt2: 'Hy-MT2（高质量）',
    recModelXAsr: 'X-ASR 流式（中英）',
    recModelSenseVoice: 'SenseVoice 多语言',
    recNoModel: '未选择',
    recLabelAsr: 'ASR：',
    recLabelLangs: '语言：',
    recLangsXAsr: '中文 / English',
    recLangsSenseVoice: '中 / 英 / 日 / 韩 / 粤',
    recLabelMic: '录制：',
    recLabelSpeaker: '播放：',
    recNoDevice: '未检测到',
    recAudioDevices: '音频设备',
    recOpenSoundTitle: '打开 Windows 声音设置（音频设备页面）',
    recSourceHint: '同时录制系统播放和麦克风输入的音频',
    recNoModelTitle: '尚未安装 ASR 模型',
    recNoModelPre: '请先到',
    recNoModelLink: '设置页',
    recNoModelPost: '下载模型（约 600MB–900MB），下载完成后即可开始录音转写。',
    recPaused: '已暂停',
    recMicShort: '麦',
    recSysShort: '系',
    recMicLevelTitle: '麦克风电平（说话时应有波动；恒为空说明系统未送到声音）',
    recSysLevelTitle: '系统音频电平（播放声音时应有波动）',
    recSetupTitle: '开始录音前确认',
    recSetupDesc: '选择本次录音使用的 ASR 模型与音频设备，音频仅在本地处理。',
    recAsrModel: 'ASR 模型',
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
    recMuteOnStart: '开始录音时麦克风静音',
    recTranslateCheck: '实时翻译（每段下方显示译文）',
    recCurrentDefault: '当前系统默认：{mic} / {speaker}',
    recSoundSettings: '声音设置',
    recDefaultTitle: '录音',
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
    recTargetLang: '대상 언어',
    recTranslateAuto: '자동 양방향',
    recTranslateToEn: '영어로 번역',
    recTranslateToZh: '중국어로 번역',
    recTranslateEngine: '번역 엔진',
    recEngineOpus: 'OPUS-MT (빠름)',
    recEngineHymt2: 'Hy-MT2 (고품질)',
    recModelXAsr: 'X-ASR 스트리밍(중/영)',
    recModelSenseVoice: 'SenseVoice 다국어',
    recNoModel: '선택되지 않음',
    recLabelAsr: 'ASR: ',
    recLabelLangs: '언어: ',
    recLangsXAsr: '중국어 / 영어',
    recLangsSenseVoice: '중 / 영 / 일 / 한 / 광둥어',
    recLabelMic: '마이크: ',
    recLabelSpeaker: '스피커: ',
    recNoDevice: '감지되지 않음',
    recAudioDevices: '오디오 기기',
    recOpenSoundTitle: 'Windows 소리 설정 열기(오디오 기기 페이지)',
    recSourceHint: '시스템 재생 음성과 마이크 입력을 함께 녹음합니다',
    recNoModelTitle: 'ASR 모델이 설치되어 있지 않습니다',
    recNoModelPre: '먼저',
    recNoModelLink: '설정',
    recNoModelPost: '에서 모델을 다운로드하세요(약 600~900MB). 다운로드가 완료되면 녹음 받아쓰기를 시작할 수 있습니다.',
    recPaused: '일시 정지됨',
    recMicShort: 'Mic',
    recSysShort: 'Sys',
    recMicLevelTitle: '마이크 레벨(말할 때 움직여야 합니다. 계속 비어 있으면 시스템에서 소리가 전달되지 않는 것입니다)',
    recSysLevelTitle: '시스템 오디오 레벨(소리 재생 시 움직여야 합니다)',
    recSetupTitle: '녹음 시작 전 확인',
    recSetupDesc: '이번 녹음에 사용할 ASR 모델과 오디오 기기를 선택하세요. 오디오는 로컬에서만 처리됩니다.',
    recAsrModel: 'ASR 모델',
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
    recMuteOnStart: '녹음 시작 시 마이크 음소거',
    recTranslateCheck: '실시간 번역(각 문장 아래에 번역 표시)',
    recCurrentDefault: '현재 시스템 기본값: {mic} / {speaker}',
    recSoundSettings: '소리 설정',
    recDefaultTitle: '녹음',
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
    recTargetLang: '対象言語',
    recTranslateAuto: '自動で双方向',
    recTranslateToEn: '英語に翻訳',
    recTranslateToZh: '中国語に翻訳',
    recTranslateEngine: '翻訳エンジン',
    recEngineOpus: 'OPUS-MT（高速）',
    recEngineHymt2: 'Hy-MT2（高品質）',
    recModelXAsr: 'X-ASR ストリーミング(中/英)',
    recModelSenseVoice: 'SenseVoice 多言語',
    recNoModel: '未選択',
    recLabelAsr: 'ASR: ',
    recLabelLangs: '言語: ',
    recLangsXAsr: '中国語 / 英語',
    recLangsSenseVoice: '中 / 英 / 日 / 韓 / 広東語',
    recLabelMic: 'マイク: ',
    recLabelSpeaker: 'スピーカー: ',
    recNoDevice: '未検出',
    recAudioDevices: 'オーディオデバイス',
    recOpenSoundTitle: 'Windows のサウンド設定（オーディオデバイスページ）を開きます',
    recSourceHint: 'システム再生音とマイク入力を同時に録音します',
    recNoModelTitle: 'ASR モデルがインストールされていません',
    recNoModelPre: 'まず',
    recNoModelLink: '設定ページ',
    recNoModelPost: 'でモデルをダウンロードしてください（約 600〜900MB）。完了後に録音と文字起こしを開始できます。',
    recPaused: '一時停止中',
    recMicShort: 'Mic',
    recSysShort: 'Sys',
    recMicLevelTitle: 'マイクレベル（話すと変動します。常に空の場合は音声が届いていません）',
    recSysLevelTitle: 'システム音声レベル（音の再生中に変動します）',
    recSetupTitle: '録音開始前の確認',
    recSetupDesc: 'この録音で使用する ASR モデルとオーディオデバイスを選択してください。音声はローカルでのみ処理されます。',
    recAsrModel: 'ASR モデル',
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
    recMuteOnStart: '録音開始時にマイクをミュート',
    recTranslateCheck: 'リアルタイム翻訳（各セグメントの下に訳文を表示）',
    recCurrentDefault: '現在のシステムデフォルト：{mic} / {speaker}',
    recSoundSettings: 'サウンド設定',
    recDefaultTitle: '録音',
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
  },
}
