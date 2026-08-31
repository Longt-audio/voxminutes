import type { Language } from '../languages'

/** 首次启动引导向导读案：欢迎页、ASR/翻译/总结模型选择、完成页。 */
export interface OnboardingMessages {
  onbWelcomeTitle: string
  onbWelcomeSubtitle: string
  onbWelcomePoint1: string
  onbWelcomePoint2: string
  onbWelcomePoint3: string
  onbStart: string
  onbUseRemote: string
  onbSetupLocal: string
  onbStepIndicator: string
  onbStepAsrTitle: string
  onbStepAsrDesc: string
  onbAsrRequiredHint: string
  onbStepTranslateTitle: string
  onbStepTranslateDesc: string
  onbOpusPairTitle: string
  onbStepSummaryTitle: string
  onbStepSummaryDesc: string
  onbStepDoneTitle: string
  onbDoneDesc: string
  onbNext: string
  onbBack: string
  onbSkip: string
  onbFinish: string
  onbRemoteTitle: string
  onbRemoteDesc: string
  onbRemoteUrl: string
  onbRemoteKey: string
  onbRemoteEnable: string
  onbRemoteTest: string
  onbRemoteTesting: string
  onbRemoteSkipStep: string
  onbRemoteSaved: string
  onbRemoteSaveFailed: string
  onbRemoteNeedUrl: string
  onbRemoteNeedKey: string
}

