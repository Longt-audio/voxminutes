import type { Language } from '../languages'

/** 核心文案：导航、slogan、窗口控制、主页空状态，以及跨页面通用按钮词。 */
export interface CoreMessages {
  navTranscribe: string
  navHistory: string
  navTranslate: string
  navTts: string
  navSettings: string
  navAccount: string
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
  msgTypeTip: string
  msgTypeAnnouncement: string
  msgTypeUpdate: string
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
  modelLoadingFirstDone: string
  modelUnloaded: string
  modelUnloadedSwap: string
  modelUnloadedIdle: string
}

export const CORE_MESSAGES: Record<Language, CoreMessages> = {
  en: {
    navTranscribe: 'Transcribe',
    navHistory: 'History',
    navTranslate: 'Translate',
    navTts: 'Speech',
    navSettings: 'Settings',
    navAccount: 'Account',
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
    msgTypeTip: 'Tip',
    msgTypeAnnouncement: 'Announcement',
    msgTypeUpdate: 'Update',
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
    modelLoadingFirstDone: 'First load of {model} took {seconds}s. When you no longer need the models, click the CPU button in the top bar to free memory.',
    modelUnloaded: 'Model {model} unloaded, memory freed',
    modelUnloadedSwap: 'Switched models: {model} unloaded to free memory',
    modelUnloadedIdle: '{model} was idle and has been unloaded automatically to free memory',
  },
  zh: {
    navTranscribe: '实时转录',
    navHistory: '历史记录',
    navTranslate: '翻译',
    navTts: '语音合成',
    navSettings: '设置',
    navAccount: '用户中心',
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
    msgTypeTip: '技巧',
    msgTypeAnnouncement: '公告',
    msgTypeUpdate: '更新',
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
    modelLoadingFirstDone: '首次加载 {model} 完成，耗时 {seconds} 秒。不需要模型时，可点击顶栏「清空模型后台」按钮释放内存。',
    modelUnloaded: '模型 {model} 已卸载，内存已释放',
    modelUnloadedSwap: '已切换模型，{model} 已卸载释放内存',
    modelUnloadedIdle: '{model} 闲置超时，已自动卸载释放内存',
  },
  ko: {
    navTranscribe: '받아쓰기',
    navHistory: '기록',
    navTranslate: '번역',
    navTts: '음성 합성',
    navSettings: '설정',
    navAccount: '계정',
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
    msgTypeTip: '팁',
    msgTypeAnnouncement: '공지',
    msgTypeUpdate: '업데이트',
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
    modelLoadingFirstDone: '{model} 첫 로딩 완료({seconds}초). 모델이 필요 없으면 상단 바의 「모델 백엔드 비우기」 버튼으로 메모리를 해제하세요.',
    modelUnloaded: '모델 {model}이(가) 언로드되어 메모리가 해제되었습니다',
    modelUnloadedSwap: '모델 전환: {model}이(가) 언로드되어 메모리가 해제되었습니다',
    modelUnloadedIdle: '{model}이(가) 유휴 시간 초과로 자동 언로드되어 메모리가 해제되었습니다',
  },
  ja: {
    navTranscribe: '文字起こし',
    navHistory: '履歴',
    navTranslate: '翻訳',
    navTts: '音声合成',
    navSettings: '設定',
    navAccount: 'アカウント',
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
    msgTypeTip: 'ヒント',
    msgTypeAnnouncement: 'お知らせ',
    msgTypeUpdate: '更新',
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
    modelLoadingFirstDone: '{model} の初回読み込みが完了しました（{seconds} 秒）。不要になったら上部バーの「モデルバックエンドをクリア」ボタンでメモリを解放できます。',
    modelUnloaded: 'モデル {model} をアンロードし、メモリを解放しました',
    modelUnloadedSwap: 'モデルを切り替えました：{model} をアンロードしてメモリを解放しました',
    modelUnloadedIdle: '{model} はアイドルタイムアウトにより自動でアンロードされ、メモリが解放されました',
  },
}
