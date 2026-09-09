import type { Language } from '../languages'

/** 用户中心页文案：积分余额、信息区、反馈。 */
export interface AccountMessages {
  accTitle: string
  accCredits: string
  accInfo: string
  accFeedback: string
  accFeedbackPlaceholder: string
  accScreenshotAttached: string
  accContact: string
  accSubmit: string
  accLogHint: string
  accFeedbackEmpty: string
  accFeedbackSent: string
  accFeedbackFailed: string
  accRefresh: string
  accRefreshed: string
  accRemoteTitle: string
  accRemoteUrl: string
  accRemoteKey: string
  accRemoteEnable: string
  accRemoteTest: string
  accRemoteTesting: string
  accRemoteSaved: string
  accRemoteSaveFailed: string
  accRemoteNeedKey: string
  accRemoteNeedUrl: string
  accRemoteOnline: string
  accRemoteOffline: string
  accRemoteAutoSaved: string
  accModelsTitle: string
  accModelsHint: string
  accModelKindAsr: string
  accModelKindTranslate: string
  accModelKindSummary: string
  accModelKindTts: string
  accModeStreaming: string
  accModeBatch: string
  accSpeedTitle: string
  accSpeedRun: string
  accSpeedRunAll: string
  accSpeedTest: string
  accSpeedRunning: string
  accSpeedColModel: string
  accSpeedColLatency: string
  accSpeedHint: string
  accBillingTitle: string
  accBillingHint: string
  accRedeemTitle: string
  accRedeemPlaceholder: string
  accRedeemBtn: string
  accRedeemSuccess: string
  accRegisterBtn: string
  accRegisterSuccess: string
  accLowBalance: string
  accLedgerTitle: string
  accLedgerEmpty: string
}

