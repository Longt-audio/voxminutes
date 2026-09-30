import type { Language } from '../languages'

/** 会议总结功能文案。 */
export interface SummaryMessages {
  sumButton: string
  sumDialogTitle: string
  sumTabWeb: string
  sumTabApi: string
  sumTabLocal: string
  sumTabRemote: string
  sumRemoteHint: string
  sumRemoteNeedConfig: string
  sumPromptPreset: string
  sumPromptEdit: string
  sumGenerate: string
  sumGenerating: string
  /** 思考模式下的进度提示（{chars} 为已收到的思维链字数） */
  sumThinking: string
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
  /** api tab 内的引导文案：自定义 API 配置入口已移到「设置 → 自定义 LLM」 */
  sumApiConfigGuide: string
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
  sumRemoteNote: string
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
  /** 「输出语言」附加提示词区块的标题（作用于所有模板的二级提示词） */
  sumOutputLanguageLabel: string
  /** 输出语言下拉的说明（按所选语言自动生成输出语言指令，拼接在所有模板之后） */
  sumOutputLanguageHint: string
  /** 「附加要求」输入框的标签（可空的额外要求，拼接在输出语言指令之后） */
  sumExtraLabel: string
  /** 「附加要求」输入框的占位提示 */
  sumExtraPlaceholder: string
  sumResultTitle: string
  sumRegenerate: string
  sumExportMd: string
  /** 导出 PDF（打印预览窗口 + 系统打印对话框「存为 PDF」） */
  sumExportPdf: string
  sumExportedTo: string
  sumExportFailed: string
  /** 折叠块标题：AI 思考过程（reasoning_content） */
  sumThinkingTitle: string
  /** 思考过程字数：{chars} = 字符数 */
  sumThinkingChars: string
  /** 思考过程折叠块展开但还没内容时的占位 */
  sumThinkingWaiting: string
  /** 工具栏：展开 Markdown 源码（默认不展开） */
  sumShowSource: string
  /** 工具栏：收起源码，只看格式预览 */
  sumPreviewOnly: string
  /** 「Markdown 源码」按钮的悬停提示 */
  sumSourceHint: string
  /** 源码编辑框占位符 */
  sumSourcePlaceholder: string
  /** 源码已修改未保存的提示 */
  sumSourceDirty: string
  /** 「带格式复制」（写 text/html + text/plain 到剪贴板） */
  sumCopyRich: string
  /** 模型只思考、没产出正文时的提示 */
  sumEmptyWithThinking: string
  /** 总结输出顶到 max_tokens 被截断时的内联警告（内容已保存，可重新生成） */
  sumOutputTruncated: string
}