export const ONBOARDING_MESSAGES: Record<Language, OnboardingMessages> = {
  en: {
    onbWelcomeTitle: "Welcome to VoxMinutes",
    onbWelcomeSubtitle: "Your local meeting assistant",
    onbWelcomePoint1: "Records system audio and microphone at the same time",
    onbWelcomePoint2: "Real-time transcription, translation and meeting summaries",
    onbWelcomePoint3: "All data stays on your device — nothing leaves it",
    onbStart: "Get started",
    onbUseRemote: "Use remote models",
    onbSetupLocal: "Set up local models",
    onbStepIndicator: "Step {n} of {total}",
    onbStepAsrTitle: "Speech recognition model (required)",
    onbStepAsrDesc: "Pick one ASR model and download it, or import it from a local file.",
    onbAsrRequiredHint: "Install at least one speech recognition model to continue",
    onbStepTranslateTitle: "Translation model (optional)",
    onbStepTranslateDesc: "Used by the Translate page and translation during real-time transcription. You can install one later.",
    onbOpusPairTitle: "OPUS-MT Chinese–English (2 models)",
    onbStepSummaryTitle: "Summary model (optional)",
    onbStepSummaryDesc: "A local LLM that generates meeting minutes. You can install one later.",
    onbStepDoneTitle: "All set",
    onbDoneDesc: "You can download or import models anytime in Settings → Models.",
    onbNext: "Next",
    onbBack: "Back",
    onbSkip: "Skip",
    onbFinish: "Finish",
    onbRemoteTitle: "Remote service (optional)",
    onbRemoteDesc: "Use the gateway for cloud ASR / translation / summary. You can skip this and use local models instead.",
    onbRemoteUrl: "Server address",
    onbRemoteKey: "License (authorization code)",
    onbRemoteEnable: "Enable remote service",
    onbRemoteTest: "Test connection",
    onbRemoteTesting: "Testing…",
    onbRemoteSkipStep: "Skip remote service",
    onbRemoteSaved: "Saved",
    onbRemoteSaveFailed: "Save failed: {error}",
    onbRemoteNeedUrl: "Enter a server address first",
    onbRemoteNeedKey: "Enter a license first to test connection",
  },
  zh: {
    onbWelcomeTitle: "欢迎使用 VoxMinutes",
    onbWelcomeSubtitle: "你的本地会议助手",
    onbWelcomePoint1: "系统声音 + 麦克风双路同时录制",
    onbWelcomePoint2: "实时转写、翻译与会议总结",
    onbWelcomePoint3: "所有数据保存在本机，不出设备",
    onbStart: "开始设置",
    onbUseRemote: "使用远程模型",
    onbSetupLocal: "设置本地模型",
    onbStepIndicator: "第 {n} / {total} 步",
    onbStepAsrTitle: "语音识别模型（必装一个）",
    onbStepAsrDesc: "选择一个 ASR 模型下载，或从本地文件导入。",
    onbAsrRequiredHint: "至少安装一个语音识别模型后才能继续",
    onbStepTranslateTitle: "翻译模型（可选）",
    onbStepTranslateDesc: "用于翻译页与实时转录中的翻译功能，也可以稍后再装。",
    onbOpusPairTitle: "OPUS-MT 中英互译（2 个模型）",
    onbStepSummaryTitle: "总结模型（可选）",
    onbStepSummaryDesc: "本地大模型生成会议纪要，也可以稍后再装。",
    onbStepDoneTitle: "一切就绪",
    onbDoneDesc: "以后可随时在「设置 → 模型」页下载或导入模型。",
    onbNext: "下一步",
    onbBack: "上一步",
    onbSkip: "跳过",
    onbFinish: "完成",
    onbRemoteTitle: "远程服务（可选）",
    onbRemoteDesc: "通过网关使用云端 ASR / 翻译 / 总结。可以跳过，改用本地模型。",
    onbRemoteUrl: "服务器地址",
    onbRemoteKey: "授权码",
    onbRemoteEnable: "启用远程服务",
    onbRemoteTest: "测试连接",
    onbRemoteTesting: "检测中…",
    onbRemoteSkipStep: "跳过远程服务",
    onbRemoteSaved: "已保存",
    onbRemoteSaveFailed: "保存失败：{error}",
    onbRemoteNeedUrl: "请先填写服务器地址",
    onbRemoteNeedKey: "请先填写授权码再测试连接",
  },
  ko: {
    onbWelcomeTitle: "VoxMinutes에 오신 것을 환영합니다",
    onbWelcomeSubtitle: "로컬 회의 어시스턴트",
    onbWelcomePoint1: "시스템 사운드와 마이크를 동시에 녹음",
    onbWelcomePoint2: "실시간 받아쓰기, 번역 및 회의 요약",
    onbWelcomePoint3: "모든 데이터는 기기에만 저장되며 외부로 나가지 않습니다",
    onbStart: "시작하기",
    onbUseRemote: "원격 모델 사용",
    onbSetupLocal: "로컬 모델 설정",
    onbStepIndicator: "{total}단계 중 {n}단계",
    onbStepAsrTitle: "음성 인식 모델 (필수)",
    onbStepAsrDesc: "ASR 모델 하나를 선택해 다운로드하거나 로컬 파일에서 가져오세요.",
    onbAsrRequiredHint: "계속하려면 음성 인식 모델을 하나 이상 설치하세요",
    onbStepTranslateTitle: "번역 모델 (선택)",
    onbStepTranslateDesc: "번역 페이지와 실시간 받아쓰기 번역에 사용됩니다. 나중에 설치할 수 있습니다.",
    onbOpusPairTitle: "OPUS-MT 중-영 번역 (모델 2개)",
    onbStepSummaryTitle: "요약 모델 (선택)",
    onbStepSummaryDesc: "로컬 LLM으로 회의록을 생성합니다. 나중에 설치할 수 있습니다.",
    onbStepDoneTitle: "준비 완료",
    onbDoneDesc: "설정 → 모델 페이지에서 언제든지 모델을 다운로드하거나 가져올 수 있습니다.",
    onbNext: "다음",
    onbBack: "이전",
    onbSkip: "걄너뛰기",
    onbFinish: "완료",
    onbRemoteTitle: "원격 서비스 (선택)",
    onbRemoteDesc: "게이트웨이로 클라우드 ASR / 번역 / 요약을 사용합니다. 걸어넘고 로컬 모델을 쓸 수도 있습니다.",
    onbRemoteUrl: "서버 주소",
    onbRemoteKey: "라이선스(인증 코드)",
    onbRemoteEnable: "원격 서비스 사용",
    onbRemoteTest: "연결 테스트",
    onbRemoteTesting: "테스트 중…",
    onbRemoteSkipStep: "원격 서비스 걸어넘기",
    onbRemoteSaved: "저장됨",
    onbRemoteSaveFailed: "저장 실패: {error}",
    onbRemoteNeedUrl: "먼저 서버 주소를 입력하세요",
    onbRemoteNeedKey: "연결 테스트 전에 라이선스를 입력하세요",
  },
  ja: {
    onbWelcomeTitle: "VoxMinutes へようこそ",
    onbWelcomeSubtitle: "ローカルで動く会議アシスタント",
    onbWelcomePoint1: "システム音声とマイクを同時に録音",
    onbWelcomePoint2: "リアルタイム文字起こし・翻訳・会議要約",
    onbWelcomePoint3: "すべてのデータは端末内に保存され、外部に出ません",
    onbStart: "はじめる",
    onbUseRemote: "リモートモデルを使用",
    onbSetupLocal: "ローカルモデルを設定",
    onbStepIndicator: "ステップ {n} / {total}",
    onbStepAsrTitle: "音声認識モデル（必須）",
    onbStepAsrDesc: "ASR モデルを 1 つ選んでダウンロードするか、ローカルファイルからインポートしてください。",
    onbAsrRequiredHint: "続行するには音声認識モデルを 1 つ以上インストールしてください",
    onbStepTranslateTitle: "翻訳モデル（任意）",
    onbStepTranslateDesc: "翻訳ページとリアルタイム文字起こしの翻訳に使用します。後からインストールも可能です。",
    onbOpusPairTitle: "OPUS-MT 中英翻訳（2 モデル）",
    onbStepSummaryTitle: "要約モデル（任意）",
    onbStepSummaryDesc: "ローカル LLM が議事録を生成します。後からインストールも可能です。",
    onbStepDoneTitle: "準備完了",
    onbDoneDesc: "モデルは「設定 → モデル」ページからいつでもダウンロード／インポートできます。",
    onbNext: "次へ",
    onbBack: "戻る",
    onbSkip: "スキップ",
    onbFinish: "完了",
    onbRemoteTitle: "リモートサービス（任意）",
    onbRemoteDesc: "ゲートウェイ経由でクラウド ASR / 翻訳 / 要約を利用します。スキップしてローカルモデルでも使えます。",
    onbRemoteUrl: "サーバーアドレス",
    onbRemoteKey: "ライセンス(認証コード)",
    onbRemoteEnable: "リモートサービスを有効化",
    onbRemoteTest: "接続テスト",
    onbRemoteTesting: "テスト中…",
    onbRemoteSkipStep: "リモートサービスをスキップ",
    onbRemoteSaved: "保存しました",
    onbRemoteSaveFailed: "保存失敗: {error}",
    onbRemoteNeedUrl: "先にサーバーアドレスを入力してください",
    onbRemoteNeedKey: "接続テストの前にライセンスを入力してください",
  },
}
