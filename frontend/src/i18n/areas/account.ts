import type { Language } from '../languages'

/** 用户中心页文案：积分余额、信息区、反馈。 */
export interface AccountMessages {
  accTitle: string
  accCredits: string
  accInfo: string
  accFeedback: string
  accFeedbackPlaceholder: string
  accScreenshotAttached: string
  /** 截图文件选择框前的标注「附加截图」（与日志附件区分） */
  accScreenshotAttach: string
  /** 「附加日志文件…」按钮：打开定位到日志目录的文件对话框，多选 .log */
  accAttachLogs: string
  accDiagAttach: string
  /** 日志附加范围下拉：最近 1 次运行 */
  accDiagRange1: string
  /** 日志附加范围下拉：最近 2 次运行（默认） */
  accDiagRange2: string
  /** 日志附加范围下拉：最近 5 次运行 */
  accDiagRange5: string
  /** 日志附加范围下拉：最近一天 */
  accDiagRangeDay: string
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
  accRemoteTestOk: string
  accRemoteTestFailed: string
  /** 服务器在线、但流式识别通道（wss 握手）不可用（2026-09-22 新增，两段式测试连接） */
  accStreamingUnavailable: string
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
  /** 只读服务器状态行：「服务器：」前缀 */
  accServerLabel: string
  /** 服务器地址旁的「默认」标注（使用内置地址时） */
  accServerDefaultBadge: string
  /** 服务器地址编辑按钮（铅笔）的悬浮提示 */
  accEditEndpoint: string
  /** 服务器地址编辑框下方提示：留空保存即恢复默认 */
  accEndpointEmptyHint: string
  /** 连接测试成功状态行「✓ 已连接」 */
  accConnected: string
  /** 测试成功后的按钮文案「重新测试」 */
  accRetest: string
  /** 失败后的按钮文案「重试」 */
  accRetry: string
  /** 领取积分主按钮「免费领取 100 积分（90 天内有效）」 */
  accClaimBtn: string
  /** 领取按钮副文案。
   *  ⚠️ 文案里的时长是**按生产库真实单价实算**的（系数 2.0x：文件转写 6.3h / 实时流式 5.0h），
   *  不是估算值。**改单价系数就必须回来改这里**，否则是不实宣传。
   *  依据与算式见 ~/vox/营销-赠送额度.md（该文件含复核脚本）。 */
  accClaimDesc: string
  accClaiming: string
  /** 新设备领取成功「✓ 已激活，已赠送 {credits} 积分」 */
  accClaimOk: string
  /** 老设备（is_new=false）「✓ 已恢复你的账号」 */
  accClaimRestored: string
  /** 账户页成功态附加提示：保存授权码 */
  accClaimSaveHint: string
  /** 领取成功态：授权码明文展示行的小标签 */
  accClaimLicenseLabel: string
  accClaimFailed: string
  /** 领取成功态：当前积分余额行 */
  accClaimBalance: string
  /** 领取成功态：官网（发卡站）链接前的说明文字 */
  accClaimWebsiteHint: string
  /** 任务消耗时间列的悬浮说明（时间跨度口径：首笔~末笔消费；末笔才是结算时间） */
  accTaskTimeSpanHint: string
  /** 积分余额卡下方的小字提示：赠送积分每设备限领一次、90 天有效 */
  accGiftNote: string
  /** 官网卡片标题 */
  accWebsite: string
  /** 官网卡片说明（购买充值卡、充值积分） */
  accWebsiteHint: string
  /** 积分余额卡内官网链接行的说明段「购买充值卡充值积分」（官网卡片已并入余额卡） */
  accWebsitePromo: string
}

