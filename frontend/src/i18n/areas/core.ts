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
  floatingBall: string
  floatingBallShow: string
  floatingBallHide: string
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
  /** 页面级错误边界（app/error.tsx / app/global-error.tsx）文案 */
  errPageTitle: string
  errPageHint: string
  errReload: string
  /** 领取积分成功卡片：余额标签（数字单独大号渲染） */
  claimBalanceLabel: string
  /** 领取积分成功卡片：每台设备限领一次 + 赠送积分 90 天有效 */
  claimOnceNote: string
  /** 顶部横幅通用：弱化文字按钮「不再提示」 */
  comNeverRemind: string
  /** 版本更新横幅：彻底关闭提醒的确认对话框 */
  updNeverTitle: string
  updNeverDesc: string
  /** 积分不足横幅：去充值链接 */
  lowCreditTopUp: string
  /** 积分不足横幅：彻底关闭提醒的确认对话框（余额回升自动恢复） */
  lowCreditNeverTitle: string
  lowCreditNeverDesc: string
  /** 分页通用：上一页 / 下一页 */
  comPrev: string
  comNext: string
  /** 空值占位：未配置 */
  comNotSet: string
  /** 版本更新横幅：新版本提示与当前版本注记 */
  updNewVersion: string
  updCurrentVersion: string
  /** 重要信息推送横幅：内容前缀标签 */
  noticeLabel: string
  /** 用户中心页补充 key（account.ts 归另一 agent 所有，暂放 core 避免并行冲突） */
  accLicenseTitle: string
  accLicenseCopied: string
  accLicenseBackupHint: string
  accUsageTitle: string
  accUsageTabTasks: string
  accUsageTabDate: string
  accUsageTabModel: string
  accUsageEmpty: string
  accCreditsUnit: string
  accConsumeLabel: string
  accIncomeLabel: string
  accCallsFmt: string
  accUnitSeconds: string
  accUnitChars: string
  accTotalFmt: string
  accModelsTooltip: string
  /** 用户中心：账本流水类型标签（赠送/兑换/消费/调整/退款/过期/其他） */
  accLedgerGift: string
  accLedgerRedeem: string
  accLedgerConsume: string
  accLedgerAdjust: string
  accLedgerRefund: string
  accLedgerExpire: string
  accLedgerOther: string
  /** 远程模型价格单位（lib/remoteModelChoice.ts formatModelPrice；{price} 为数值占位） */
  priceFree: string
  pricePerMinute: string
  pricePerKTokens: string
  pricePerKChars: string
  /** 用户中心任务消耗：网关中文任务名 → 界面语言（account 页，旧名已归一化为功能名） */
  taskNameLive: string
  taskNameOffline: string
  taskNameSummary: string
  taskNameTts: string
  taskNameTranslate: string
  taskNameOther: string
  /** 任务消耗环节名（breakdown.kind = asr 时；translate/summary/tts 环节复用 taskName*） */
  stepKindAsr: string
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
    floatingBall: 'Ball',
    floatingBallShow: 'Show floating ball',
    floatingBallHide: 'Hide floating ball',
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
    errPageTitle: 'This page ran into a problem',
    errPageHint: 'Your recordings and history are safe. Go back to another page or reload the app.',
    errReload: 'Reload app',
    claimBalanceLabel: 'Current credit balance',
    claimOnceNote: 'Each computer can claim only once. Bonus credits are valid for 90 days.',
    comNeverRemind: "Don't show again",
    updNeverTitle: 'Stop update reminders?',
    updNeverDesc: 'Once confirmed, the app will no longer notify you about any new version.',
    lowCreditTopUp: 'Top up',
    lowCreditNeverTitle: 'Stop low-balance reminders?',
    lowCreditNeverDesc: 'Once confirmed, the low-balance banner will stay hidden. It re-enables automatically once your balance rises above the warning threshold.',
    comPrev: 'Prev',
    comNext: 'Next',
    comNotSet: 'Not set',
    updNewVersion: 'New version v{version} available',
    updCurrentVersion: ' (current v{version})',
    noticeLabel: 'Important',
    accLicenseTitle: 'My license',
    accLicenseCopied: 'License copied',
    accLicenseBackupHint: 'Your license is your account — top-ups and recovery both depend on it. Keep a backup.',
    accUsageTitle: 'Credit usage',
    accUsageTabTasks: 'By task',
    accUsageTabDate: 'By date',
    accUsageTabModel: 'By model',
    accUsageEmpty: 'No usage records yet',
    accCreditsUnit: 'credits',
    accConsumeLabel: 'Used',
    accIncomeLabel: 'Received',
    accCallsFmt: '{n} calls',
    accUnitSeconds: 's',
    accUnitChars: 'chars',
    accTotalFmt: '{n} {unit} total',
    accModelsTooltip: 'Models: {list}',
    accLedgerGift: 'Gift',
    accLedgerRedeem: 'Redeem',
    accLedgerConsume: 'Spend',
    accLedgerAdjust: 'Adjust',
    accLedgerRefund: 'Refund',
    accLedgerExpire: 'Expire',
    accLedgerOther: 'Other',
    priceFree: 'Free',
    pricePerMinute: '{price} credits/min',
    pricePerKTokens: '{price} credits/1K tokens',
    pricePerKChars: '{price} credits/1K chars',
    taskNameLive: 'Live Transcription',
    taskNameOffline: 'Offline Recognition',
    taskNameSummary: 'Meeting Summary',
    taskNameTts: 'Speech Synthesis',
    taskNameTranslate: 'Translation',
    taskNameOther: 'Other',
    stepKindAsr: 'Speech Recognition',
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
    floatingBall: '悬浮球',
    floatingBallShow: '显示悬浮球',
    floatingBallHide: '隐藏悬浮球',
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
    errPageTitle: '这个页面出错了',
    errPageHint: '录音与历史记录不受影响。可以返回其它页面，或重新加载应用。',
    errReload: '重新加载',
    claimBalanceLabel: '当前积分余额',
    claimOnceNote: '每个电脑设备仅可领取一次，赠送积分 90 天内有效。',
    comNeverRemind: '不再提示',
    updNeverTitle: '不再提醒新版本？',
    updNeverDesc: '确认后，本软件将不再提醒任何新版本更新。',
    lowCreditTopUp: '去充值',
    lowCreditNeverTitle: '不再提醒积分不足？',
    lowCreditNeverDesc: '确认后将不再显示积分不足提醒；余额回升到预警值以上时会自动恢复提醒。',
    comPrev: '上一页',
    comNext: '下一页',
    comNotSet: '未配置',
    updNewVersion: '新版本 v{version} 可用',
    updCurrentVersion: '（当前 v{version}）',
    noticeLabel: '重要通知',
    accLicenseTitle: '我的授权码',
    accLicenseCopied: '已复制授权码',
    accLicenseBackupHint: '授权码是你的账号，充值/找回都靠它，建议自行备份。',
    accUsageTitle: '积分消耗',
    accUsageTabTasks: '任务明细',
    accUsageTabDate: '按日期',
    accUsageTabModel: '按模型',
    accUsageEmpty: '暂无消耗记录',
    accCreditsUnit: '积分',
    accConsumeLabel: '消耗',
    accIncomeLabel: '入账',
    accCallsFmt: '{n} 次',
    accUnitSeconds: '秒',
    accUnitChars: '字符',
    accTotalFmt: '共 {n} {unit}',
    accModelsTooltip: '模型：{list}',
    accLedgerGift: '赠送',
    accLedgerRedeem: '兑换',
    accLedgerConsume: '消费',
    accLedgerAdjust: '调整',
    accLedgerRefund: '退款',
    accLedgerExpire: '过期',
    accLedgerOther: '其他',
    priceFree: '免费',
    pricePerMinute: '{price} 积分/分钟',
    pricePerKTokens: '{price} 积分/千token',
    pricePerKChars: '{price} 积分/千字符',
    taskNameLive: '实时转录',
    taskNameOffline: '离线识别',
    taskNameSummary: '会议总结',
    taskNameTts: '语音合成',
    taskNameTranslate: '翻译',
    taskNameOther: '其他调用',
    stepKindAsr: '语音识别',
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
    floatingBall: '플로팅 볼',
    floatingBallShow: '플로팅 볼 표시',
    floatingBallHide: '플로팅 볼 숨기기',
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
    errPageTitle: '이 페이지에 문제가 발생했습니다',
    errPageHint: '녹음과 기록은 그대로 유지됩니다. 다른 페이지로 돌아가거나 앱을 다시 불러오세요.',
    errReload: '앱 다시 불러오기',
    claimBalanceLabel: '현재 크레딧 잔액',
    claimOnceNote: '컴퓨터 1대당 한 번만 받을 수 있으며, 증정 크레딧은 90일간 유효합니다.',
    comNeverRemind: '다시 표시하지 않음',
    updNeverTitle: '업데이트 알림을 끄시겠습니까?',
    updNeverDesc: '확인하면 앞으로 새 버전 업데이트 알림이 표시되지 않습니다.',
    lowCreditTopUp: '충전하기',
    lowCreditNeverTitle: '잔액 부족 알림을 끄시겠습니까?',
    lowCreditNeverDesc: '확인하면 잔액 부족 알림이 표시되지 않습니다. 잔액이 경고 기준 이상으로 회복되면 알림이 자동으로 다시 활성화됩니다.',
    comPrev: '이전',
    comNext: '다음',
    comNotSet: '설정되지 않음',
    updNewVersion: '새 버전 v{version} 사용 가능',
    updCurrentVersion: ' (현재 v{version})',
    noticeLabel: '중요 알림',
    accLicenseTitle: '내 라이선스',
    accLicenseCopied: '라이선스가 복사되었습니다',
    accLicenseBackupHint: '라이선스는 계정 그 자체입니다. 충전과 복구에 모두 필요하니 백업해 두세요.',
    accUsageTitle: '크레딧 사용',
    accUsageTabTasks: '작업별',
    accUsageTabDate: '날짜별',
    accUsageTabModel: '모델별',
    accUsageEmpty: '사용 기록이 없습니다',
    accCreditsUnit: '크레딧',
    accConsumeLabel: '사용',
    accIncomeLabel: '적립',
    accCallsFmt: '{n}회',
    accUnitSeconds: '초',
    accUnitChars: '자',
    accTotalFmt: '합계 {n} {unit}',
    accModelsTooltip: '모델: {list}',
    accLedgerGift: '증정',
    accLedgerRedeem: '교환',
    accLedgerConsume: '소비',
    accLedgerAdjust: '조정',
    accLedgerRefund: '환불',
    accLedgerExpire: '만료',
    accLedgerOther: '기타',
    priceFree: '무료',
    pricePerMinute: '{price} 크레딧/분',
    pricePerKTokens: '{price} 크레딧/1K 토큰',
    pricePerKChars: '{price} 크레딧/1K 문자',
    taskNameLive: '실시간 전사',
    taskNameOffline: '오프라인 인식',
    taskNameSummary: '회의 요약',
    taskNameTts: '음성 합성',
    taskNameTranslate: '번역',
    taskNameOther: '기타 호출',
    stepKindAsr: '음성 인식',
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
    floatingBall: 'フローティングボール',
    floatingBallShow: 'フローティングボールを表示',
    floatingBallHide: 'フローティングボールを隠す',
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
    errPageTitle: 'このページで問題が発生しました',
    errPageHint: '録音と履歴はそのまま残ります。別のページに戻るか、アプリを再読み込みしてください。',
    errReload: '再読み込み',
    claimBalanceLabel: '現在のクレジット残高',
    claimOnceNote: '1 台のパソコンにつき 1 回のみ受け取れます。付与クレジットの有効期限は 90 日です。',
    comNeverRemind: '今後表示しない',
    updNeverTitle: '更新通知を停止しますか？',
    updNeverDesc: '確認すると、今後新しいバージョンの通知は表示されません。',
    lowCreditTopUp: 'チャージする',
    lowCreditNeverTitle: '残高不足の通知を停止しますか？',
    lowCreditNeverDesc: '確認すると残高不足の通知は表示されなくなります。残高が警告しきい値を上回ると、通知は自動的に再有効化されます。',
    comPrev: '前へ',
    comNext: '次へ',
    comNotSet: '未設定',
    updNewVersion: '新バージョン v{version} が利用可能です',
    updCurrentVersion: '（現在 v{version}）',
    noticeLabel: '重要なお知らせ',
    accLicenseTitle: 'マイライセンス',
    accLicenseCopied: 'ライセンスをコピーしました',
    accLicenseBackupHint: 'ライセンスはあなたのアカウントです。チャージや復旧に必要なので、バックアップをおすすめします。',
    accUsageTitle: 'クレジット消費',
    accUsageTabTasks: 'タスク別',
    accUsageTabDate: '日付別',
    accUsageTabModel: 'モデル別',
    accUsageEmpty: '消費記録はまだありません',
    accCreditsUnit: 'クレジット',
    accConsumeLabel: '消費',
    accIncomeLabel: '入金',
    accCallsFmt: '{n}回',
    accUnitSeconds: '秒',
    accUnitChars: '文字',
    accTotalFmt: '計 {n} {unit}',
    accModelsTooltip: 'モデル: {list}',
    accLedgerGift: '付与',
    accLedgerRedeem: '交換',
    accLedgerConsume: '消費',
    accLedgerAdjust: '調整',
    accLedgerRefund: '返金',
    accLedgerExpire: '期限切れ',
    accLedgerOther: 'その他',
    priceFree: '無料',
    pricePerMinute: '{price} クレジット/分',
    pricePerKTokens: '{price} クレジット/1Kトークン',
    pricePerKChars: '{price} クレジット/1K文字',
    taskNameLive: 'リアルタイム文字起こし',
    taskNameOffline: 'オフライン認識',
    taskNameSummary: '会議要約',
    taskNameTts: '音声合成',
    taskNameTranslate: '翻訳',
    taskNameOther: 'その他の呼び出し',
    stepKindAsr: '音声認識',
  },
}
