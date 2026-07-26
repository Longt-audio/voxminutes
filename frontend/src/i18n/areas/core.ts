import type { Language } from '../languages'

/** 核心文案：导航、slogan、窗口控制、主页空状态，以及跨页面通用按钮词。 */
export interface CoreMessages {
  navTranscribe: string
  navHistory: string
  navTranslate: string
  navSettings: string
  sloganFooter: string
  emptyTitle: string
  emptyHint: string
  winMinimize: string
  winMaximize: string
  winRestore: string
  winClose: string
  languageLabel: string
  comCancel: string
  comConfirm: string
  comSave: string
  comDelete: string
  comClose: string
  comBack: string
  comLoading: string
  comRetry: string
  comCopy: string
  comCopied: string
  comDownload: string
  comImport: string
  comExport: string
  comSearch: string
  comRefresh: string
  comEdit: string
  comEnabled: string
  comDisabled: string
  comAll: string
  comError: string
  modelLoadingStart: string
  modelLoadingDone: string
  modelLoadingError: string
}

export const CORE_MESSAGES: Record<Language, CoreMessages> = {
  en: {
    navTranscribe: 'Transcribe',
    navHistory: 'History',
    navTranslate: 'Translate',
    navSettings: 'Settings',
    sloganFooter: 'Your local meeting assistant · Records system audio & mic together · Real-time transcription, translation & summaries — all on your device.',
    emptyTitle: 'No transcripts yet',
    emptyHint: 'Start recording to see live speech recognition',
    winMinimize: 'Minimize',
    winMaximize: 'Maximize',
    winRestore: 'Restore',
    winClose: 'Close',
    languageLabel: 'Language',
    comCancel: 'Cancel',
    comConfirm: 'Confirm',
    comSave: 'Save',
    comDelete: 'Delete',
    comClose: 'Close',
    comBack: 'Back',
    comLoading: 'Loading…',
    comRetry: 'Retry',
    comCopy: 'Copy',
    comCopied: 'Copied',
    comDownload: 'Download',
    comImport: 'Import',
    comExport: 'Export',
    comSearch: 'Search',
    comRefresh: 'Refresh',
    comEdit: 'Edit',
    comEnabled: 'Enabled',
    comDisabled: 'Disabled',
    comAll: 'All',
    comError: 'Error',
    modelLoadingStart: 'Loading model {model}…',
    modelLoadingDone: 'Model {model} loaded ({seconds}s)',
    modelLoadingError: 'Model {model} failed to load: {message}',
  },
  zh: {
    navTranscribe: '实时转录',
    navHistory: '历史记录',
    navTranslate: '翻译',
    navSettings: '设置',
    sloganFooter: '您的本地会议助手 · 系统声音与麦克风同步录制 · 实时转写、翻译与总结，数据不出设备',
    emptyTitle: '暂无转录内容',
    emptyHint: '开始录音以查看实时语音识别',
    winMinimize: '最小化',
    winMaximize: '最大化',
    winRestore: '还原',
    winClose: '关闭',
    languageLabel: '语言',
    comCancel: '取消',
    comConfirm: '确认',
    comSave: '保存',
    comDelete: '删除',
    comClose: '关闭',
    comBack: '返回',
    comLoading: '加载中…',
    comRetry: '重试',
    comCopy: '复制',
    comCopied: '已复制',
    comDownload: '下载',
    comImport: '导入',
    comExport: '导出',
    comSearch: '搜索',
    comRefresh: '刷新',
    comEdit: '编辑',
    comEnabled: '已启用',
    comDisabled: '已禁用',
    comAll: '全部',
    comError: '错误',
    modelLoadingStart: '正在加载模型 {model}…',
    modelLoadingDone: '模型 {model} 加载完成（{seconds} 秒）',
    modelLoadingError: '模型 {model} 加载失败：{message}',
  },
  ko: {
    navTranscribe: '받아쓰기',
    navHistory: '기록',
    navTranslate: '번역',
    navSettings: '설정',
    sloganFooter: '로컬 회의 어시스턴트 · 시스템 오디오와 마이크 동시 녹음 · 실시간 받아쓰기, 번역, 요약 — 데이터는 기기 밖으로 나가지 않습니다.',
    emptyTitle: '아직 받아쓰기 내용이 없습니다',
    emptyHint: '녹음을 시작하면 실시간 음성 인식이 표시됩니다',
    winMinimize: '최소화',
    winMaximize: '최대화',
    winRestore: '복원',
    winClose: '닫기',
    languageLabel: '언어',
    comCancel: '취소',
    comConfirm: '확인',
    comSave: '저장',
    comDelete: '삭제',
    comClose: '닫기',
    comBack: '뒤로',
    comLoading: '로딩 중…',
    comRetry: '다시 시도',
    comCopy: '복사',
    comCopied: '복사됨',
    comDownload: '다운로드',
    comImport: '가져오기',
    comExport: '내보내기',
    comSearch: '검색',
    comRefresh: '새로고침',
    comEdit: '편집',
    comEnabled: '활성화됨',
    comDisabled: '비활성화됨',
    comAll: '전체',
    comError: '오류',
    modelLoadingStart: '모델 {model} 로딩 중…',
    modelLoadingDone: '모델 {model} 로딩 완료({seconds}초)',
    modelLoadingError: '모델 {model} 로딩 실패: {message}',
  },
  ja: {
    navTranscribe: '文字起こし',
    navHistory: '履歴',
    navTranslate: '翻訳',
    navSettings: '設定',
    sloganFooter: 'ローカル会議アシスタント · システム音声とマイクを同時録音 · リアルタイム文字起こし・翻訳・要約。データはデバイスの外に出ません。',
    emptyTitle: '文字起こし結果はまだありません',
    emptyHint: '録音を開始すると、リアルタイムの音声認識が表示されます',
    winMinimize: '最小化',
    winMaximize: '最大化',
    winRestore: '元に戻す',
    winClose: '閉じる',
    languageLabel: '言語',
    comCancel: 'キャンセル',
    comConfirm: '確認',
    comSave: '保存',
    comDelete: '削除',
    comClose: '閉じる',
    comBack: '戻る',
    comLoading: '読み込み中…',
    comRetry: '再試行',
    comCopy: 'コピー',
    comCopied: 'コピーしました',
    comDownload: 'ダウンロード',
    comImport: 'インポート',
    comExport: 'エクスポート',
    comSearch: '検索',
    comRefresh: '更新',
    comEdit: '編集',
    comEnabled: '有効',
    comDisabled: '無効',
    comAll: 'すべて',
    comError: 'エラー',
    modelLoadingStart: 'モデル {model} を読み込み中…',
    modelLoadingDone: 'モデル {model} の読み込みが完了しました（{seconds} 秒）',
    modelLoadingError: 'モデル {model} の読み込みに失敗しました：{message}',
  },
}
