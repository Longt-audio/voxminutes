import type { Language } from '../languages'

/** 设置页文案：tab、模型下载、音频、导出、远程 ASR、Advanced 占位。 */
export interface SettingsMessages {
  setPageSubtitle: string
  setTabModels: string
  setTabAudioExport: string
  setTabApi: string
  setTabAdvanced: string
  setAsrModels: string
  setGroupAsr: string
  setGroupTranslate: string
  setGroupSummary: string
  setModelDir: string
  setModelDirHint: string
  setNoModels: string
  setInstalled: string
  setDownloading: string
  setNotInstalled: string
  setDownloadingPending: string
  setStageDownloading: string
  setStageExtracting: string
  setStageVerifying: string
  setDownloadDone: string
  setDownloadFailed: string
  setImport: string
  setImportDone: string
  setImportFailed: string
  setReopenOnboarding: string
  setDeleteConfirm: string
  setModelDeleted: string
  setDeleteFailed: string
  setModelNote1: string
  setModelNote2: string
  setModelDescSenseVoice: string
  setModelDescXAsr: string
  setModelDescOpusMt: string
  setModelDescHymt2: string
  setModelDescQwen25: string
  setModelDescQwen3: string
  setModelDescGemma: string
  setModelNameSenseVoice: string
  setModelNameXAsr: string
  setModelNameOpusZhEn: string
  setModelNameOpusEnZh: string
  setModelNameHymt2: string
  setModelNameQwen25: string
  setModelNameQwen3: string
  setModelNameGemma: string
  setAudio: string
  setMicrophone: string
  setNoDevice: string
  setSystemAudio: string
  setOpenSoundSettings: string
  setDeviceHint: string
  setRecordingExport: string
  setRecordingsFolder: string
  setChange: string
  setOpenFolder: string
  setExportDir: string
  setExportDirUnset: string
  setFolderUpdated: string
  setFolderFailed: string
  setOpenFolderFailed: string
  setExportDirUpdated: string
  setExportDirFailed: string
  setRemoteAsrTitle: string
  setRemoteAsrAddress: string
  setSaving: string
  setChecking: string
  setTestConnection: string
  setOnline: string
  setOffline: string
  setModelLabel: string
  setEnterEndpoint: string
  setEndpointSaved: string
  setSaveFailed: string
  setRemoteNote: string
  setAdvancedTitle: string
  setPlanned: string
  setAdvTtsName: string
  setAdvTtsDesc: string
  setAdvPttName: string
  setAdvPttDesc: string
  setAdvSubtitleName: string
  setAdvSubtitleDesc: string
  setAdvSelectionName: string
  setAdvSelectionDesc: string
  setAdvLiveSummaryName: string
  setAdvLiveSummaryDesc: string
  setAdvSpeakerIdName: string
  setAdvSpeakerIdDesc: string
  setAdvCloudAsrName: string
  setAdvCloudAsrDesc: string
  setAdvHelpName: string
  setAdvHelpDesc: string
}

