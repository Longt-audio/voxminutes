import type { Language } from '../languages'

/** 会议总结功能文案。 */
export interface SummaryMessages {
  sumButton: string
  sumDialogTitle: string
  sumTabWeb: string
  sumTabApi: string
  sumTabLocal: string
  sumPromptPreset: string
  sumPromptEdit: string
  sumGenerate: string
  sumGenerating: string
  sumStop: string
  sumCopyAndOpen: string
  sumWebHint: string
  sumSitesManage: string
  sumSiteName: string
  sumSiteUrl: string
  sumAddSite: string
  sumResetSites: string
  sumApiNotConfigured: string
  sumGoSettings: string
  sumLocalNotInstalled: string
  sumDownloadInSettings: string
  sumResultPlaceholder: string
  sumSavedTo: string
  sumSaveFailed: string
  sumGenFailed: string
  sumTruncated: string
  sumNoTranscript: string
  sumSettingsTitle: string
  sumSettingsHint: string
  sumApiProtocol: string
  sumApiEndpoint: string
  sumApiKey: string
  sumApiKeyHint: string
  sumApiModel: string
  sumApiFetchModels: string
  sumApiTest: string
  sumApiTestOk: string
  sumApiTestFailed: string
  sumApiSaved: string
  sumApiSaveFailed: string
  sumProtocolOpenAI: string
  sumProtocolAnthropic: string
  sumViewSaved: string
  sumLocalModel: string
  sumCopiedFull: string
  sumFetchModelsFailed: string
  sumEndpointPlaceholder: string
  sumAddPreset: string
  sumPresetName: string
  sumPresetSaved: string
  sumResetPreset: string
  sumResultTitle: string
  sumRegenerate: string
  sumExportMd: string
  sumExportedTo: string
  sumExportFailed: string
}

