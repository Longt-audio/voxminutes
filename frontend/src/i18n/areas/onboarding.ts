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
  onbRemoteSaved: string
  onbRemoteSaveFailed: string
  onbRemoteNeedUrl: string
  onbRemoteNeedKey: string
  /** 欢迎弹窗：折叠的手动粘贴授权码入口「我已有授权码」 */
  onbRemoteHasKey: string
  /** 欢迎弹窗：授权码输入框下方的保存提醒 */
  onbRemoteKeySaveHint: string
  onbStepLocalModelsTitle: string
  onbStepLocalModelsDesc: string
  /** 欢迎弹窗：「以后不再打开」勾选框 */
  welDontShowAgain: string
  /** 欢迎弹窗：公告区标题 */
  welNoticesTitle: string
  /** 欢迎弹窗：网关不可达时的默认欢迎词 */
  welDefaultTitle: string
  welDefaultBody: string
  /** 欢迎弹窗：未安装本地识别模型时的提示行 */
  welNoAsrHint: string
  /** 欢迎弹窗 P1：语言选择器下方的小字说明 */
  welChooseLanguage: string
  /** 首次启动的同意勾选框文案（后面跟《用户协议》和《隐私政策》两个链接） */
  welConsentCheck: string
  /** 同意提示里两个链接之间的连接词（"和" / "and"） */
  welConsentJoin: string
  /** 协议名（按钮文本），点击用系统浏览器打开官网页面 */
  welTermsLink: string
  welPrivacyLink: string
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
    onbStepAsrDesc: "Pick one speech recognition model and download it, or import it from a local file.",
    onbAsrRequiredHint: "Install at least one speech recognition model to continue",
    onbStepLocalModelsTitle: "Local models",
    onbStepLocalModelsDesc: "Pick the models you need (speech recognition / translate / summary). You can also skip and install them later.",
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
    onbRemoteDesc: "Use the gateway for cloud speech recognition / translation / summary. You can skip this and use local models instead.",
    onbRemoteUrl: "Server address",
    onbRemoteKey: "License (authorization code)",
    onbRemoteEnable: "Enable remote service",
    onbRemoteTest: "Test connection",
    onbRemoteTesting: "Testing…",
    onbRemoteSaved: "Saved",
    onbRemoteSaveFailed: "Save failed: {error}",
    onbRemoteNeedUrl: "Enter a server address first",
    onbRemoteNeedKey: "Enter a license first to test connection",
    onbRemoteHasKey: "I already have a license",
    onbRemoteKeySaveHint: "Save this license — you will need it to restore your credits on another computer.",
    welDontShowAgain: "Don't show again",
    welNoticesTitle: "Announcements",
    welDefaultTitle: "Welcome to VoxMinutes",
    welDefaultBody: "Basic features — recording, real-time transcription, translation and meeting summaries — run on local models: free forever and available offline.\n\nThe remote service comes with the official server built in — just turn on the switch to use high-precision cloud transcription / translation / summary (credits charged by actual usage). You can also skip it; all local features keep working for free.",
    welNoAsrHint: "No local speech recognition model installed yet — click \"Next\" to download one.",
    welChooseLanguage: "Choose your language",
    welConsentCheck: "I have read and agree to the",
    welConsentJoin: "and",
    welTermsLink: "Terms of Service",
    welPrivacyLink: "Privacy Policy",
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
    onbStepAsrDesc: "选择一个语音识别模型下载，或从本地文件导入。",
    onbAsrRequiredHint: "至少安装一个语音识别模型后才能继续",
    onbStepLocalModelsTitle: "本地模型",
    onbStepLocalModelsDesc: "选择你需要的模型（语音识别 / 翻译 / 总结），也可以跳过稍后再装。",
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
    onbRemoteDesc: "通过网关使用云端语音识别 / 翻译 / 总结。可以跳过，改用本地模型。",
    onbRemoteUrl: "服务器地址",
    onbRemoteKey: "授权码",
    onbRemoteEnable: "启用远程服务",
    onbRemoteTest: "测试连接",
    onbRemoteTesting: "检测中…",
    onbRemoteSaved: "已保存",
    onbRemoteSaveFailed: "保存失败：{error}",
    onbRemoteNeedUrl: "请先填写服务器地址",
    onbRemoteNeedKey: "请先填写授权码再测试连接",
    onbRemoteHasKey: "我已有授权码",
    onbRemoteKeySaveHint: "请复制保存好授权码，更换电脑时凭它恢复积分。",
    welDontShowAgain: "以后不再打开",
    welNoticesTitle: "公告",
    welDefaultTitle: "欢迎使用 VoxMinutes",
    welDefaultBody: "录音、实时转写、翻译、会议总结等基础功能使用本地模型，永久免费、离线可用。\n\n远程服务已内置官方服务器，打开开关即可使用云端高精度识别 / 翻译 / 总结（按量消耗积分）；也可以跳过，所有本地功能照常免费使用。",
    welNoAsrHint: "尚未安装本地识别模型，点击「下一步」去下载。",
    welChooseLanguage: "选择界面语言",
    welConsentCheck: "我已阅读并同意",
    welConsentJoin: "与",
    welTermsLink: "《用户协议》",
    welPrivacyLink: "《隐私政策》",
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
    onbStepAsrDesc: "음성 인식 모델 하나를 선택해 다운로드하거나 로컬 파일에서 가져오세요.",
    onbAsrRequiredHint: "계속하려면 음성 인식 모델을 하나 이상 설치하세요",
    onbStepLocalModelsTitle: "로컬 모델",
    onbStepLocalModelsDesc: "필요한 모델(음성 인식 / 번역 / 요약)을 선택하세요. 걸어넘고 나중에 설치해도 됩니다.",
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
    onbRemoteDesc: "게이트웨이로 클라우드 음성 인식 / 번역 / 요약을 사용합니다. 걸어넘고 로컬 모델을 쓸 수도 있습니다.",
    onbRemoteUrl: "서버 주소",
    onbRemoteKey: "라이선스(인증 코드)",
    onbRemoteEnable: "원격 서비스 사용",
    onbRemoteTest: "연결 테스트",
    onbRemoteTesting: "테스트 중…",
    onbRemoteSaved: "저장됨",
    onbRemoteSaveFailed: "저장 실패: {error}",
    onbRemoteNeedUrl: "먼저 서버 주소를 입력하세요",
    onbRemoteNeedKey: "연결 테스트 전에 라이선스를 입력하세요",
    onbRemoteHasKey: "라이선스가 이미 있습니다",
    onbRemoteKeySaveHint: "라이선스를 복사해 보관하세요. 다른 컴퓨터에서 크레딧을 복구할 때 필요합니다.",
    welDontShowAgain: "다시 표시하지 않음",
    welNoticesTitle: "공지",
    welDefaultTitle: "VoxMinutes에 오신 것을 환영합니다",
    welDefaultBody: "녹음, 실시간 받아쓰기, 번역, 회의 요약 등 기본 기능은 로컬 모델을 사용하며 영구 무료로, 오프라인에서도 사용할 수 있습니다.\n\n원격 서비스에는 공식 서버가 내장되어 있어 스위치만 켜면 클라우드 고정밀 인식 / 번역 / 요약을 사용할 수 있습니다(사용량만큼 크레딧 차감). 건너뛰어도 모든 로컬 기능은 계속 무료로 사용할 수 있습니다.",
    welNoAsrHint: "로컬 음성 인식 모델이 설치되지 않았습니다. \"다음\"을 눌러 다운로드하세요.",
    welChooseLanguage: "인터페이스 언어 선택",
    welConsentCheck: "다음을 읽었으며 이에 동의합니다:",
    welConsentJoin: "및",
    welTermsLink: "이용약관",
    welPrivacyLink: "개인정보 처리방침",
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
    onbStepAsrDesc: "音声認識モデルを 1 つ選んでダウンロードするか、ローカルファイルからインポートしてください。",
    onbAsrRequiredHint: "続行するには音声認識モデルを 1 つ以上インストールしてください",
    onbStepLocalModelsTitle: "ローカルモデル",
    onbStepLocalModelsDesc: "必要なモデル（音声認識 / 翻訳 / 要約）を選択してください。スキップして後でインストールしても構いません。",
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
    onbRemoteDesc: "ゲートウェイ経由でクラウド音声認識 / 翻訳 / 要約を利用します。スキップしてローカルモデルでも使えます。",
    onbRemoteUrl: "サーバーアドレス",
    onbRemoteKey: "ライセンス(認証コード)",
    onbRemoteEnable: "リモートサービスを有効化",
    onbRemoteTest: "接続テスト",
    onbRemoteTesting: "テスト中…",
    onbRemoteSaved: "保存しました",
    onbRemoteSaveFailed: "保存失敗: {error}",
    onbRemoteNeedUrl: "先にサーバーアドレスを入力してください",
    onbRemoteNeedKey: "接続テストの前にライセンスを入力してください",
    onbRemoteHasKey: "ライセンスをお持ちの方",
    onbRemoteKeySaveHint: "ライセンスをコピーして保存してください。別のパソコンでクレジットを復元する際に必要です。",
    welDontShowAgain: "今後表示しない",
    welNoticesTitle: "お知らせ",
    welDefaultTitle: "VoxMinutes へようこそ",
    welDefaultBody: "録音・リアルタイム文字起こし・翻訳・会議要約などの基本機能はローカルモデルで動作し、永久無料でオフラインでも使えます。\n\nリモートサービスには公式サーバーが組み込まれており、スイッチをオンにするだけでクラウドの高精度認識 / 翻訳 / 要約を利用できます（使用量に応じてクレジットを消費）。スキップしてもローカル機能はすべて無料で使えます。",
    welNoAsrHint: "ローカル音声認識モデルが未インストールです。「次へ」からダウンロードできます。",
    welChooseLanguage: "表示言語を選択",
    welConsentCheck: "以下を読み、同意します:",
    welConsentJoin: "および",
    welTermsLink: "利用規約",
    welPrivacyLink: "プライバシーポリシー",
  },
}