export const SUMMARY_MESSAGES: Record<Language, SummaryMessages> = {
  en: {
    sumButton: 'Meeting Summary',
    sumDialogTitle: 'Meeting Summary',
    sumTabWeb: 'AI Websites',
    sumTabApi: 'API',
    sumTabLocal: 'Local Model',
    sumTabRemote: 'Remote',
    sumRemoteHint: 'Summarize with an LLM from the remote gateway. Credits are charged by token.',
    sumRemoteNeedConfig: 'The remote service is not configured or not enabled. Configure it in Account → Remote service.',
    sumPromptPreset: 'Prompt preset',
    sumPromptEdit: 'Edit prompt',
    sumGenerate: 'Generate',
    sumGenerating: 'Generating…',
    sumThinking: 'Thinking… ({chars} chars of reasoning received)',
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
    sumApiConfigGuide: 'Custom APIs are configured in Settings → Custom LLM',
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
    sumRemoteNote: 'Summaries now run through the remote gateway (configured in the Remote Service tab). Choose the summary model below.',
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
    sumOutputLanguageLabel: 'Output language',
    sumOutputLanguageHint: 'An output-language instruction is generated from your choice and appended after every template prompt.',
    sumExtraLabel: 'Additional requirements',
    sumExtraPlaceholder: 'Optional: other requirements for the summary, e.g. focus on action items, keep it under 500 words…',
    sumResultTitle: 'Meeting Summary',
    sumRegenerate: 'Regenerate',
    sumExportMd: 'Export MD',
    sumExportPdf: 'Export PDF',
    sumExportedTo: 'Exported to {path}',
    sumExportFailed: 'Export failed: {error}',
    sumThinkingTitle: 'AI reasoning',
    sumThinkingChars: '({chars} chars)',
    sumThinkingWaiting: 'Waiting for the model to reason…',
    sumShowSource: 'Markdown source',
    sumPreviewOnly: 'Preview only',
    sumSourceHint: 'Show the Markdown source on the right — you can edit it (collapsed by default)',
    sumSourcePlaceholder: 'Markdown source…',
    sumSourceDirty: 'Source edited — save to apply',
    sumCopyRich: 'Copy formatted',
    sumEmptyWithThinking: 'The model only produced reasoning and no summary text (this run is not charged). Regenerate, or switch to a faster summary model.',
    sumOutputTruncated: 'The summary may be incomplete: the model hit its output limit and the text was cut off. You can regenerate it.',
  },
  zh: {
    sumButton: '会议总结',
    sumDialogTitle: '会议总结',
    sumTabWeb: 'AI 网站',
    sumTabApi: 'API',
    sumTabLocal: '本地模型',
    sumTabRemote: '远程',
    sumRemoteHint: '使用远程网关的大模型生成总结，按 token 消耗积分',
    sumRemoteNeedConfig: '远程服务未配置或未开启，请到 用户中心 → 远程服务 配置',
    sumPromptPreset: '总结模板',
    sumPromptEdit: '编辑 Prompt',
    sumGenerate: '生成总结',
    sumGenerating: '正在生成…',
    sumThinking: '正在思考…（已收到 {chars} 字推理）',
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
    sumApiConfigGuide: '自定义 API 在「设置 → 自定义 LLM」中配置',
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
    sumRemoteNote: '会议总结已改为走远程服务网关（在「远程服务」tab 配置服务器地址与授权码），请在下方选择总结模型。',
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
    sumOutputLanguageLabel: '输出语言',
    sumOutputLanguageHint: '按所选语言自动生成一句输出语言指令，拼接到所有模板之后。',
    sumExtraLabel: '附加要求',
    sumExtraPlaceholder: '可选：对总结的其他要求，如重点记录行动项、控制在 500 字以内…',
    sumResultTitle: '会议总结',
    sumRegenerate: '重新生成',
    sumExportMd: '导出 MD',
    sumExportPdf: '导出 PDF',
    sumExportedTo: '已导出到 {path}',
    sumExportFailed: '导出失败：{error}',
    sumThinkingTitle: 'AI 思考过程',
    sumThinkingChars: '（{chars} 字）',
    sumThinkingWaiting: '等待模型输出思考内容…',
    sumShowSource: 'Markdown 源码',
    sumPreviewOnly: '只看预览',
    sumSourceHint: '展开右侧 Markdown 源码，可直接编辑（默认不展开）',
    sumSourcePlaceholder: 'Markdown 源码…',
    sumSourceDirty: '源码已修改，保存后生效',
    sumCopyRich: '带格式复制',
    sumEmptyWithThinking: '模型只输出了思考过程、没有生成正文（本轮不计费）。可以重新生成，或换一个更快的总结模型。',
    sumOutputTruncated: '总结可能不完整：模型输出达到上限被截断。可以重新生成。',
  },
  ko: {
    sumButton: '회의 요약',
    sumDialogTitle: '회의 요약',
    sumTabWeb: 'AI 웹사이트',
    sumTabApi: 'API',
    sumTabLocal: '로컬 모델',
    sumTabRemote: '원격',
    sumRemoteHint: '원격 게이트웨이의 대형 모델로 요약을 생성하며 토큰에 따라 포인트가 차감됩니다',
    sumRemoteNeedConfig: '원격 서비스가 설정되지 않았거나 꺼져 있습니다. 계정 → 원격 서비스에서 설정하세요',
    sumPromptPreset: '요약 템플릿',
    sumPromptEdit: '프롬프트 편집',
    sumGenerate: '요약 생성',
    sumGenerating: '생성 중…',
    sumThinking: '생각하는 중… (추론 {chars}자 수신)',
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
    sumApiConfigGuide: '사용자 지정 API는 「설정 → 사용자 정의 LLM」에서 설정합니다',
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
    sumRemoteNote: '회의 요약은 이제 원격 게이트웨이(원격 서비스 탭에서 구성)를 사용합니다. 아래에서 요약 모델을 선택하세요.',
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
    sumOutputLanguageLabel: '출력 언어',
    sumOutputLanguageHint: '선택한 언어로 출력 언어 지시문이 자동 생성되어 모든 템플릿 뒤에 추가됩니다.',
    sumExtraLabel: '추가 요구 사항',
    sumExtraPlaceholder: '선택 사항: 요약에 대한 기타 요구 사항 (예: 실행 항목 중심, 500자 이내)…',
    sumResultTitle: '회의 요약',
    sumRegenerate: '다시 생성',
    sumExportMd: 'MD 내보내기',
    sumExportPdf: 'PDF로보내기',
    sumExportedTo: '내보내기 완료: {path}',
    sumExportFailed: '내보내기 실패: {error}',
    sumThinkingTitle: 'AI 사고 과정',
    sumThinkingChars: '({chars}자)',
    sumThinkingWaiting: '모델이 사고하는 것을 기다리는 중…',
    sumShowSource: 'Markdown 소스',
    sumPreviewOnly: '미리보기만',
    sumSourceHint: '오른쪽에 Markdown 소스를 표시합니다. 직접 편집할 수 있습니다(기본은 접힘)',
    sumSourcePlaceholder: 'Markdown 소스…',
    sumSourceDirty: '소스가 수정되었습니다. 저장하면 반영됩니다',
    sumCopyRich: '서식 포함 복사',
    sumEmptyWithThinking: '모델이 사고 과정만 출력하고 본문을 만들지 않았습니다(이번 실행은 과금되지 않습니다). 다시 생성하거나 더 빠른 요약 모델로 바꿔 주세요.',
    sumOutputTruncated: '요약이 불완전할 수 있습니다: 모델 출력이 한도에 도달해 잘렸습니다. 다시 생성해 보세요.',
  },
  ja: {
    sumButton: '会議の要約',
    sumDialogTitle: '会議の要約',
    sumTabWeb: 'AI サイト',
    sumTabApi: 'API',
    sumTabLocal: 'ローカルモデル',
    sumTabRemote: 'リモート',
    sumRemoteHint: 'リモートゲートウェイの大規模モデルで要約を生成します。トークンに応じてポイントを消費します',
    sumRemoteNeedConfig: 'リモートサービスが未設定または無効です。アカウント → リモートサービス で設定してください',
    sumPromptPreset: '要約テンプレート',
    sumPromptEdit: 'プロンプトを編集',
    sumGenerate: '要約を生成',
    sumGenerating: '生成中…',
    sumThinking: '考え中…（推論 {chars} 文字を受信）',
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
    sumApiConfigGuide: 'カスタム API は「設定 → カスタム LLM」で設定します',
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
    sumRemoteNote: '会議要約はリモートゲートウェイ（リモートサービス タブで構成）を使用します。以下で要約モデルを選択してください。',
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
    sumOutputLanguageLabel: '出力言語',
    sumOutputLanguageHint: '選択した言語から出力言語の指示文が自動生成され、すべてのテンプレートの後に付加されます。',
    sumExtraLabel: '追加要件',
    sumExtraPlaceholder: '任意：要約へのその他の要件（例：アクションアイテムを重点的に、500字以内）…',
    sumResultTitle: '会議の要約',
    sumRegenerate: '再生成',
    sumExportMd: 'MD をエクスポート',
    sumExportPdf: 'PDF をエクスポート',
    sumExportedTo: 'エクスポートしました: {path}',
    sumExportFailed: 'エクスポートに失敗しました: {error}',
    sumThinkingTitle: 'AI の思考プロセス',
    sumThinkingChars: '（{chars} 文字）',
    sumThinkingWaiting: 'モデルが思考するのを待っています…',
    sumShowSource: 'Markdown ソース',
    sumPreviewOnly: 'プレビューのみ',
    sumSourceHint: '右側に Markdown ソースを表示します（直接編集できます・既定は折りたたみ）',
    sumSourcePlaceholder: 'Markdown ソース…',
    sumSourceDirty: 'ソースを編集しました。保存すると反映されます',
    sumCopyRich: '書式付きでコピー',
    sumEmptyWithThinking: 'モデルが思考プロセスだけを出力し、本文を生成しませんでした（今回の実行は課金されません）。再生成するか、より高速な要約モデルに切り替えてください。',
    sumOutputTruncated: '要約が不完全な可能性があります：モデルの出力が上限に達して途中で切れました。再生成をお試しください。',
  },
}
