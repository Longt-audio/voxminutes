import type { Language } from '../languages'

export interface TranslateMessages {
  trTitle: string
  trAutoDetect: string
  trAutoPair: string
  trLangZh: string
  trLangEn: string
  trLangJa: string
  trLangKo: string
  trLangFr: string
  trLangDe: string
  trLangEs: string
  trLangRu: string
  trLangPt: string
  trLangZhHant: string
  trLangYue: string
  trLangTh: string
  trLangVi: string
  trTargetLang: string
  trSwap: string
  trSwapTitle: string
  trTranslating: string
  trTranslate: string
  trShortcutHint: string
  trModelMissingTitle: string
  trModelMissingPre: string
  trModelMissingLink: string
  trModelMissingPost: string
  trCharCount: string
  trInputPlaceholder: string
  trOutputPlaceholder: string
  trTranslateFailed: string
  trCopyFailed: string
  trEngine: string
  trEngineOpus: string
  trRemoteModel: string
  trEngineHymt2: string
  recEngineCustomApi: string
  trPlaySource: string
  trPlayTarget: string
  trTtsLoading: string
  trTtsFailed: string
  trTtsSaved: string
  trTtsSaving: string
  trSetDefaultVoice: string
  trQuickSwitch: string
  trModelSettings: string
  trBackToTranslate: string
  trTranslateModelsTitle: string
  trCustomApiSection: string
  trCustomApiNotConfigured: string
  trTtsSection: string
  trDefaultVoice: string
  trVoiceProviderDefault: string
  trGotoTtsPage: string
  trOpusDesc: string
  trHymt2Desc: string
  trPickModel: string
  trMoreSettings: string
  /** 翻译页模型快速切换按钮上方的小标题 */
  trModelLabel: string
  /** 自定义 API 引导文案（配置入口已移到「设置 → 自定义 LLM」） */
  trCustomApiGuide: string
}