export const ACCOUNT_MESSAGES: Record<Language, AccountMessages> = {
  en: {
    accTitle: 'Account',
    accCredits: 'Credits balance',
    accInfo: 'Messages',
    accFeedback: 'Feedback',
    accFeedbackPlaceholder: 'Describe the problem or suggestion…',
    accScreenshotAttached: 'Screenshot attached',
    accScreenshotAttach: 'Attach screenshot',
    accAttachLogs: 'Attach log files…',
    accDiagAttach: 'Recent run logs are attached automatically (sanitized)',
    accDiagRange1: 'Last run',
    accDiagRange2: 'Last 2 runs (default)',
    accDiagRange5: 'Last 5 runs',
    accDiagRangeDay: 'Last 24 hours',
    accContact: 'Contact (email, optional)',
    accSubmit: 'Submit',
    accLogHint: 'With the box checked we attach the most recent app log (license keys and usernames are stripped) so we can see what actually happened. Uncheck it if you prefer not to send logs.',
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
    accRemoteTestOk: 'Connected',
    accRemoteTestFailed: 'Connection failed. Check the address and your network.',
    accStreamingUnavailable: 'Streaming unavailable',
    accRemoteAutoSaved: 'Changes are saved automatically',
    accModelsTitle: 'Available models',
    accModelsHint: 'Model choices are made where each feature is used (recording dialog / translation page / summary).',
    accModelKindAsr: 'Speech Recognition',
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
    accBillingHint: 'Credits are charged on the gateway per model price × usage: speech recognition per minute, LLM per token, TTS per char.',
    accRedeemTitle: 'Redeem code',
    accRedeemPlaceholder: 'Enter redemption code',
    accRedeemBtn: 'Redeem',
    accRedeemSuccess: 'Redeemed +{added} credits',
    accRegisterBtn: 'Get 100 credits',
    accRegisterSuccess: 'Registered, balance {credits} credits',
    accLowBalance: 'Low balance ({credits} left), please top up',
    accLedgerTitle: 'Credit history',
    accLedgerEmpty: 'No records yet',
    accServerLabel: 'Server: ',
    accServerDefaultBadge: 'Default',
    accEditEndpoint: 'Edit server address',
    accEndpointEmptyHint: 'Leave empty and save to restore the built-in default address.',
    accConnected: 'Connected',
    accRetest: 'Retest',
    accRetry: 'Retry',
    accClaimBtn: 'Claim 100 free credits',
    // ⚠️ 「90 days」是必写项：Phase 1 没有邮箱/手机号，无法在到期前提醒用户，
    //    所以领取处必须明示有效期（见 PHASE_0_1_IMPLEMENTATION_PLAN.md 修正 4）。
    accClaimDesc: 'Register this device to get 100 credits (valid for 90 days) — about 3 hours of meeting transcription, or 2.5 hours of live captions.',
    accClaiming: 'Claiming…',
    accClaimOk: 'Activated — {credits} credits granted',
    accClaimRestored: 'Your account has been restored',
    accClaimSaveHint: 'Take a screenshot or copy and save your license — you will need it to restore your credits on another computer.',
    accClaimLicenseLabel: 'Your license (auto-filled and active)',
    accClaimFailed: 'Claim failed: {error}',
    accClaimBalance: 'Current balance: {credits} credits',
    accClaimWebsiteHint: 'Official website — buy top-up cards to recharge your credits:',
    accTaskTimeSpanHint: 'Time span of this task: first charge → last charge (local time). Speech recognition is billed after the recording ends, so the last charge can be much later than the first.',
    accGiftNote: 'Gifted credits can only be claimed once per computer device and are valid for 90 days.',
    accWebsite: 'Official website',
    accWebsiteHint: 'Buy top-up cards to recharge your credits:',
    accWebsitePromo: 'Buy top-up cards to recharge credits',
  },
  zh: {
    accTitle: '用户中心',
    accCredits: '积分余额',
    accInfo: '信息发布',
    accFeedback: '意见反馈',
    accFeedbackPlaceholder: '描述问题或建议…',
    accScreenshotAttached: '已附加截图',
    accScreenshotAttach: '附加截图',
    accAttachLogs: '附加日志文件…',
    accDiagAttach: '已自动附加最近运行日志（脱敏）',
    accDiagRange1: '最近 1 次运行',
    accDiagRange2: '最近 2 次运行（默认）',
    accDiagRange5: '最近 5 次运行',
    accDiagRangeDay: '最近一天',
    accContact: '联系方式（邮箱，可选）',
    accSubmit: '提交',
    accLogHint: '勾选后会自动附上最近的运行日志（授权码与用户名已脱敏），我们才能看到问题现场；不想发日志可以取消勾选。',
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
    accRemoteTestOk: '连接成功',
    accRemoteTestFailed: '连接失败，请检查服务器地址和网络',
    accStreamingUnavailable: '流式识别不可用',
    accRemoteAutoSaved: '修改会自动保存',
    accModelsTitle: '可用模型',
    accModelsHint: '模型选择请到各功能使用处进行（录制对话框 / 翻译页 / 会议总结）。',
    accModelKindAsr: '语音识别',
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
    accBillingHint: '积分在网关按「模型单价 × 用量」计费：语音识别按分钟、LLM 按 token、TTS 按字符。',
    accRedeemTitle: '充值 / 兑换',
    accRedeemPlaceholder: '输入兑换码',
    accRedeemBtn: '兑换',
    accRedeemSuccess: '兑换成功，到账 +{added} 积分',
    accRegisterBtn: '领取 100 积分',
    accRegisterSuccess: '已领取，当前 {credits} 积分',
    accLowBalance: '积分不足（剩余 {credits}），请及时充值',
    accLedgerTitle: '积分明细',
    accLedgerEmpty: '暂无记录',
    accServerLabel: '服务器：',
    accServerDefaultBadge: '默认',
    accEditEndpoint: '编辑服务器地址',
    accEndpointEmptyHint: '留空并保存，即恢复内置默认地址。',
    accConnected: '已连接',
    accRetest: '重新测试',
    accRetry: '重试',
    accClaimBtn: '免费领取 100 积分',
    // ⚠️ 「90 天内有效」是必写项，不是可选文案（见 PHASE_0_1_IMPLEMENTATION_PLAN.md 修正 4）
    accClaimDesc: '新设备注册即送 100 积分（90 天内有效），约可转写 3 小时会议录音，或 2.5 小时实时字幕。',
    accClaiming: '领取中…',
    accClaimOk: '已激活，已赠送 {credits} 积分',
    accClaimRestored: '已恢复你的账号',
    accClaimSaveHint: '建议截图或复制保存授权码，更换电脑时凭它恢复积分。',
    accClaimLicenseLabel: '你的授权码（已自动填入并生效）',
    accClaimFailed: '领取失败：{error}',
    accClaimBalance: '当前积分余额：{credits}',
    accClaimWebsiteHint: '官方网站，可购买充值卡充值积分：',
    accTaskTimeSpanHint: '时间跨度 = 该任务首笔 ~ 末笔消费的本地时间。语音识别费在录音结束后才结算，所以末笔时间可能明显晚于首笔。',
    accGiftNote: '每个电脑设备仅可领取一次赠送积分，赠送积分 90 天内有效。',
    accWebsite: '官方网站',
    accWebsiteHint: '购买充值卡、充值积分：',
    accWebsitePromo: '购买充值卡充值积分',
  },
  ko: {
    accTitle: '계정',
    accCredits: '크레딧 잔액',
    accInfo: '메시지',
    accFeedback: '피드백',
    accFeedbackPlaceholder: '문제나 제안을 설명하세요…',
    accScreenshotAttached: '스크린샷 첨부됨',
    accScreenshotAttach: '스크린샷 첨부',
    accAttachLogs: '로그 파일 첨부…',
    accDiagAttach: '최근 실행 로그가 자동으로 첨부됩니다(마스킹됨)',
    accDiagRange1: '최근 1회 실행',
    accDiagRange2: '최근 2회 실행(기본)',
    accDiagRange5: '최근 5회 실행',
    accDiagRangeDay: '최근 24시간',
    accContact: '연락처(이메일, 선택)',
    accSubmit: '제출',
    accLogHint: '체크하면 최근 실행 로그를 자동으로 첨부합니다(라이선스 키와 사용자 이름은 마스킹됨). 로그를 보내지 않으려면 체크를 해제하세요.',
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
    accRemoteTestOk: '연결 성공',
    accRemoteTestFailed: '연결 실패. 주소와 네트워크를 확인하세요.',
    accStreamingUnavailable: '스트리밍 사용 불가',
    accRemoteAutoSaved: '변경 사항은 자동 저장됩니다',
    accModelsTitle: '사용 가능한 모델',
    accModelsHint: '모델 선택은 각 기능 사용처(녹음 대화상자 / 번역 페이지 / 요약)에서 하세요.',
    accModelKindAsr: '음성 인식',
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
    accBillingHint: '크레딧은 게이트웨이에서「모델 단가 × 사용량」으로 청구됩니다: 음성 인식은 분당, LLM 토큰당, TTS 문자당.',
    accRedeemTitle: '코드 교환',
    accRedeemPlaceholder: '교환 코드 입력',
    accRedeemBtn: '교환',
    accRedeemSuccess: '+{added} 크레딧 충전됨',
    accRegisterBtn: '100 크레딧 받기',
    accRegisterSuccess: '등록 완료, 잔액 {credits} 크레딧',
    accLowBalance: '잔액 부족(남은 {credits}), 충전하세요',
    accLedgerTitle: '크레딧 내역',
    accLedgerEmpty: '기록 없음',
    accServerLabel: '서버: ',
    accServerDefaultBadge: '기본',
    accEditEndpoint: '서버 주소 편집',
    accEndpointEmptyHint: '비워 두고 저장하면 내장 기본 주소로 복원됩니다.',
    accConnected: '연결됨',
    accRetest: '다시 테스트',
    accRetry: '재시도',
    accClaimBtn: '100 크레딧 무료 받기',
    accClaimDesc: '새 기기를 등록하면 100 크레딧(90일 유효)을 드립니다. 회의 녹음 약 3시간 또는 실시간 자막 2.5시간 분량입니다.',
    accClaiming: '받는 중…',
    accClaimOk: '활성화됨 — {credits} 크레딧 지급',
    accClaimRestored: '계정이 복구되었습니다',
    accClaimSaveHint: '라이선스를 스크린샷하거나 복사해 보관하세요. 다른 컴퓨터에서 크레딧을 복구할 때 필요합니다.',
    accClaimLicenseLabel: '내 라이선스 (자동 입력되어 활성화됨)',
    accClaimFailed: '받기 실패: {error}',
    accClaimBalance: '현재 크레딧 잔액: {credits}',
    accClaimWebsiteHint: '공식 웹사이트 — 충전 카드를 구매해 크레딧을 충전할 수 있습니다:',
    accTaskTimeSpanHint: '시간 범위 = 이 작업의 첫 번째 ~ 마지막 과금 시각(로컬 시간). 음성 인식 요금은 녹음이 끝난 뒤 정산되므로 마지막 시각이 첫 시각보다 많이 늦을 수 있습니다.',
    accGiftNote: '증정 크레딧은 컴퓨터 기기당 한 번만 받을 수 있으며 90일간 유효합니다.',
    accWebsite: '공식 웹사이트',
    accWebsiteHint: '충전 카드를 구매해 크레딧을 충전하세요:',
    accWebsitePromo: '충전 카드 구매 · 크레딧 충전',
  },
  ja: {
    accTitle: 'アカウント',
    accCredits: 'クレジット残高',
    accInfo: 'メッセージ',
    accFeedback: 'フィードバック',
    accFeedbackPlaceholder: '問題や提案を説明してください…',
    accScreenshotAttached: 'スクリーンショット添付済み',
    accScreenshotAttach: 'スクリーンショットを添付',
    accAttachLogs: 'ログファイルを添付…',
    accDiagAttach: '直近の実行ログを自動添付します（マスク済み）',
    accDiagRange1: '前回の実行',
    accDiagRange2: '直近 2 回の実行（デフォルト）',
    accDiagRange5: '直近 5 回の実行',
    accDiagRangeDay: '最近 24 時間',
    accContact: '連絡先(メール、任意)',
    accSubmit: '送信',
    accLogHint: 'チェックすると直近の実行ログを自動で添付します（ライセンスキーとユーザー名はマスク済み）。送りたくない場合はチェックを外してください。',
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
    accRemoteTestOk: '接続成功',
    accRemoteTestFailed: '接続失敗。アドレスとネットワークを確認してください',
    accStreamingUnavailable: 'ストリーミング利用不可',
    accRemoteAutoSaved: '変更は自動保存されます',
    accModelsTitle: '利用可能なモデル',
    accModelsHint: 'モデル選択は各機能の使用場所(録音ダイアログ / 翻訳ページ / 要約)で行ってください。',
    accModelKindAsr: '音声認識',
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
    accBillingHint: 'クレジットはゲートウェイで「モデル単価 × 使用量」課金：音声認識は分単位、LLM はトークン単位、TTS は文字単位。',
    accRedeemTitle: 'チャージ / コード交換',
    accRedeemPlaceholder: '交換コードを入力',
    accRedeemBtn: '交換',
    accRedeemSuccess: '+{added} クレジットをチャージしました',
    accRegisterBtn: '100 クレジットを受け取る',
    accRegisterSuccess: '登録完了、残高 {credits} クレジット',
    accLowBalance: '残高不足(残り {credits})、チャージしてください',
    accLedgerTitle: 'クレジット履歴',
    accLedgerEmpty: '履歴なし',
    accServerLabel: 'サーバー：',
    accServerDefaultBadge: 'デフォルト',
    accEditEndpoint: 'サーバーアドレスを編集',
    accEndpointEmptyHint: '空欄のまま保存すると、組み込みのデフォルトアドレスに戻ります。',
    accConnected: '接続済み',
    accRetest: '再テスト',
    accRetry: '再試行',
    accClaimBtn: '100 クレジットを無料で受け取る',
    accClaimDesc: '新しいデバイスを登録すると 100 クレジット（90日間有効）を付与。会議録音 約3時間、またはリアルタイム字幕 2.5時間分です。',
    accClaiming: '受け取り中…',
    accClaimOk: '有効化しました — {credits} クレジットを付与',
    accClaimRestored: 'アカウントを復元しました',
    accClaimSaveHint: 'ライセンスをスクリーンショットまたはコピーして保存してください。別のパソコンでクレジットを復元する際に必要です。',
    accClaimLicenseLabel: 'あなたのライセンス（自動入力済み・有効）',
    accClaimFailed: '受け取り失敗: {error}',
    accClaimBalance: '現在のクレジット残高: {credits}',
    accClaimWebsiteHint: '公式サイト — チャージカードを購入してクレジットをチャージできます：',
    accTaskTimeSpanHint: '時間帯 = このタスクの最初〜最後の課金時刻（ローカル時間）。音声認識料金は録音終了後に精算されるため、最後の時刻は最初よりかなり遅くなることがあります。',
    accGiftNote: 'ギフトクレジットはパソコン1台につき1回のみ受け取れ、90日間有効です。',
    accWebsite: '公式サイト',
    accWebsiteHint: 'チャージカードの購入・クレジットのチャージ：',
    accWebsitePromo: 'チャージカード購入・クレジットチャージ',
  },
}
