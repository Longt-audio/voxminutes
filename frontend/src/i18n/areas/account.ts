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
  },
}
