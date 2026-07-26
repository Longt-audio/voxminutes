import type { Language } from '../languages'

export interface TranslateMessages {
  trTitle: string
  trSubtitle: string
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
  trEngineHymt2: string
}

export const TRANSLATE_MESSAGES: Record<Language, TranslateMessages> = {
  en: {
    trTitle: 'Translate',
    trSubtitle: 'Local translation models. Text never leaves your device.',
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
    trEngineOpus: 'OPUS-MT (fast)',
    trEngineHymt2: 'Hy-MT2 (high quality)',
  },
  zh: {
    trTitle: '翻译',
    trSubtitle: '本地模型翻译，文本不上传云端',
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
    trEngineOpus: 'OPUS-MT（快速）',
    trEngineHymt2: 'Hy-MT2（高质量）',
  },
  ko: {
    trTitle: '번역',
    trSubtitle: '로컬 번역 모델을 사용하며, 텍스트는 클라우드에 업로드되지 않습니다.',
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
    trEngineOpus: 'OPUS-MT (빠름)',
    trEngineHymt2: 'Hy-MT2 (고품질)',
  },
  ja: {
    trTitle: '翻訳',
    trSubtitle: 'ローカルの翻訳モデルを使用。テキストはクラウドにアップロードされません。',
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
    trEngineOpus: 'OPUS-MT（高速）',
    trEngineHymt2: 'Hy-MT2（高品質）',
  },
}