export const ACCOUNT_MESSAGES: Record<Language, AccountMessages> = {
  en: {
    accTitle: 'Account',
    accCredits: 'Credits balance',
    accInfo: 'Messages',
    accFeedback: 'Feedback',
    accFeedbackPlaceholder: 'Describe the problem or suggestion…',
    accScreenshotAttached: 'Screenshot attached',
    accContact: 'Contact (email, optional)',
    accSubmit: 'Submit',
    accLogHint: 'Need to send logs? Find them in the app data directory (logs/) and email or paste them to us.',
    accFeedbackEmpty: 'Please enter feedback content',
    accFeedbackSent: 'Feedback sent, thanks!',
    accFeedbackFailed: 'Failed to send: {error}',
    accRefresh: 'Refresh',
    accRefreshed: 'Refreshed',
    accRemoteTitle: 'Remote service',
    accRemoteUrl: 'Server address',
    accRemoteKey: 'License (authorization code)',
    accRemoteEnable: 'Enable remote service',
    accRemoteTest: 'Test connection',
    accRemoteTesting: 'Testing…',
    accRemoteSaved: 'Saved',
    accRemoteSaveFailed: 'Save failed: {error}',
    accRemoteNeedKey: 'Enter a license first to test connection',
    accRemoteNeedUrl: 'Enter a server address first',
    accRemoteOnline: 'Online',
    accRemoteOffline: 'Offline',
    accRemoteAutoSaved: 'Changes are saved automatically',
    accModelsTitle: 'Available models',
    accModelsHint: 'Model choices are made where each feature is used (recording dialog / translation page / summary).',
    accModelKindAsr: 'ASR',
    accModelKindTranslate: 'Translate',
    accModelKindSummary: 'Summary',
    accModelKindTts: 'TTS',
    accModeStreaming: 'Streaming',
    accModeBatch: 'File',
    accSpeedTitle: 'Model speed test',
    accSpeedRun: 'Run speed test',
    accSpeedRunAll: 'Test all models',
    accSpeedTest: 'Test',
    accSpeedRunning: 'Testing…',
    accSpeedColModel: 'Model',
    accSpeedColLatency: 'Latency',
    accSpeedHint: 'Measures round-trip latency only (no quality evaluation, no credits charged).',
    accBillingTitle: 'Credits usage',
    accBillingHint: 'Credits are charged on the gateway per model price × usage (ASR per second, LLM per token, TTS per char). Estimate cost from each model price below before you record or summarize.',
    accRedeemTitle: 'Redeem code',
    accRedeemPlaceholder: 'Enter redemption code',
    accRedeemBtn: 'Redeem',
    accRedeemSuccess: 'Redeemed +{added} credits',
    accRegisterBtn: 'Get 200 credits',
    accRegisterSuccess: 'Registered, balance {credits} credits',
    accLowBalance: 'Low balance ({credits} left), please top up',
    accLedgerTitle: 'Credit history',
    accLedgerEmpty: 'No records yet',
  },
  zh: {
    accTitle: '用户中心',
    accCredits: '积分余额',
    accInfo: '信息发布',
    accFeedback: '意见反馈',
    accFeedbackPlaceholder: '描述问题或建议…',
    accScreenshotAttached: '已附加截图',
    accContact: '联系方式（邮箱，可选）',
    accSubmit: '提交',
    accLogHint: '需要发送日志？请在应用数据目录的 logs/ 下找到日志文件，通过邮件或粘贴发给我们。',
    accFeedbackEmpty: '请输入反馈内容',
    accFeedbackSent: '反馈已提交，感谢！',
    accFeedbackFailed: '提交失败：{error}',
    accRefresh: '刷新',
    accRefreshed: '已刷新',
    accRemoteTitle: '远程服务',
    accRemoteUrl: '服务器地址',
    accRemoteKey: '授权码',
    accRemoteEnable: '启用远程服务',
    accRemoteTest: '测试连接',
    accRemoteTesting: '检测中…',
    accRemoteSaved: '已保存',
    accRemoteSaveFailed: '保存失败：{error}',
    accRemoteNeedKey: '请先填写授权码再测试连接',
    accRemoteNeedUrl: '请先填写服务器地址',
    accRemoteOnline: '在线',
    accRemoteOffline: '离线',
    accRemoteAutoSaved: '修改会自动保存',
    accModelsTitle: '可用模型',
    accModelsHint: '模型选择请到各功能使用处进行（录制对话框 / 翻译页 / 会议总结）。',
    accModelKindAsr: 'ASR',
    accModelKindTranslate: '翻译',
    accModelKindSummary: '总结',
    accModelKindTts: 'TTS',
    accModeStreaming: '流式',
    accModeBatch: '文件',
    accSpeedTitle: '模型测速',
    accSpeedRun: '开始测速',
    accSpeedRunAll: '全部测速',
    accSpeedTest: '测速',
    accSpeedRunning: '测速中…',
    accSpeedColModel: '模型',
    accSpeedColLatency: '延迟',
    accSpeedHint: '仅测往返延迟（不做质量评估，不扣积分）。',
    accBillingTitle: '积分使用',
    accBillingHint: '积分在网关按「模型单价 × 用量」计算：ASR 按秒、LLM 按 token、TTS 按字符。录音或总结前可参考下方各模型单价估算成本。',
    accRedeemTitle: '充值 / 兑换',
    accRedeemPlaceholder: '输入兑换码',
    accRedeemBtn: '兑换',
    accRedeemSuccess: '兑换成功，到账 +{added} 积分',
    accRegisterBtn: '领取 200 积分',
    accRegisterSuccess: '已领取，当前 {credits} 积分',
    accLowBalance: '积分不足（剩余 {credits}），请及时充值',
    accLedgerTitle: '积分明细',
    accLedgerEmpty: '暂无记录',
  },
  ko: {
    accTitle: '계정',
    accCredits: '크레딧 잔액',
    accInfo: '메시지',
    accFeedback: '피드백',
    accFeedbackPlaceholder: '문제나 제안을 설명하세요…',
    accScreenshotAttached: '스크린샷 첨부됨',
    accContact: '연락처(이메일, 선택)',
    accSubmit: '제출',
    accLogHint: '로그를 보내야 하나요? 앱 데이터 디렉터리의 logs/ 에서 찾아 이메일이나 붙여넣기로 보내주세요.',
    accFeedbackEmpty: '피드백 내용을 입력하세요',
    accFeedbackSent: '피드백이 제출되었습니다. 감사합니다!',
    accFeedbackFailed: '제출 실패: {error}',
    accRefresh: '새로고침',
    accRefreshed: '새로고침됨',
    accRemoteTitle: '원격 서비스',
    accRemoteUrl: '서버 주소',
    accRemoteKey: '라이선스(인증 코드)',
    accRemoteEnable: '원격 서비스 사용',
    accRemoteTest: '연결 테스트',
    accRemoteTesting: '테스트 중…',
    accRemoteSaved: '저장됨',
    accRemoteSaveFailed: '저장 실패: {error}',
    accRemoteNeedKey: '먼저 라이선스를 입력하세요',
    accRemoteNeedUrl: '먼저 서버 주소를 입력하세요',
    accRemoteOnline: '온라인',
    accRemoteOffline: '오프라인',
    accRemoteAutoSaved: '변경 사항은 자동 저장됩니다',
    accModelsTitle: '사용 가능한 모델',
    accModelsHint: '모델 선택은 각 기능 사용처(녹음 대화상자 / 번역 페이지 / 요약)에서 하세요.',
    accModelKindAsr: 'ASR',
    accModelKindTranslate: '번역',
    accModelKindSummary: '요약',
    accModelKindTts: 'TTS',
    accModeStreaming: '스트리밍',
    accModeBatch: '파일',
    accSpeedTitle: '모델 속도 테스트',
    accSpeedRun: '속도 테스트',
    accSpeedRunAll: '전체 테스트',
    accSpeedTest: '테스트',
    accSpeedRunning: '테스트 중…',
    accSpeedColModel: '모델',
    accSpeedColLatency: '지연',
    accSpeedHint: '왕복 지연만 측정합니다(품질 평가 없음, 크레딧 차감 없음).',
    accBillingTitle: '크레딧 사용',
    accBillingHint: '크레딧은 게이트웨이에서「모델 단가 × 사용량」으로 계산됩니다(ASR 초당, LLM 토큰당, TTS 문자당). 녹음이나 요약 전 아래 단가로 비용을 추정하세요.',
    accRedeemTitle: '코드 교환',
    accRedeemPlaceholder: '교환 코드 입력',
    accRedeemBtn: '교환',
    accRedeemSuccess: '+{added} 크레딧 충전됨',
    accRegisterBtn: '200 크레딧 받기',
    accRegisterSuccess: '등록 완료, 잔액 {credits} 크레딧',
    accLowBalance: '잔액 부족(남은 {credits}), 충전하세요',
    accLedgerTitle: '크레딧 내역',
    accLedgerEmpty: '기록 없음',
  },
  ja: {
    accTitle: 'アカウント',
    accCredits: 'クレジット残高',
    accInfo: 'メッセージ',
    accFeedback: 'フィードバック',
    accFeedbackPlaceholder: '問題や提案を説明してください…',
    accScreenshotAttached: 'スクリーンショット添付済み',
    accContact: '連絡先(メール、任意)',
    accSubmit: '送信',
    accLogHint: 'ログを送るには、アプリデータディレクトリの logs/ にあるログファイルをメールまたは貼り付けでお送りください。',
    accFeedbackEmpty: 'フィードバック内容を入力してください',
    accFeedbackSent: 'フィードバックを送信しました。ありがとうございます！',
    accFeedbackFailed: '送信に失敗: {error}',
    accRefresh: '更新',
    accRefreshed: '更新しました',
    accRemoteTitle: 'リモートサービス',
    accRemoteUrl: 'サーバーアドレス',
    accRemoteKey: 'ライセンス(認証コード)',
    accRemoteEnable: 'リモートサービスを有効化',
    accRemoteTest: '接続テスト',
    accRemoteTesting: 'テスト中…',
    accRemoteSaved: '保存しました',
    accRemoteSaveFailed: '保存失敗: {error}',
    accRemoteNeedKey: '先にライセンスを入力してください',
    accRemoteNeedUrl: '先にサーバーアドレスを入力してください',
    accRemoteOnline: 'オンライン',
    accRemoteOffline: 'オフライン',
    accRemoteAutoSaved: '変更は自動保存されます',
    accModelsTitle: '利用可能なモデル',
    accModelsHint: 'モデル選択は各機能の使用場所(録音ダイアログ / 翻訳ページ / 要約)で行ってください。',
    accModelKindAsr: 'ASR',
    accModelKindTranslate: '翻訳',
    accModelKindSummary: '要約',
    accModelKindTts: 'TTS',
    accModeStreaming: 'ストリーミング',
    accModeBatch: 'ファイル',
    accSpeedTitle: 'モデル速度テスト',
    accSpeedRun: '速度テスト',
    accSpeedRunAll: '全モデルテスト',
    accSpeedTest: 'テスト',
    accSpeedRunning: 'テスト中…',
    accSpeedColModel: 'モデル',
    accSpeedColLatency: '遅延',
    accSpeedHint: '往復遅延のみ測定(品質評価なし、クレジット消費なし)。',
    accBillingTitle: 'クレジット使用',
    accBillingHint: 'クレジットはゲートウェイで「モデル単価 × 使用量」で計算されます(ASR 秒単位、LLM トークン単位、TTS 文字単位)。録音や要約の前に下の単価でコストを見積もってください。',
    accRedeemTitle: 'チャージ / コード交換',
    accRedeemPlaceholder: '交換コードを入力',
    accRedeemBtn: '交換',
    accRedeemSuccess: '+{added} クレジットをチャージしました',
    accRegisterBtn: '200 クレジットを受け取る',
    accRegisterSuccess: '登録完了、残高 {credits} クレジット',
    accLowBalance: '残高不足(残り {credits})、チャージしてください',
    accLedgerTitle: 'クレジット履歴',
    accLedgerEmpty: '履歴なし',
  },
}