export const SETTINGS_MESSAGES: Record<Language, SettingsMessages> = {
  en: {
    setPageSubtitle: "Model, audio and export preferences",
    setTabModels: "Models",
    setTabAudioExport: "Audio & export",
    setTabApi: "API",
    setTabAdvanced: "Advanced",
    setAsrModels: "Models",
    setGroupAsr: "ASR models",
    setGroupTranslate: "Translation models",
    setGroupSummary: "Summary models",
    setModelDir: "Model directory",
    setModelDirHint: "Where model files are stored",
    setNoModels: "No models available for download",
    setInstalled: "Installed",
    setDownloading: "Downloading",
    setNotInstalled: "Not installed",
    setDownloadingPending: "Downloading…",
    setStageDownloading: "Downloading {percent}%",
    setStageExtracting: "Extracting…",
    setStageVerifying: "Verifying…",
    setDownloadDone: "Model downloaded",
    setDownloadFailed: "Model download failed",
    setImport: "Import",
    setImportDone: "Model imported",
    setImportFailed: "Model import failed: {error}",
    setReopenOnboarding: "Reopen setup guide",
    setDeleteConfirm: "Delete model \"{name}\"? You will need to download it again before using it.",
    setModelDeleted: "Model deleted",
    setDeleteFailed: "Failed to delete: {error}",
    setModelNote1: "Models are not bundled with the app, so download one before first use. Models are downloaded from official GitHub Releases and automatically switch to a mirror when the network is poor.",
    setModelNote2: "Translation models run on-device and are used by the Translate page and by translation during real-time transcription.",
    setModelDescSenseVoice: "Multilingual ASR (zh/en/ja/ko/yue), VAD pseudo-streaming",
    setModelDescXAsr: "Chinese–English fully-streaming ASR with lower latency",
    setModelDescOpusMt: "OPUS-MT Chinese–English translation, 113 MB, fast",
    setModelDescHymt2: "Tencent Hunyuan Hy-MT2, 1.1 GB, higher quality, 13 target languages",
    setModelDescQwen25: "Local meeting summary LLM (Qwen2.5-3B, 2.1 GB, smaller and faster)",
    setModelDescQwen3: "Local meeting summary LLM (Qwen3-4B-2507, better quality)",
    setModelDescGemma: "Local meeting summary LLM (Gemma-3-4B, stronger in English)",
    setModelNameSenseVoice: "SenseVoice Multilingual ASR",
    setModelNameXAsr: "X-ASR Streaming ASR (ZH/EN, 480 ms)",
    setModelNameOpusZhEn: "OPUS-MT Translation (ZH → EN)",
    setModelNameOpusEnZh: "OPUS-MT Translation (EN → ZH)",
    setModelNameHymt2: "Hy-MT2-1.8B (high-quality translation)",
    setModelNameQwen25: "Qwen2.5-3B-Instruct (meeting summary)",
    setModelNameQwen3: "Qwen3-4B-Instruct-2507 (meeting summary)",
    setModelNameGemma: "Gemma-3-4B-it (meeting summary)",
    setAudio: "Audio",
    setMicrophone: "Microphone",
    setNoDevice: "Not detected",
    setSystemAudio: "System audio",
    setOpenSoundSettings: "Open system sound settings",
    setDeviceHint: "Recording devices are selected in the recording controls on the Transcribe page",
    setRecordingExport: "Recording & export",
    setRecordingsFolder: "Recording folder",
    setChange: "Change…",
    setOpenFolder: "Open folder",
    setExportDir: "Default export folder",
    setExportDirUnset: "Not set (defaults to the recordings folder)",
    setFolderUpdated: "Recording folder updated",
    setFolderFailed: "Failed to set recording folder: {error}",
    setOpenFolderFailed: "Failed to open folder: {error}",
    setExportDirUpdated: "Default export folder updated",
    setExportDirFailed: "Failed to set export folder: {error}",
    setRemoteAsrTitle: "Remote ASR",
    setRemoteAsrAddress: "Remote ASR endpoint",
    setSaving: "Saving…",
    setChecking: "Checking…",
    setTestConnection: "Test connection",
    setOnline: "Online",
    setOffline: "Offline",
    setModelLabel: "Model: {name}",
    setEnterEndpoint: "Please enter the remote ASR endpoint",
    setEndpointSaved: "Remote ASR endpoint saved",
    setSaveFailed: "Save failed: {error}",
    setRemoteNote: "Remote ASR is reserved for the paid version and is unavailable in the open-source MVP.",
    setAdvancedTitle: "Coming soon",
    setPlanned: "Planned",
    setAdvTtsName: "TTS voice synthesis",
    setAdvTtsDesc: "Read transcripts and summaries aloud",
    setAdvPttName: "Push-to-talk interpreting",
    setAdvPttDesc: "Hold a key to speak and hear live interpretation",
    setAdvSubtitleName: "Subtitle overlay window",
    setAdvSubtitleDesc: "Always-on-top live subtitles",
    setAdvSelectionName: "Selection translation",
    setAdvSelectionDesc: "Translate selected text anywhere",
    setAdvLiveSummaryName: "Real-time summary",
    setAdvLiveSummaryDesc: "Rolling summary while recording",
    setAdvSpeakerIdName: "Speaker recognition",
    setAdvSpeakerIdDesc: "Distinguish who said what",
    setAdvCloudAsrName: "Cloud high-accuracy ASR",
    setAdvCloudAsrDesc: "Higher accuracy via cloud models",
    setAdvHelpName: "In-app help",
    setAdvHelpDesc: "Built-in guides and FAQ",
  },
  zh: {
    setPageSubtitle: "模型、音频与导出偏好",
    setTabModels: "模型",
    setTabAudioExport: "音频与导出",
    setTabApi: "API",
    setTabAdvanced: "高级",
    setAsrModels: "模型",
    setGroupAsr: "ASR 模型",
    setGroupTranslate: "翻译模型",
    setGroupSummary: "总结模型",
    setModelDir: "模型目录",
    setModelDirHint: "模型文件存放位置",
    setNoModels: "暂无可下载的模型",
    setInstalled: "已安装",
    setDownloading: "下载中",
    setNotInstalled: "未安装",
    setDownloadingPending: "下载中…",
    setStageDownloading: "下载中 {percent}%",
    setStageExtracting: "解压中…",
    setStageVerifying: "校验中…",
    setDownloadDone: "模型下载完成",
    setDownloadFailed: "模型下载失败",
    setImport: "导入",
    setImportDone: "模型导入完成",
    setImportFailed: "模型导入失败：{error}",
    setReopenOnboarding: "重新打开新手指引",
    setDeleteConfirm: "确定删除模型「{name}」吗？删除后需重新下载才能使用。",
    setModelDeleted: "模型已删除",
    setDeleteFailed: "删除失败：{error}",
    setModelNote1: "应用不内置模型文件，首次使用请先下载。模型从 GitHub 官方 Release 下载，网络不佳时自动切换镜像。",
    setModelNote2: "翻译模型在本地运行，用于翻译页与实时转录中的翻译功能。",
    setModelDescSenseVoice: "多语言 ASR（中/英/日/韩/粤），VAD 伪流式",
    setModelDescXAsr: "中英双语纯流式 ASR，延迟更低",
    setModelDescOpusMt: "OPUS-MT 中英互译模型，113MB，速度快",
    setModelDescHymt2: "腾讯混元 Hy-MT2，1.1GB，翻译质量更高，支持 13 种目标语言",
    setModelDescQwen25: "本地会议总结 LLM（Qwen2.5-3B，2.1GB，较小较快）",
    setModelDescQwen3: "本地会议总结 LLM（Qwen3-4B-2507，质量更好）",
    setModelDescGemma: "本地会议总结 LLM（Gemma-3-4B，英文较强）",
    setModelNameSenseVoice: "SenseVoice 多语言模型",
    setModelNameXAsr: "X-ASR 流式模型（中英，480ms）",
    setModelNameOpusZhEn: "OPUS-MT 翻译模型（中 → 英）",
    setModelNameOpusEnZh: "OPUS-MT 翻译模型（英 → 中）",
    setModelNameHymt2: "Hy-MT2-1.8B（高质量翻译）",
    setModelNameQwen25: "Qwen2.5-3B-Instruct（会议总结）",
    setModelNameQwen3: "Qwen3-4B-Instruct-2507（会议总结）",
    setModelNameGemma: "Gemma-3-4B-it（会议总结）",
    setAudio: "音频",
    setMicrophone: "麦克风",
    setNoDevice: "未检测到",
    setSystemAudio: "系统音频",
    setOpenSoundSettings: "打开系统声音设置",
    setDeviceHint: "录音设备在「实时转录」页的录音控制面板中选择",
    setRecordingExport: "录音与导出",
    setRecordingsFolder: "录音保存目录",
    setChange: "更改…",
    setOpenFolder: "打开文件夹",
    setExportDir: "默认导出目录",
    setExportDirUnset: "未设置（默认使用录音所在文件夹）",
    setFolderUpdated: "录音保存目录已更新",
    setFolderFailed: "设置录音保存目录失败：{error}",
    setOpenFolderFailed: "打开文件夹失败：{error}",
    setExportDirUpdated: "默认导出目录已更新",
    setExportDirFailed: "设置导出目录失败：{error}",
    setRemoteAsrTitle: "远程 ASR",
    setRemoteAsrAddress: "远程 ASR 服务地址",
    setSaving: "保存中…",
    setChecking: "检测中…",
    setTestConnection: "测试连接",
    setOnline: "在线",
    setOffline: "离线",
    setModelLabel: "模型：{name}",
    setEnterEndpoint: "请输入远程 ASR 服务地址",
    setEndpointSaved: "远程 ASR 地址已保存",
    setSaveFailed: "保存失败：{error}",
    setRemoteNote: "远程 ASR 为付费版预留接口，开源 MVP 版本不可用。",
    setAdvancedTitle: "即将上线",
    setPlanned: "规划中",
    setAdvTtsName: "TTS 语音合成",
    setAdvTtsDesc: "朗读转写与总结内容",
    setAdvPttName: "按键实时传译",
    setAdvPttDesc: "按住说话，实时播报译文",
    setAdvSubtitleName: "字幕悬浮窗",
    setAdvSubtitleDesc: "置顶显示实时字幕",
    setAdvSelectionName: "划词翻译",
    setAdvSelectionDesc: "选中任意文本即时翻译",
    setAdvLiveSummaryName: "实时摘要",
    setAdvLiveSummaryDesc: "录音过程中滚动生成摘要",
    setAdvSpeakerIdName: "说话人识别",
    setAdvSpeakerIdDesc: "区分不同说话人",
    setAdvCloudAsrName: "云端高精度 ASR",
    setAdvCloudAsrDesc: "云端大模型，更高识别精度",
    setAdvHelpName: "应用内帮助",
    setAdvHelpDesc: "内置使用指南与常见问题",
  },
  ko: {
    setPageSubtitle: "모델, 오디오 및 내보내기 환경 설정",
    setTabModels: "모델",
    setTabAudioExport: "오디오 및 내보내기",
    setTabApi: "API",
    setTabAdvanced: "고급",
    setAsrModels: "모델",
    setGroupAsr: "ASR 모델",
    setGroupTranslate: "번역 모델",
    setGroupSummary: "요약 모델",
    setModelDir: "모델 디렉터리",
    setModelDirHint: "모델 파일이 저장되는 위치",
    setNoModels: "다운로드할 수 있는 모델이 없습니다",
    setInstalled: "설치됨",
    setDownloading: "다운로드 중",
    setNotInstalled: "설치되지 않음",
    setDownloadingPending: "다운로드 중…",
    setStageDownloading: "다운로드 중 {percent}%",
    setStageExtracting: "압축 해제 중…",
    setStageVerifying: "검증 중…",
    setDownloadDone: "모델 다운로드가 완료되었습니다",
    setDownloadFailed: "모델 다운로드에 실패했습니다",
    setImport: "가져오기",
    setImportDone: "모델을 가져왔습니다",
    setImportFailed: "모델 가져오기 실패: {error}",
    setReopenOnboarding: "시작 가이드 다시 열기",
    setDeleteConfirm: "모델 \"{name}\"을(를) 삭제하시겠습니까? 삭제 후 다시 사용하려면 재다운로드가 필요합니다.",
    setModelDeleted: "모델이 삭제되었습니다",
    setDeleteFailed: "삭제 실패: {error}",
    setModelNote1: "앱에 모델 파일이 포함되어 있지 않으므로 처음 사용 전에 다운로드하세요. 모델은 GitHub 공식 Release에서 다운로드되며, 네트워크 상태가 좋지 않으면 자동으로 미러로 전환됩니다.",
    setModelNote2: "번역 모델은 로컬에서 실행되며, 번역 페이지와 실시간 받아쓰기의 번역 기능에 사용됩니다.",
    setModelDescSenseVoice: "다국어 ASR(중/영/일/한/광둥어), VAD 유사 스트리밍",
    setModelDescXAsr: "중-영 이중언어 완전 스트리밍 ASR, 지연 시간이 더 짧습니다",
    setModelDescOpusMt: "OPUS-MT 중-영 번역 모델, 113MB, 빠른 속도",
    setModelDescHymt2: "Tencent Hunyuan Hy-MT2, 1.1GB, 더 높은 품질, 13개 대상 언어 지원",
    setModelDescQwen25: "로컬 회의 요약 LLM(Qwen2.5-3B, 2.1GB, 더 작고 빠름)",
    setModelDescQwen3: "로컬 회의 요약 LLM(Qwen3-4B-2507, 더 나은 품질)",
    setModelDescGemma: "로컬 회의 요약 LLM(Gemma-3-4B, 영어에 강함)",
    setModelNameSenseVoice: "SenseVoice 다국어 모델",
    setModelNameXAsr: "X-ASR 스트리밍 모델(중/영, 480ms)",
    setModelNameOpusZhEn: "OPUS-MT 번역 모델(중 → 영)",
    setModelNameOpusEnZh: "OPUS-MT 번역 모델(영 → 중)",
    setModelNameHymt2: "Hy-MT2-1.8B(고품질 번역)",
    setModelNameQwen25: "Qwen2.5-3B-Instruct(회의 요약)",
    setModelNameQwen3: "Qwen3-4B-Instruct-2507(회의 요약)",
    setModelNameGemma: "Gemma-3-4B-it(회의 요약)",
    setAudio: "오디오",
    setMicrophone: "마이크",
    setNoDevice: "감지되지 않음",
    setSystemAudio: "시스템 오디오",
    setOpenSoundSettings: "시스템 사운드 설정 열기",
    setDeviceHint: "녹음 기기는 \"받아쓰기\" 페이지의 녹음 컨트롤 패널에서 선택합니다",
    setRecordingExport: "녹음 및 내보내기",
    setRecordingsFolder: "녹음 저장 폴더",
    setChange: "변경…",
    setOpenFolder: "폴더 열기",
    setExportDir: "기본 내보내기 폴더",
    setExportDirUnset: "설정되지 않음(기본값: 녹음 폴더)",
    setFolderUpdated: "녹음 저장 폴더가 업데이트되었습니다",
    setFolderFailed: "녹음 저장 폴더 설정 실패: {error}",
    setOpenFolderFailed: "폴더 열기 실패: {error}",
    setExportDirUpdated: "기본 내보내기 폴더가 업데이트되었습니다",
    setExportDirFailed: "내보내기 폴더 설정 실패: {error}",
    setRemoteAsrTitle: "원격 ASR",
    setRemoteAsrAddress: "원격 ASR 서비스 주소",
    setSaving: "저장 중…",
    setChecking: "확인 중…",
    setTestConnection: "연결 테스트",
    setOnline: "온라인",
    setOffline: "오프라인",
    setModelLabel: "모델: {name}",
    setEnterEndpoint: "원격 ASR 서비스 주소를 입력하세요",
    setEndpointSaved: "원격 ASR 주소가 저장되었습니다",
    setSaveFailed: "저장 실패: {error}",
    setRemoteNote: "원격 ASR은 유료 버전을 위해 예약된 인터페이스로, 오픈소스 MVP 버전에서는 사용할 수 없습니다.",
    setAdvancedTitle: "출시 예정",
    setPlanned: "계획 중",
    setAdvTtsName: "TTS 음성 합성",
    setAdvTtsDesc: "받아쓰기와 요약을 소리 내어 읽기",
    setAdvPttName: "버튼 실시간 통역",
    setAdvPttDesc: "버튼을 누른 채 말하면 실시간으로 번역을 재생합니다",
    setAdvSubtitleName: "자막 플로팅 창",
    setAdvSubtitleDesc: "항상 위에 표시되는 실시간 자막",
    setAdvSelectionName: "드래그 번역",
    setAdvSelectionDesc: "선택한 텍스트를 어디서나 즉시 번역",
    setAdvLiveSummaryName: "실시간 요약",
    setAdvLiveSummaryDesc: "녹음 중 요약을 계속 생성합니다",
    setAdvSpeakerIdName: "화자 인식",
    setAdvSpeakerIdDesc: "누가 말했는지 구분합니다",
    setAdvCloudAsrName: "클라우드 고정밀 ASR",
    setAdvCloudAsrDesc: "클라우드 대형 모델로 더 높은 인식 정확도",
    setAdvHelpName: "앱 내 도움말",
    setAdvHelpDesc: "내장 사용 가이드와 FAQ",
  },
  ja: {
    setPageSubtitle: "モデル・オーディオ・エクスポートの設定",
    setTabModels: "モデル",
    setTabAudioExport: "オーディオとエクスポート",
    setTabApi: "API",
    setTabAdvanced: "詳細",
    setAsrModels: "モデル",
    setGroupAsr: "ASR モデル",
    setGroupTranslate: "翻訳モデル",
    setGroupSummary: "要約モデル",
    setModelDir: "モデルディレクトリ",
    setModelDirHint: "モデルファイルの保存場所",
    setNoModels: "ダウンロード可能なモデルはありません",
    setInstalled: "インストール済み",
    setDownloading: "ダウンロード中",
    setNotInstalled: "未インストール",
    setDownloadingPending: "ダウンロード中…",
    setStageDownloading: "ダウンロード中 {percent}%",
    setStageExtracting: "展開中…",
    setStageVerifying: "検証中…",
    setDownloadDone: "モデルのダウンロードが完了しました",
    setDownloadFailed: "モデルのダウンロードに失敗しました",
    setImport: "インポート",
    setImportDone: "モデルをインポートしました",
    setImportFailed: "モデルのインポートに失敗しました: {error}",
    setReopenOnboarding: "セットアップガイドを再表示",
    setDeleteConfirm: "モデル「{name}」を削除しますか？削除後は再度ダウンロードしないと使用できません。",
    setModelDeleted: "モデルを削除しました",
    setDeleteFailed: "削除に失敗しました: {error}",
    setModelNote1: "アプリにモデルファイルは同梱されていません。初回利用前にダウンロードしてください。モデルは GitHub 公式 Release からダウンロードされ、接続状況が悪い場合は自動でミラーに切り替わります。",
    setModelNote2: "翻訳モデルはローカルで動作し、翻訳ページとリアルタイム文字起こしの翻訳機能で使用されます。",
    setModelDescSenseVoice: "多言語 ASR（中/英/日/韓/広東語）、VAD 疑似ストリーミング",
    setModelDescXAsr: "中英バイリンガルの完全ストリーミング ASR、低遅延",
    setModelDescOpusMt: "OPUS-MT 中英翻訳モデル、113MB、高速",
    setModelDescHymt2: "Tencent Hunyuan Hy-MT2、1.1GB、より高品質、13 言語に対応",
    setModelDescQwen25: "ローカル会議要約 LLM（Qwen2.5-3B、2.1GB、小型で高速）",
    setModelDescQwen3: "ローカル会議要約 LLM（Qwen3-4B-2507、より高品質）",
    setModelDescGemma: "ローカル会議要約 LLM（Gemma-3-4B、英語に強い）",
    setModelNameSenseVoice: "SenseVoice 多言語モデル",
    setModelNameXAsr: "X-ASR ストリーミングモデル（中英、480ms）",
    setModelNameOpusZhEn: "OPUS-MT 翻訳モデル（中 → 英）",
    setModelNameOpusEnZh: "OPUS-MT 翻訳モデル（英 → 中）",
    setModelNameHymt2: "Hy-MT2-1.8B（高品質翻訳）",
    setModelNameQwen25: "Qwen2.5-3B-Instruct（会議要約）",
    setModelNameQwen3: "Qwen3-4B-Instruct-2507（会議要約）",
    setModelNameGemma: "Gemma-3-4B-it（会議要約）",
    setAudio: "オーディオ",
    setMicrophone: "マイク",
    setNoDevice: "検出されません",
    setSystemAudio: "システムオーディオ",
    setOpenSoundSettings: "システムのサウンド設定を開く",
    setDeviceHint: "録音デバイスは「文字起こし」ページの録音コントロールパネルで選択します",
    setRecordingExport: "録音とエクスポート",
    setRecordingsFolder: "録音の保存先",
    setChange: "変更…",
    setOpenFolder: "フォルダーを開く",
    setExportDir: "デフォルトのエクスポート先",
    setExportDirUnset: "未設定（デフォルトでは録音フォルダーを使用）",
    setFolderUpdated: "録音の保存先を更新しました",
    setFolderFailed: "録音の保存先の設定に失敗しました: {error}",
    setOpenFolderFailed: "フォルダーを開けませんでした: {error}",
    setExportDirUpdated: "デフォルトのエクスポート先を更新しました",
    setExportDirFailed: "エクスポート先の設定に失敗しました: {error}",
    setRemoteAsrTitle: "リモート ASR",
    setRemoteAsrAddress: "リモート ASR サービスアドレス",
    setSaving: "保存中…",
    setChecking: "確認中…",
    setTestConnection: "接続をテスト",
    setOnline: "オンライン",
    setOffline: "オフライン",
    setModelLabel: "モデル: {name}",
    setEnterEndpoint: "リモート ASR サービスアドレスを入力してください",
    setEndpointSaved: "リモート ASR アドレスを保存しました",
    setSaveFailed: "保存に失敗しました: {error}",
    setRemoteNote: "リモート ASR は有料版向けに予約されたインターフェースで、オープンソース MVP 版では利用できません。",
    setAdvancedTitle: "近日公開",
    setPlanned: "計画中",
    setAdvTtsName: "TTS 音声合成",
    setAdvTtsDesc: "文字起こしや要約を読み上げます",
    setAdvPttName: "プッシュトゥートーク通訳",
    setAdvPttDesc: "キーを押しながら話すとリアルタイムで訳文を再生",
    setAdvSubtitleName: "字幕フローティングウィンドウ",
    setAdvSubtitleDesc: "常に最前面に表示されるリアルタイム字幕",
    setAdvSelectionName: "選択テキスト翻訳",
    setAdvSelectionDesc: "選択したテキストをどこでも即座に翻訳",
    setAdvLiveSummaryName: "リアルタイム要約",
    setAdvLiveSummaryDesc: "録音中に要約を継続的に生成",
    setAdvSpeakerIdName: "話者認識",
    setAdvSpeakerIdDesc: "誰が話したかを区別します",
    setAdvCloudAsrName: "クラウド高精度 ASR",
    setAdvCloudAsrDesc: "クラウドの大規模モデルでより高い認識精度",
    setAdvHelpName: "アプリ内ヘルプ",
    setAdvHelpDesc: "使い方ガイドとよくある質問を内蔵",
  },
}