export const TRANSLATE_MESSAGES: Record<Language, TranslateMessages> = {
  en: {
    trTitle: 'Translate',
    trAutoDetect: 'Auto-detect',
    trAutoPair: 'Auto both ways',
    trLangZh: 'Chinese',
    trLangEn: 'English',
    trLangJa: 'Japanese',
    trLangKo: 'Korean',
    trLangFr: 'French',
    trLangDe: 'German',
    trLangEs: 'Spanish',
    trLangRu: 'Russian',
    trLangPt: 'Portuguese',
    trLangZhHant: 'Traditional Chinese',
    trLangYue: 'Cantonese',
    trLangTh: 'Thai',
    trLangVi: 'Vietnamese',
    trTargetLang: 'Target language',
    trSwap: 'Swap',
    trSwapTitle: 'Swap input and translation',
    trTranslating: 'Translating…',
    trTranslate: 'Translate',
    trShortcutHint: 'Ctrl + Enter to translate',
    trModelMissingTitle: 'Translation model not installed',
    trModelMissingPre: 'Please go to the ',
    trModelMissingLink: 'Settings page',
    trModelMissingPost: ' to download a translation model.',
    trCharCount: '{count} chars',
    trInputPlaceholder: 'Enter Chinese or English — auto-detected and translated…',
    trOutputPlaceholder: 'Translation will appear here',
    trTranslateFailed: 'Translation failed',
    trCopyFailed: 'Copy failed',
    trEngine: 'Engine',
    trRemoteModel: 'Remote model',
    trEngineOpus: 'OPUS-MT (fast)',
    trEngineHymt2: 'Hy-MT2 (high quality)',
    recEngineCustomApi: 'Custom API',
    trPlaySource: 'Play source',
    trPlayTarget: 'Play translation',
    trTtsLoading: 'Synthesizing…',
    trTtsFailed: 'Speech synthesis failed',
    trTtsSaved: 'Audio saved to {path}',
    trTtsSaving: 'Saving audio…',
    trSetDefaultVoice: 'Set default voice',
    trQuickSwitch: 'Model',
    trModelSettings: 'Models & Settings',
    trBackToTranslate: 'Back to Translate',
    trTranslateModelsTitle: 'Translation Models',
    trCustomApiSection: 'Custom API',
    trCustomApiNotConfigured: 'Not configured · Click to set up (use your own or self-hosted OpenAI-compatible / Anthropic API)',
    trTtsSection: 'Text to Speech',
    trDefaultVoice: 'Default voice',
    trVoiceProviderDefault: 'Provider default',
    trGotoTtsPage: 'Set voices on the TTS page',
    trOpusDesc: 'Fast, lightweight Chinese ⇄ English translation',
    trHymt2Desc: 'High-quality multilingual translation (zh/en/ja/ko and more)',
    trPickModel: 'Choose translation model',
    trMoreSettings: 'More settings',
    trModelLabel: 'Translation model',
    trCustomApiGuide: 'Custom APIs are configured in Settings → Custom LLM',
  },
  zh: {
    trTitle: '翻译',
    trAutoDetect: '自动检测',
    trAutoPair: '自动互译',
    trLangZh: '中文',
    trLangEn: 'English',
    trLangJa: '日语',
    trLangKo: '韩语',
    trLangFr: '法语',
    trLangDe: '德语',
    trLangEs: '西班牙语',
    trLangRu: '俄语',
    trLangPt: '葡萄牙语',
    trLangZhHant: '繁体中文',
    trLangYue: '粤语',
    trLangTh: '泰语',
    trLangVi: '越南语',
    trTargetLang: '目标语言',
    trSwap: '交换',
    trSwapTitle: '交换输入与译文',
    trTranslating: '翻译中…',
    trTranslate: '翻译',
    trShortcutHint: 'Ctrl + Enter 快速翻译',
    trModelMissingTitle: '翻译模型未安装',
    trModelMissingPre: '请先到',
    trModelMissingLink: '设置页',
    trModelMissingPost: '下载翻译模型。',
    trCharCount: '{count} 字',
    trInputPlaceholder: '输入中文或英文，自动识别并互译…',
    trOutputPlaceholder: '翻译结果将显示在这里',
    trTranslateFailed: '翻译失败',
    trCopyFailed: '复制失败',
    trEngine: '翻译引擎',
    trRemoteModel: '远程模型',
    trEngineOpus: 'OPUS-MT（快速）',
    trEngineHymt2: 'Hy-MT2（高质量）',
    recEngineCustomApi: '自定义 API',
    trPlaySource: '播放原文',
    trPlayTarget: '播放译文',
    trTtsLoading: '语音合成中…',
    trTtsFailed: '语音合成失败',
    trTtsSaved: '音频已保存到 {path}',
    trTtsSaving: '正在保存音频…',
    trSetDefaultVoice: '设置默认语音',
    trQuickSwitch: '模型',
    trModelSettings: '模型与设置',
    trBackToTranslate: '返回翻译',
    trTranslateModelsTitle: '翻译模型',
    trCustomApiSection: '自定义 API',
    trCustomApiNotConfigured: '未配置 · 点击前往配置（支持自有或本地部署的 OpenAI 兼容 / Anthropic API）',
    trTtsSection: '语音合成',
    trDefaultVoice: '默认音色',
    trVoiceProviderDefault: '供应商默认',
    trGotoTtsPage: '去语音合成页设置音色',
    trOpusDesc: '轻量快速的中英互译模型',
    trHymt2Desc: '高质量多语言翻译模型（中/英/日/韩等）',
    trPickModel: '选择翻译模型',
    trMoreSettings: '更多设置',
    trModelLabel: '翻译模型',
    trCustomApiGuide: '自定义 API 在「设置 → 自定义 LLM」中配置',
  },
  ko: {
    trTitle: '번역',
    trAutoDetect: '자동 감지',
    trAutoPair: '자동 양방향',
    trLangZh: '중국어',
    trLangEn: '영어',
    trLangJa: '일본어',
    trLangKo: '한국어',
    trLangFr: '프랑스어',
    trLangDe: '독일어',
    trLangEs: '스페인어',
    trLangRu: '러시아어',
    trLangPt: '포르투갈어',
    trLangZhHant: '번체 중국어',
    trLangYue: '광둥어',
    trLangTh: '태국어',
    trLangVi: '베트남어',
    trTargetLang: '대상 언어',
    trSwap: '바꾸기',
    trSwapTitle: '입력과 번역 결과 바꾸기',
    trTranslating: '번역 중…',
    trTranslate: '번역',
    trShortcutHint: 'Ctrl + Enter로 빠르게 번역',
    trModelMissingTitle: '번역 모델이 설치되지 않았습니다',
    trModelMissingPre: '',
    trModelMissingLink: '설정 페이지',
    trModelMissingPost: '에서 번역 모델을 다운로드하세요.',
    trCharCount: '{count}자',
    trInputPlaceholder: '중국어 또는 영어를 입력하면 자동으로 감지하여 번역합니다…',
    trOutputPlaceholder: '번역 결과가 여기에 표시됩니다',
    trTranslateFailed: '번역에 실패했습니다',
    trCopyFailed: '복사에 실패했습니다',
    trEngine: '번역 엔진',
    trRemoteModel: '원격 모델',
    trEngineOpus: 'OPUS-MT (빠름)',
    trEngineHymt2: 'Hy-MT2 (고품질)',
    recEngineCustomApi: '사용자 지정 API',
    trPlaySource: '원문 재생',
    trPlayTarget: '번역 재생',
    trTtsLoading: '음성 합성 중…',
    trTtsFailed: '음성 합성에 실패했습니다',
    trTtsSaved: '오디오가 {path}에 저장되었습니다',
    trTtsSaving: '오디오 저장 중…',
    trSetDefaultVoice: '기본 음성 설정',
    trQuickSwitch: '모델',
    trModelSettings: '모델 및 설정',
    trBackToTranslate: '번역으로 돌아가기',
    trTranslateModelsTitle: '번역 모델',
    trCustomApiSection: '사용자 지정 API',
    trCustomApiNotConfigured: '설정되지 않음 · 클릭하여 설정 (자체 또는 로컬 배포 OpenAI 호환 / Anthropic API 지원)',
    trTtsSection: '음성 합성',
    trDefaultVoice: '기본 음색',
    trVoiceProviderDefault: '공급자 기본값',
    trGotoTtsPage: '음성 합성 페이지에서 음색 설정',
    trOpusDesc: '빠르고 가벼운 중국어 ⇄ 영어 번역 모델',
    trHymt2Desc: '고품질 다국어 번역 모델(중/영/일/한 등)',
    trPickModel: '번역 모델 선택',
    trMoreSettings: '더 많은 설정',
    trModelLabel: '번역 모델',
    trCustomApiGuide: '사용자 지정 API는 「설정 → 사용자 정의 LLM」에서 설정합니다',
  },
  ja: {
    trTitle: '翻訳',
    trAutoDetect: '自動検出',
    trAutoPair: '自動相互翻訳',
    trLangZh: '中国語',
    trLangEn: '英語',
    trLangJa: '日本語',
    trLangKo: '韓国語',
    trLangFr: 'フランス語',
    trLangDe: 'ドイツ語',
    trLangEs: 'スペイン語',
    trLangRu: 'ロシア語',
    trLangPt: 'ポルトガル語',
    trLangZhHant: '繁体字中国語',
    trLangYue: '広東語',
    trLangTh: 'タイ語',
    trLangVi: 'ベトナム語',
    trTargetLang: '対象言語',
    trSwap: '入れ替え',
    trSwapTitle: '入力と訳文を入れ替える',
    trTranslating: '翻訳中…',
    trTranslate: '翻訳',
    trShortcutHint: 'Ctrl + Enter で翻訳',
    trModelMissingTitle: '翻訳モデルがインストールされていません',
    trModelMissingPre: '',
    trModelMissingLink: '設定ページ',
    trModelMissingPost: 'で翻訳モデルをダウンロードしてください。',
    trCharCount: '{count}文字',
    trInputPlaceholder: '中国語または英語を入力すると、自動で検出して翻訳します…',
    trOutputPlaceholder: '翻訳結果がここに表示されます',
    trTranslateFailed: '翻訳に失敗しました',
    trCopyFailed: 'コピーに失敗しました',
    trEngine: '翻訳エンジン',
    trRemoteModel: 'リモートモデル',
    trEngineOpus: 'OPUS-MT（高速）',
    trEngineHymt2: 'Hy-MT2（高品質）',
    recEngineCustomApi: 'カスタム API',
    trPlaySource: '原文を再生',
    trPlayTarget: '訳文を再生',
    trTtsLoading: '音声合成中…',
    trTtsFailed: '音声合成に失敗しました',
    trTtsSaved: '音声を {path} に保存しました',
    trTtsSaving: '音声を保存中…',
    trSetDefaultVoice: 'デフォルト音声を設定',
    trQuickSwitch: 'モデル',
    trModelSettings: 'モデルと設定',
    trBackToTranslate: '翻訳に戻る',
    trTranslateModelsTitle: '翻訳モデル',
    trCustomApiSection: 'カスタム API',
    trCustomApiNotConfigured: '未設定 · クリックして設定（自前またはローカルデプロイの OpenAI 互換 / Anthropic API に対応）',
    trTtsSection: '音声合成',
    trDefaultVoice: 'デフォルト音声',
    trVoiceProviderDefault: 'プロバイダーのデフォルト',
    trGotoTtsPage: '音声合成ページで音声を設定',
    trOpusDesc: '軽量で高速な中国語 ⇄ 英語翻訳モデル',
    trHymt2Desc: '高品質な多言語翻訳モデル（中/英/日/韓など）',
    trPickModel: '翻訳モデルを選択',
    trMoreSettings: 'その他の設定',
    trModelLabel: '翻訳モデル',
    trCustomApiGuide: 'カスタム API は「設定 → カスタム LLM」で設定します',
  },
}