export const SUMMARY_MESSAGES: Record<Language, SummaryMessages> = {
  en: {
    sumButton: 'Meeting Summary',
    sumDialogTitle: 'Meeting Summary',
    sumTabWeb: 'AI Websites',
    sumTabApi: 'API',
    sumTabLocal: 'Local Model',
    sumPromptPreset: 'Prompt preset',
    sumPromptEdit: 'Edit prompt',
    sumGenerate: 'Generate',
    sumGenerating: 'Generating…',
    sumStop: 'Stop',
    sumCopyAndOpen: 'Copy & open',
    sumWebHint: 'Pick a site — the prompt and transcript are copied and the site opens. Just paste.',
    sumSitesManage: 'Manage sites',
    sumSiteName: 'Name',
    sumSiteUrl: 'URL',
    sumAddSite: 'Add site',
    sumResetSites: 'Reset to defaults',
    sumApiNotConfigured: 'API not configured yet',
    sumGoSettings: 'Open Settings',
    sumLocalNotInstalled: 'Local summary model is not downloaded yet',
    sumDownloadInSettings: 'Download it in Settings',
    sumResultPlaceholder: 'The generated meeting summary will appear here.',
    sumSavedTo: 'Saved to {path}',
    sumSaveFailed: 'Failed to save: {error}',
    sumGenFailed: 'Generation failed: {error}',
    sumTruncated: 'Transcript is too long — kept the beginning and end.',
    sumNoTranscript: 'No transcript to summarize',
    sumSettingsTitle: 'Meeting Summary / AI',
    sumSettingsHint: 'Configure an API for meeting summaries. OpenAI-compatible and Anthropic formats are supported; local or LAN endpoints work too.',
    sumApiProtocol: 'Protocol',
    sumApiEndpoint: 'Endpoint',
    sumApiKey: 'API key',
    sumApiKeyHint: 'Leave empty for local/LAN endpoints',
    sumApiModel: 'Model',
    sumApiFetchModels: 'Fetch models',
    sumApiTest: 'Test connection',
    sumApiTestOk: 'Connection successful',
    sumApiTestFailed: 'Connection failed: {error}',
    sumApiSaved: 'Configuration saved',
    sumApiSaveFailed: 'Failed to save: {error}',
    sumProtocolOpenAI: 'OpenAI-compatible',
    sumProtocolAnthropic: 'Anthropic',
    sumViewSaved: 'View saved summary',
    sumLocalModel: 'Local summary model',
    sumCopiedFull: 'Copied — paste it on the site',
    sumFetchModelsFailed: 'Failed to fetch models: {error}',
    sumEndpointPlaceholder: 'http://192.168.1.10:8000/v1',
    sumAddPreset: 'Add preset',
    sumPresetName: 'Preset name',
    sumPresetSaved: 'Preset saved',
    sumResetPreset: 'Reset to default',
    sumResultTitle: 'Meeting Summary',
    sumRegenerate: 'Regenerate',
    sumExportMd: 'Export MD',
    sumExportedTo: 'Exported to {path}',
    sumExportFailed: 'Export failed: {error}',
  },
  zh: {
    sumButton: '会议总结',
    sumDialogTitle: '会议总结',
    sumTabWeb: 'AI 网站',
    sumTabApi: 'API',
    sumTabLocal: '本地模型',
    sumPromptPreset: '总结模板',
    sumPromptEdit: '编辑 Prompt',
    sumGenerate: '生成总结',
    sumGenerating: '正在生成…',
    sumStop: '停止',
    sumCopyAndOpen: '复制并打开',
    sumWebHint: '点击网站会自动复制 prompt 和会议记录并打开网站，粘贴即可',
    sumSitesManage: '管理网站',
    sumSiteName: '名称',
    sumSiteUrl: '网址',
    sumAddSite: '添加网站',
    sumResetSites: '重置为默认',
    sumApiNotConfigured: '尚未配置 API',
    sumGoSettings: '前往设置',
    sumLocalNotInstalled: '本地总结模型尚未下载',
    sumDownloadInSettings: '前往设置页下载',
    sumResultPlaceholder: '生成的会议纪要将显示在这里',
    sumSavedTo: '已保存到 {path}',
    sumSaveFailed: '保存失败：{error}',
    sumGenFailed: '生成失败：{error}',
    sumTruncated: '转写文本过长，已保留开头与结尾进行截断',
    sumNoTranscript: '当前没有可总结的转写内容',
    sumSettingsTitle: '会议总结 / AI',
    sumSettingsHint: '配置用于会议总结的 API，支持 OpenAI 兼容 / Anthropic 格式，本地或局域网端点也可以',
    sumApiProtocol: '协议',
    sumApiEndpoint: '端点地址',
    sumApiKey: 'API 密钥',
    sumApiKeyHint: '本地/局域网端点可留空',
    sumApiModel: '模型',
    sumApiFetchModels: '获取模型列表',
    sumApiTest: '测试连接',
    sumApiTestOk: '连接成功',
    sumApiTestFailed: '连接失败：{error}',
    sumApiSaved: '配置已保存',
    sumApiSaveFailed: '保存失败：{error}',
    sumProtocolOpenAI: 'OpenAI 兼容',
    sumProtocolAnthropic: 'Anthropic',
    sumViewSaved: '查看已保存的总结',
    sumLocalModel: '本地总结模型',
    sumCopiedFull: '已复制，到网站粘贴即可',
    sumFetchModelsFailed: '获取模型列表失败：{error}',
    sumEndpointPlaceholder: 'http://192.168.1.10:8000/v1',
    sumAddPreset: '新增模板',
    sumPresetName: '模板名称',
    sumPresetSaved: '模板已保存',
    sumResetPreset: '恢复默认',
    sumResultTitle: '会议总结',
    sumRegenerate: '重新生成',
    sumExportMd: '导出 MD',
    sumExportedTo: '已导出到 {path}',
    sumExportFailed: '导出失败：{error}',
  },
  ko: {
    sumButton: '회의 요약',
    sumDialogTitle: '회의 요약',
    sumTabWeb: 'AI 웹사이트',
    sumTabApi: 'API',
    sumTabLocal: '로컬 모델',
    sumPromptPreset: '요약 템플릿',
    sumPromptEdit: '프롬프트 편집',
    sumGenerate: '요약 생성',
    sumGenerating: '생성 중…',
    sumStop: '중지',
    sumCopyAndOpen: '복사 후 열기',
    sumWebHint: '사이트를 클릭하면 프롬프트와 회의록이 복사되고 사이트가 열립니다. 붙여넣기만 하면 됩니다',
    sumSitesManage: '사이트 관리',
    sumSiteName: '이름',
    sumSiteUrl: 'URL',
    sumAddSite: '사이트 추가',
    sumResetSites: '기본값으로 재설정',
    sumApiNotConfigured: 'API가 아직 구성되지 않았습니다',
    sumGoSettings: '설정으로 이동',
    sumLocalNotInstalled: '로컬 요약 모델이 다운로드되지 않았습니다',
    sumDownloadInSettings: '설정에서 다운로드하세요',
    sumResultPlaceholder: '생성된 회의 요약이 여기에 표시됩니다',
    sumSavedTo: '저장 위치: {path}',
    sumSaveFailed: '저장 실패: {error}',
    sumGenFailed: '생성 실패: {error}',
    sumTruncated: '받아쓰기가 너무 길어 앞뒤를 남기고 잘랐습니다',
    sumNoTranscript: '요약할 받아쓰기 내용이 없습니다',
    sumSettingsTitle: '회의 요약 / AI',
    sumSettingsHint: '회의 요약에 사용할 API를 구성합니다. OpenAI 호환 / Anthropic 형식을 지원하며 로컬 또는 LAN 엔드포인트도 사용할 수 있습니다',
    sumApiProtocol: '프로토콜',
    sumApiEndpoint: '엔드포인트 주소',
    sumApiKey: 'API 키',
    sumApiKeyHint: '로컬/LAN 엔드포인트는 비워 둘 수 있습니다',
    sumApiModel: '모델',
    sumApiFetchModels: '모델 목록 가져오기',
    sumApiTest: '연결 테스트',
    sumApiTestOk: '연결 성공',
    sumApiTestFailed: '연결 실패: {error}',
    sumApiSaved: '구성이 저장되었습니다',
    sumApiSaveFailed: '저장 실패: {error}',
    sumProtocolOpenAI: 'OpenAI 호환',
    sumProtocolAnthropic: 'Anthropic',
    sumViewSaved: '저장된 요약 보기',
    sumLocalModel: '로컬 요약 모델',
    sumCopiedFull: '복사되었습니다. 사이트에 붙여넣으세요',
    sumFetchModelsFailed: '모델 목록을 가져오지 못했습니다: {error}',
    sumEndpointPlaceholder: 'http://192.168.1.10:8000/v1',
    sumAddPreset: '프리셋 추가',
    sumPresetName: '프리셋 이름',
    sumPresetSaved: '프리셋이 저장되었습니다',
    sumResetPreset: '기본값으로 되돌리기',
    sumResultTitle: '회의 요약',
    sumRegenerate: '다시 생성',
    sumExportMd: 'MD 내보내기',
    sumExportedTo: '내보내기 완료: {path}',
    sumExportFailed: '내보내기 실패: {error}',
  },
  ja: {
    sumButton: '会議の要約',
    sumDialogTitle: '会議の要約',
    sumTabWeb: 'AI サイト',
    sumTabApi: 'API',
    sumTabLocal: 'ローカルモデル',
    sumPromptPreset: '要約テンプレート',
    sumPromptEdit: 'プロンプトを編集',
    sumGenerate: '要約を生成',
    sumGenerating: '生成中…',
    sumStop: '停止',
    sumCopyAndOpen: 'コピーして開く',
    sumWebHint: 'サイトをクリックすると、プロンプトと会議録をコピーしてサイトを開きます。貼り付けるだけで使えます',
    sumSitesManage: 'サイト管理',
    sumSiteName: '名前',
    sumSiteUrl: 'URL',
    sumAddSite: 'サイトを追加',
    sumResetSites: 'デフォルトに戻す',
    sumApiNotConfigured: 'API がまだ設定されていません',
    sumGoSettings: '設定を開く',
    sumLocalNotInstalled: 'ローカル要約モデルがダウンロードされていません',
    sumDownloadInSettings: '設定でダウンロードしてください',
    sumResultPlaceholder: '生成された議事録がここに表示されます',
    sumSavedTo: '保存しました: {path}',
    sumSaveFailed: '保存に失敗しました: {error}',
    sumGenFailed: '生成に失敗しました: {error}',
    sumTruncated: '文字起こしが長すぎるため、冒頭と末尾を残して省略しました',
    sumNoTranscript: '要約できる文字起こしがありません',
    sumSettingsTitle: '会議の要約 / AI',
    sumSettingsHint: '会議の要約に使用する API を設定します。OpenAI 互換 / Anthropic 形式に対応。ローカルや LAN のエンドポイントも使えます',
    sumApiProtocol: 'プロトコル',
    sumApiEndpoint: 'エンドポイント',
    sumApiKey: 'API キー',
    sumApiKeyHint: 'ローカル/LAN エンドポイントでは空欄可',
    sumApiModel: 'モデル',
    sumApiFetchModels: 'モデル一覧を取得',
    sumApiTest: '接続をテスト',
    sumApiTestOk: '接続に成功しました',
    sumApiTestFailed: '接続に失敗しました: {error}',
    sumApiSaved: '設定を保存しました',
    sumApiSaveFailed: '保存に失敗しました: {error}',
    sumProtocolOpenAI: 'OpenAI 互換',
    sumProtocolAnthropic: 'Anthropic',
    sumViewSaved: '保存済みの要約を見る',
    sumLocalModel: 'ローカル要約モデル',
    sumCopiedFull: 'コピーしました。サイトに貼り付けてください',
    sumFetchModelsFailed: 'モデル一覧の取得に失敗しました: {error}',
    sumEndpointPlaceholder: 'http://192.168.1.10:8000/v1',
    sumAddPreset: 'プリセットを追加',
    sumPresetName: 'プリセット名',
    sumPresetSaved: 'プリセットを保存しました',
    sumResetPreset: 'デフォルトに戻す',
    sumResultTitle: '会議の要約',
    sumRegenerate: '再生成',
    sumExportMd: 'MD をエクスポート',
    sumExportedTo: 'エクスポートしました: {path}',
    sumExportFailed: 'エクスポートに失敗しました: {error}',
  },
}
