import type { Language } from '../languages'

/** 语音合成（TTS）页文案：远程 TTS 模型试听 + 默认语音设置。 */
export interface TtsMessages {
  ttsTitle: string
  ttsSubtitle: string
  ttsTextPlaceholder: string
  ttsVoice: string
  ttsVoicePlaceholder: string
  ttsGenerate: string
  ttsGenerating: string
  ttsGenerateFailed: string
  ttsDefaultTab: string
  ttsSetAsDefault: string
  ttsSetDefaultDone: string
  ttsResetDefault: string
  ttsResetDefaultDone: string
  ttsNoModelsHint: string
  ttsColModel: string
  ttsColVoice: string
  ttsProviderDefault: string
  ttsSaved: string
  ttsSaving: string
}

export const TTS_MESSAGES: Record<Language, TtsMessages> = {
  en: {
    ttsTitle: 'Speech Synthesis',
    ttsSubtitle: 'Preview remote TTS models and set the default voice',
    ttsTextPlaceholder: 'Enter text to synthesize…',
    ttsVoice: 'Voice',
    ttsVoicePlaceholder: 'Leave empty to use the provider default',
    ttsGenerate: 'Generate',
    ttsGenerating: 'Generating…',
    ttsGenerateFailed: 'Generation failed',
    ttsDefaultTab: 'Default Voice',
    ttsSetAsDefault: 'Set as default',
    ttsSetDefaultDone: 'Default voice set to {voice}',
    ttsResetDefault: 'Reset to provider default',
    ttsResetDefaultDone: 'Reset to provider default',
    ttsNoModelsHint: 'No remote TTS model available. Configure the remote service in the Account page first.',
    ttsColModel: 'Model',
    ttsColVoice: 'Default voice',
    ttsProviderDefault: 'Provider default',
    ttsSaved: 'Audio saved to {path}',
    ttsSaving: 'Saving audio…',
  },
  zh: {
    ttsTitle: '语音合成',
    ttsSubtitle: '试听远程 TTS 模型并设置默认语音',
    ttsTextPlaceholder: '输入要合成的文本…',
    ttsVoice: '音色',
    ttsVoicePlaceholder: '留空使用供应商默认音色',
    ttsGenerate: '生成语音',
    ttsGenerating: '合成中…',
    ttsGenerateFailed: '合成失败',
    ttsDefaultTab: '默认语音',
    ttsSetAsDefault: '设为默认语音',
    ttsSetDefaultDone: '已将默认语音设为 {voice}',
    ttsResetDefault: '恢复供应商默认',
    ttsResetDefaultDone: '已恢复为供应商默认音色',
    ttsNoModelsHint: '暂无可用远程 TTS 模型，请先到用户中心配置远程服务。',
    ttsColModel: '模型',
    ttsColVoice: '默认语音',
    ttsProviderDefault: '供应商默认',
    ttsSaved: '音频已保存到 {path}',
    ttsSaving: '正在保存音频…',
  },
  ko: {
    ttsTitle: '음성 합성',
    ttsSubtitle: '원격 TTS 모델을 미리 듣고 기본 음성을 설정합니다',
    ttsTextPlaceholder: '합성할 텍스트를 입력하세요…',
    ttsVoice: '음색',
    ttsVoicePlaceholder: '비우면 공급자 기본 음색 사용',
    ttsGenerate: '생성',
    ttsGenerating: '합성 중…',
    ttsGenerateFailed: '생성에 실패했습니다',
    ttsDefaultTab: '기본 음성',
    ttsSetAsDefault: '기본 음성으로 설정',
    ttsSetDefaultDone: '기본 음성을 {voice}(으)로 설정했습니다',
    ttsResetDefault: '공급자 기본값으로 재설정',
    ttsResetDefaultDone: '공급자 기본 음색으로 재설정했습니다',
    ttsNoModelsHint: '사용 가능한 원격 TTS 모델이 없습니다. 먼저 계정 페이지에서 원격 서비스를 설정하세요.',
    ttsColModel: '모델',
    ttsColVoice: '기본 음성',
    ttsProviderDefault: '공급자 기본값',
    ttsSaved: '오디오가 {path}에 저장되었습니다',
    ttsSaving: '오디오 저장 중…',
  },
  ja: {
    ttsTitle: '音声合成',
    ttsSubtitle: 'リモート TTS モデルを試聴し、デフォルト音声を設定します',
    ttsTextPlaceholder: '合成するテキストを入力…',
    ttsVoice: '音色',
    ttsVoicePlaceholder: '空欄でプロバイダ既定の音色を使用',
    ttsGenerate: '生成',
    ttsGenerating: '合成中…',
    ttsGenerateFailed: '生成に失敗しました',
    ttsDefaultTab: 'デフォルト音声',
    ttsSetAsDefault: 'デフォルト音声に設定',
    ttsSetDefaultDone: 'デフォルト音声を {voice} に設定しました',
    ttsResetDefault: 'プロバイダ既定に戻す',
    ttsResetDefaultDone: 'プロバイダ既定の音色に戻しました',
    ttsNoModelsHint: '利用可能なリモート TTS モデルがありません。先にアカウントページでリモートサービスを設定してください。',
    ttsColModel: 'モデル',
    ttsColVoice: 'デフォルト音声',
    ttsProviderDefault: 'プロバイダ既定',
    ttsSaved: '音声を {path} に保存しました',
    ttsSaving: '音声を保存中…',
  },
}
