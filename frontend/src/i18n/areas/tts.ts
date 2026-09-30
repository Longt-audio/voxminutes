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
  /** 空列表时的具体失败原因（listRemoteModels 的真实错误，便于自助排查） */
  ttsNoModelsReason: string
  /** 空列表/失败时的重试按钮 */
  ttsRetry: string
  /** 空列表时跳转用户中心配置远程服务 */
  ttsGoAccount: string
  /** 音频已合成但自动播放被拒（此前被 .catch(()=>{}) 静默吞掉） */
  ttsPlayFailed: string
  ttsColModel: string
  ttsColVoice: string
  ttsProviderDefault: string
  ttsSaved: string
  ttsSaving: string
  ttsCurrentEffective: string
  ttsInUse: string
  ttsVoiceSourceCustom: string
  /** 未自定义且供应商按文本语言自动选音色时的说明，{voice} 为自动选中的音色名 */
  ttsVoiceSourceAuto: string
  /** 自动音色的简短标注（跟在音色名后面） */
  ttsVoiceSourceAutoShort: string
  /** 合成面板提示：本次合成将使用：{desc} */
  ttsSynthWillUse: string

  // ── 2026-09-22 语音合成页重做（去掉 Tab，补 MiMo 音色/音色设计/产品说明） ──
  ttsMiMoIntro: string
  // ── 三 tab 改版新增（2026-09-23）────────────────────────────────────────────
  /** tab 标题 */
  ttsTabMimo: string
  ttsTabSupertonic: string
  ttsTabDefaults: string
  /** 顶部两个模型的介绍卡 */
  ttsIntroCardTitle: string
  ttsIntroCardHint: string
  ttsModelMimoName: string
  ttsModelMimoDesc: string
  ttsModelSuperName: string
  ttsModelSuperDesc: string
  /** 价格文案（{price} 为每千字符的积分数） */
  ttsPricePerK: string
  ttsNoPrice: string
  /** 要朗读的语言选择 */
  ttsSpokenLangLabel: string
  ttsVoiceIdLabel: string
  ttsVoiceFemale: string
  ttsVoiceMale: string
  /** Supertonic 不支持风格指令的说明 */
  ttsSuperNoInstructions: string
  /** 默认音色 tab */
  ttsCurrentDefaultFor: string
  ttsNotSet: string
  ttsSetDefaultForLang: string
  ttsDefaultsTabHint: string
  ttsDefaultsColLang: string
  ttsDefaultsColModel: string
  ttsDefaultsColVoice: string
  ttsDefaultsColStatus: string
  ttsDefaultsColActions: string
  ttsStatusCustom: string
  ttsStatusDefault: string
  ttsRestoreDefault: string
  ttsClearAllDefaults: string
  ttsClearAllDone: string
  /** 试听用的示例文本 */
  ttsDemoText: string
  ttsSupportedLangs: string
  ttsSupportedLangsValue: string
  ttsHighlightsTitle: string
  ttsHl1: string
  ttsHl2: string
  ttsHl3: string
  ttsHl4: string
  ttsHl5: string
  ttsPresetSection: string
  ttsVoiceGalleryHint: string
  ttsVoiceLangZh: string
  ttsVoiceLangEn: string
  ttsGenderFemale: string
  ttsGenderMale: string
  ttsDesignSection: string
  ttsDesignHint: string
  ttsDesignPlaceholder: string
  ttsDesignTextLabel: string
  ttsDesignTextPlaceholder: string
  ttsDesignGenerate: string
  ttsDesignUnavailable: string
  ttsStyleLabel: string
  ttsStyleHint: string
  ttsStylePlaceholder: string
  ttsFreeNote: string
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
    ttsNoModelsReason: 'Reason',
    ttsRetry: 'Retry',
    ttsGoAccount: 'Open Account',
    ttsPlayFailed: 'Audio generated but playback was blocked by the browser. Tap the player below to play it.',
    ttsColModel: 'Model',
    ttsColVoice: 'Default voice',
    ttsProviderDefault: 'Provider default',
    ttsSaved: 'Audio saved to {path}',
    ttsSaving: 'Saving audio…',
    ttsCurrentEffective: 'Active',
    ttsInUse: 'In use',
    ttsVoiceSourceCustom: 'Custom',
    ttsVoiceSourceAuto: 'Auto ({voice} for English)',
    ttsVoiceSourceAutoShort: 'Auto · by text language',
    ttsSynthWillUse: 'This synthesis will use: {desc}',
    ttsMiMoIntro: 'MiMo-V2.5 TTS is Xiaomi\'s self-developed large-scale speech synthesis model. Built on a proprietary audio tokenizer and multi-codebook joint speech-text modeling, it was pretrained on over 100 million hours of audio. It supports multi-granularity style control (paragraph → sentence → word → character), mid-sentence mood changes, natural prosody, paralinguistic events (sighs, pauses, laughter), Chinese dialects, character role-play, and even singing — all in one model.',
    ttsSupportedLangs: 'Supported languages',
    ttsSupportedLangsValue: 'Chinese / English (official voice library: 5 Chinese + 4 English)',
    ttsHighlightsTitle: 'Highlights',
    ttsHl1: '9 official voices covering lively, intellectual, sunny and mature timbres',
    ttsHl2: 'Style control from paragraph down to a single character; mood can shift mid-sentence',
    ttsHl3: 'Natural sighs, pauses, hesitations and laughter without post-processing',
    ttsHl4: 'Dialects and accents: Northeastern, Sichuan, Henan, Cantonese, Taiwan; role-play characters such as Sun Wukong and Lin Daiyu',
    ttsHl5: 'Voice design: describe a brand-new timbre in natural language — no reference audio needed',
    ttsPresetSection: 'Preset voices',
    ttsVoiceGalleryHint: 'Pick a voice, then generate. The voice list follows the official MiMo-V2.5 voice library.',
    ttsVoiceLangZh: 'Chinese',
    ttsVoiceLangEn: 'English',
    ttsGenderFemale: 'Female',
    ttsGenderMale: 'Male',
    ttsDesignSection: 'Voice design — describe a voice in words',
    ttsDesignHint: 'Describe the timbre you want in 1-4 sentences: gender and age, texture, emotion and tone, speed and rhythm, optionally persona or scene. The model creates a brand-new voice from the description — no reference audio required.',
    ttsDesignPlaceholder: 'e.g. A middle-aged man speaking standard Mandarin, deep and magnetic voice, calm and measured, like a documentary narrator',
    ttsDesignTextLabel: 'Text to speak',
    ttsDesignTextPlaceholder: 'Enter what this voice should say…',
    ttsDesignGenerate: 'Design & preview',
    ttsDesignUnavailable: 'The voice-design model (mimo-v2.5-tts-voicedesign) is not enabled on the server yet, so this feature is unavailable for now.',
    ttsStyleLabel: 'Style / emotion instruction (optional)',
    ttsStyleHint: 'Written in natural language, e.g. "speak faster, cheerful tone". Can also use inline tags such as (singing) or [pause].',
    ttsStylePlaceholder: 'e.g. Cheerful and brisk tone, slightly faster pace',
    ttsFreeNote: 'MiMo TTS is free of upstream charges for a limited time; only VoxMinutes credits are consumed.',
    ttsTabMimo: 'MiMo TTS',
    ttsTabSupertonic: 'Supertonic 3',
    ttsTabDefaults: 'Default voices',
    ttsIntroCardTitle: 'Two speech engines, pick per language',
    ttsIntroCardHint: 'In the translation page, reading the original or the translation automatically uses the default voice you set for that language.',
    ttsModelMimoName: 'MiMo-V2.5 TTS',
    ttsModelMimoDesc: 'Chinese / English · 9 preset voices · natural-language style and emotion control.',
    ttsModelSuperName: 'Supertonic 3 (self-hosted)',
    ttsModelSuperDesc: '31 languages (no Chinese) · 10 voices (0-4 female, 5-9 male) · runs on our own server, cheapest per character.',
    ttsPricePerK: '{price} credits / 1000 chars',
    ttsNoPrice: 'Price not published',
    ttsSpokenLangLabel: 'Language of the text to speak',
    ttsVoiceIdLabel: 'Voice',
    ttsVoiceFemale: 'Female',
    ttsVoiceMale: 'Male',
    ttsSuperNoInstructions: 'Supertonic 3 does not support style instructions; it reads according to the text.',
    ttsCurrentDefaultFor: 'Current default for {lang}:',
    ttsNotSet: 'not set',
    ttsSetDefaultForLang: 'Set as default for {lang}',
    ttsDefaultsTabHint: 'Automatic reading picks the voice below according to the language of the text. Click "Set as default for …" in either model tab to change one.',
    ttsDefaultsColLang: 'Language',
    ttsDefaultsColModel: 'Model',
    ttsDefaultsColVoice: 'Voice',
    ttsDefaultsColStatus: 'Status',
    ttsDefaultsColActions: 'Actions',
    ttsStatusCustom: 'Custom',
    ttsStatusDefault: 'Default',
    ttsRestoreDefault: 'Restore default',
    ttsClearAllDefaults: 'Restore all to default',
    ttsClearAllDone: 'All languages restored to default',
    ttsDemoText: 'This is a sample sentence for previewing the voice.',
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
    ttsNoModelsReason: '原因',
    ttsRetry: '重试',
    ttsGoAccount: '去用户中心',
    ttsPlayFailed: '音频已合成，但浏览器阻止了自动播放。请点下方播放器手动播放。',
    ttsColModel: '模型',
    ttsColVoice: '默认语音',
    ttsProviderDefault: '供应商默认',
    ttsSaved: '音频已保存到 {path}',
    ttsSaving: '正在保存音频…',
    ttsCurrentEffective: '当前生效',
    ttsInUse: '使用中',
    ttsVoiceSourceCustom: '自定义',
    ttsVoiceSourceAuto: '自动（英文用 {voice}）',
    ttsVoiceSourceAutoShort: '自动·按文本语言',
    ttsSynthWillUse: '本次合成将使用：{desc}',
    ttsMiMoIntro: 'MiMo-V2.5 TTS 是小米自研的大规模语音合成模型，基于自研音频 tokenizer 与多码本语音-文本联合建模，预训练语料超过 1 亿小时。支持多粒度风格控制（段落 → 句子 → 词 → 字）、句内情绪切换、自然韵律，能原生生成叹气、停顿、迟疑、笑声等副语言事件，还支持方言、角色扮演，甚至同一模型内唱歌。',
    ttsSupportedLangs: '支持语言',
    ttsSupportedLangsValue: '中文 / 英文（官方音色库：5 个中文 + 4 个英文）',
    ttsHighlightsTitle: '核心优势',
    ttsHl1: '9 个官方音色，覆盖活泼少女、知性女声、阳光少年、成熟男声等',
    ttsHl2: '风格控制细到「字」一级，句内即可切换情绪',
    ttsHl3: '原生生成叹气、停顿、迟疑、笑声等副语言事件，无需后期处理',
    ttsHl4: '方言口音：东北话 / 四川话 / 河南话 / 粤语 / 台湾腔；角色扮演：孙悟空、林黛玉',
    ttsHl5: '音色设计：用一段自然语言描述即可生成全新音色，无需参考音频',
    ttsPresetSection: '预置音色',
    ttsVoiceGalleryHint: '选一个音色即可合成；音色列表与官方 MiMo-V2.5 音色库一致。',
    ttsVoiceLangZh: '中文',
    ttsVoiceLangEn: '英文',
    ttsGenderFemale: '女声',
    ttsGenderMale: '男声',
    ttsDesignSection: '音色设计 —— 用一段话描述音色',
    ttsDesignHint: '用 1~4 句话描述想要的音色：性别与年龄、音质质感、情绪语气、语速节奏，可再加人设或场景。模型会据此生成一个全新音色，无需任何参考音频。',
    ttsDesignPlaceholder: '例：一位中年男性，说标准普通话，嗓音低沉有磁性，语速平稳，像纪录片旁白解说员',
    ttsDesignTextLabel: '要合成的文本',
    ttsDesignTextPlaceholder: '输入让这个音色朗读的内容…',
    ttsDesignGenerate: '生成并试听',
    ttsDesignUnavailable: '服务端尚未上架音色设计模型（mimo-v2.5-tts-voicedesign），该功能暂不可用。',
    ttsStyleLabel: '风格 / 情感指令（可选）',
    ttsStyleHint: '用自然语言描述，例如「语速稍快、语气欢快」；正文里也可用 (唱歌)、[停顿] 这类音频标签。',
    ttsStylePlaceholder: '例：语气欢快、语速稍快',
    ttsFreeNote: 'MiMo TTS 上游目前限时免费，仅消耗 VoxMinutes 积分。',
    ttsTabMimo: 'MiMo TTS',
    ttsTabSupertonic: 'Supertonic 3',
    ttsTabDefaults: '默认音色',
    ttsIntroCardTitle: '两个语音引擎，按语种各取所长',
    ttsIntroCardHint: '翻译页播放原文/译文时，会按该语言的默认音色自动选择模型与音色。',
    ttsModelMimoName: 'MiMo-V2.5 TTS',
    ttsModelMimoDesc: '中 / 英 · 9 个预置音色 · 支持自然语言风格与情感控制。',
    ttsModelSuperName: 'Supertonic 3（自建）',
    ttsModelSuperDesc: '31 语种（不含中文）· 10 个音色（0-4 女声，5-9 男声）· 跑在自己的服务器上，单价最低。',
    ttsPricePerK: '{price} 积分 / 千字符',
    ttsNoPrice: '未公布价格',
    ttsSpokenLangLabel: '要朗读的文本语言',
    ttsVoiceIdLabel: '音色',
    ttsVoiceFemale: '女声',
    ttsVoiceMale: '男声',
    ttsSuperNoInstructions: 'Supertonic 3 不支持风格指令，按文本自行朗读。',
    ttsCurrentDefaultFor: '{lang} 当前默认：',
    ttsNotSet: '未设置',
    ttsSetDefaultForLang: '设为{lang}默认音色',
    ttsDefaultsTabHint: '自动朗读时按文本语言选择下表的音色。要修改某个语言，请到上面两个模型 tab 里选好音色后点「设为…默认音色」。',
    ttsDefaultsColLang: '语言',
    ttsDefaultsColModel: '模型',
    ttsDefaultsColVoice: '音色',
    ttsDefaultsColStatus: '状态',
    ttsDefaultsColActions: '操作',
    ttsStatusCustom: '自定义',
    ttsStatusDefault: '默认',
    ttsRestoreDefault: '恢复默认',
    ttsClearAllDefaults: '全部恢复默认',
    ttsClearAllDone: '已把全部语言恢复为默认音色',
    ttsDemoText: '这是一句用于试听音色的示例文本。',
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
    ttsNoModelsReason: '원인',
    ttsRetry: '다시 시도',
    ttsGoAccount: '계정 열기',
    ttsPlayFailed: '오디오는 생성되었지만 브라우저가 자동 재생을 차단했습니다. 아래 플레이어를 눌러 재생하세요.',
    ttsColModel: '모델',
    ttsColVoice: '기본 음성',
    ttsProviderDefault: '공급자 기본값',
    ttsSaved: '오디오가 {path}에 저장되었습니다',
    ttsSaving: '오디오 저장 중…',
    ttsCurrentEffective: '현재 적용',
    ttsInUse: '사용 중',
    ttsVoiceSourceCustom: '사용자 지정',
    ttsVoiceSourceAuto: '자동(영어는 {voice})',
    ttsVoiceSourceAutoShort: '자동·텍스트 언어 기준',
    ttsSynthWillUse: '이번 합성에 사용: {desc}',
    ttsMiMoIntro: 'MiMo-V2.5 TTS는 샤오미가 자체 개발한 대규모 음성 합성 모델로, 자체 오디오 토크나이저와 다중 코드북 음성-텍스트 결합 모델링을 기반으로 1억 시간 이상의 오디오로 사전 학습되었습니다. 문단→문장→단어→글자 단위의 다중 스타일 제어, 문장 중간 감정 전환, 자연스러운 운율, 한숨·쉼·머뭇거림·웃음 같은 부언어 요소를 지원하며 방언, 역할극, 같은 모델에서의 노래까지 가능합니다.',
    ttsSupportedLangs: '지원 언어',
    ttsSupportedLangsValue: '중국어 / 영어 (공식 음색 5개 중국어 + 4개 영어)',
    ttsHighlightsTitle: '주요 장점',
    ttsHl1: '공식 음색 9종(활발한 소녀, 지적인 여성, 밝은 소년, 성숙한 남성 등)',
    ttsHl2: '문단부터 글자까지 스타일 제어, 문장 중간 감정 전환 가능',
    ttsHl3: '한숨·쉼·머뭇거림·웃음을 후처리 없이 자연 생성',
    ttsHl4: '방언: 동북어·사천어·하남어·광둥어·대만어 / 역할극: 손오공, 임대옥',
    ttsHl5: '음색 디자인: 자연어 설명만으로 새 음색 생성(참조 오디오 불필요)',
    ttsPresetSection: '기본 음색',
    ttsVoiceGalleryHint: '음색을 고르고 바로 생성하세요. 목록은 공식 MiMo-V2.5 음색 라이브러리와 동일합니다.',
    ttsVoiceLangZh: '중국어',
    ttsVoiceLangEn: '영어',
    ttsGenderFemale: '여성',
    ttsGenderMale: '남성',
    ttsDesignSection: '음색 디자인 — 말로 음색 설명하기',
    ttsDesignHint: '원하는 음색을 1~4문장으로 설명하세요: 성별과 나이, 음질, 감정과 어조, 속도와 리듬, 필요하면 인물이나 장면까지. 참조 오디오 없이 새 음색이 생성됩니다.',
    ttsDesignPlaceholder: '예: 표준 중국어를 쓰는 중년 남성, 낮고 매력적인 목소리, 차분한 속도, 다큐멘터리 내레이션처럼',
    ttsDesignTextLabel: '합성할 텍스트',
    ttsDesignTextPlaceholder: '이 음색이 읽을 내용을 입력하세요…',
    ttsDesignGenerate: '생성 및 미리듣기',
    ttsDesignUnavailable: '서버에 음색 디자인 모델(mimo-v2.5-tts-voicedesign)이 아직 등록되지 않아 이 기능은 사용할 수 없습니다.',
    ttsStyleLabel: '스타일 / 감정 지시(선택)',
    ttsStyleHint: '자연어로 작성하세요. 예: "조금 빠르고 밝은 어조". 본문에 (노래), [쉼] 같은 태그도 사용할 수 있습니다.',
    ttsStylePlaceholder: '예: 밝은 어조, 조금 빠른 속도',
    ttsFreeNote: 'MiMo TTS는 현재 상위 요금이 한시적으로 무료이며, VoxMinutes 크레딧만 소모됩니다.',
    ttsTabMimo: 'MiMo TTS',
    ttsTabSupertonic: 'Supertonic 3',
    ttsTabDefaults: '기본 음색',
    ttsIntroCardTitle: '두 가지 음성 엔진, 언어별로 선택',
    ttsIntroCardHint: '번역 페이지에서 원문/번역문을 읽을 때 해당 언어의 기본 음색이 자동으로 사용됩니다.',
    ttsModelMimoName: 'MiMo-V2.5 TTS',
    ttsModelMimoDesc: '중국어 / 영어 · 프리셋 음색 9종 · 자연어 스타일·감정 제어 지원.',
    ttsModelSuperName: 'Supertonic 3 (자체 호스팅)',
    ttsModelSuperDesc: '31개 언어(중국어 제외) · 음색 10종(0-4 여성, 5-9 남성) · 자체 서버에서 실행되어 단가가 가장 낮습니다.',
    ttsPricePerK: '1,000자당 {price} 크레딧',
    ttsNoPrice: '가격 미공개',
    ttsSpokenLangLabel: '읽을 텍스트의 언어',
    ttsVoiceIdLabel: '음색',
    ttsVoiceFemale: '여성',
    ttsVoiceMale: '남성',
    ttsSuperNoInstructions: 'Supertonic 3은 스타일 지시를 지원하지 않으며 텍스트대로 읽습니다.',
    ttsCurrentDefaultFor: '{lang} 현재 기본값:',
    ttsNotSet: '미설정',
    ttsSetDefaultForLang: '{lang} 기본 음색으로 설정',
    ttsDefaultsTabHint: '자동 읽기는 텍스트 언어에 따라 아래 음색을 사용합니다. 변경하려면 위 모델 탭에서 음색을 고른 뒤 「…기본 음색으로 설정」을 누르세요.',
    ttsDefaultsColLang: '언어',
    ttsDefaultsColModel: '모델',
    ttsDefaultsColVoice: '음색',
    ttsDefaultsColStatus: '상태',
    ttsDefaultsColActions: '작업',
    ttsStatusCustom: '사용자 지정',
    ttsStatusDefault: '기본',
    ttsRestoreDefault: '기본값 복원',
    ttsClearAllDefaults: '모두 기본값으로 복원',
    ttsClearAllDone: '모든 언어를 기본 음색으로 복원했습니다',
    ttsDemoText: '음색을 미리 들어보기 위한 예시 문장입니다.',
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
    ttsNoModelsReason: '理由',
    ttsRetry: '再試行',
    ttsGoAccount: 'アカウントを開く',
    ttsPlayFailed: '音声は生成されましたが、ブラウザが自動再生をブロックしました。下のプレーヤーで再生してください。',
    ttsColModel: 'モデル',
    ttsColVoice: 'デフォルト音声',
    ttsProviderDefault: 'プロバイダ既定',
    ttsSaved: '音声を {path} に保存しました',
    ttsSaving: '音声を保存中…',
    ttsCurrentEffective: '現在の適用',
    ttsInUse: '使用中',
    ttsVoiceSourceCustom: 'カスタム',
    ttsVoiceSourceAuto: '自動（英語は {voice}）',
    ttsVoiceSourceAutoShort: '自動・テキスト言語別',
    ttsSynthWillUse: 'この合成で使用: {desc}',
    ttsMiMoIntro: 'MiMo-V2.5 TTS は小米が自社開発した大規模音声合成モデルです。独自のオーディオトークナイザーとマルチコードブック音声-テキスト同時モデリングを基盤に、1億時間以上の音声で事前学習。段落→文→語→文字までの多粒度スタイル制御、文中での感情切替、自然な韻律、ため息・間・言い淀み・笑い声などの副言語要素、方言、ロールプレイ、さらに同一モデルでの歌唱まで対応します。',
    ttsSupportedLangs: '対応言語',
    ttsSupportedLangsValue: '中国語 / 英語（公式音色：中国語 5 + 英語 4）',
    ttsHighlightsTitle: '主な特長',
    ttsHl1: '公式音色 9 種（活発な少女、知的な女性、明るい少年、落ち着いた男性など）',
    ttsHl2: '段落から文字単位までのスタイル制御、文中での感情切替が可能',
    ttsHl3: 'ため息・間・言い淀み・笑い声を後処理なしで自然に生成',
    ttsHl4: '方言：東北語・四川語・河南語・広東語・台湾語／ロールプレイ：孫悟空、林黛玉',
    ttsHl5: '音色デザイン：自然言語の説明だけで新しい音色を生成（参照音声不要）',
    ttsPresetSection: 'プリセット音色',
    ttsVoiceGalleryHint: '音色を選んでそのまま生成できます。一覧は公式 MiMo-V2.5 音色ライブラリと同一です。',
    ttsVoiceLangZh: '中国語',
    ttsVoiceLangEn: '英語',
    ttsGenderFemale: '女性',
    ttsGenderMale: '男性',
    ttsDesignSection: '音色デザイン — 言葉で音色を説明する',
    ttsDesignHint: '希望する音色を 1〜4 文で説明してください：性別と年齢、音質、感情と口調、速度とリズム、必要なら人物像や場面も。参照音声なしで新しい音色が生成されます。',
    ttsDesignPlaceholder: '例：標準中国語を話す中年男性、低く響く声、落ち着いた速度、ドキュメンタリーのナレーターのように',
    ttsDesignTextLabel: '読み上げるテキスト',
    ttsDesignTextPlaceholder: 'この音色に読ませる内容を入力…',
    ttsDesignGenerate: '生成して試聴',
    ttsDesignUnavailable: 'サーバーに音色デザインモデル（mimo-v2.5-tts-voicedesign）がまだ登録されていないため、この機能は利用できません。',
    ttsStyleLabel: 'スタイル / 感情の指示（任意）',
    ttsStyleHint: '自然言語で記述します（例：「少し速めで明るい口調」）。本文中に (歌唱)、[間] などのタグも使えます。',
    ttsStylePlaceholder: '例：明るい口調、やや速め',
    ttsFreeNote: 'MiMo TTS は現在上流が期間限定で無料です（VoxMinutes のクレジットのみ消費）。',
    ttsTabMimo: 'MiMo TTS',
    ttsTabSupertonic: 'Supertonic 3',
    ttsTabDefaults: '既定の音色',
    ttsIntroCardTitle: '2つの音声エンジンを言語ごとに使い分け',
    ttsIntroCardHint: '翻訳ページで原文・訳文を読み上げる際、その言語の既定の音色が自動で使われます。',
    ttsModelMimoName: 'MiMo-V2.5 TTS',
    ttsModelMimoDesc: '中国語 / 英語 · プリセット音色9種 · 自然言語によるスタイル・感情の指示に対応。',
    ttsModelSuperName: 'Supertonic 3（自前ホスト）',
    ttsModelSuperDesc: '31言語（中国語を除く）· 音色10種（0-4 女性、5-9 男性）· 自前サーバーで動作し、単価が最も安価。',
    ttsPricePerK: '1,000文字あたり {price} クレジット',
    ttsNoPrice: '価格未公表',
    ttsSpokenLangLabel: '読み上げるテキストの言語',
    ttsVoiceIdLabel: '音色',
    ttsVoiceFemale: '女性',
    ttsVoiceMale: '男性',
    ttsSuperNoInstructions: 'Supertonic 3 はスタイル指示に対応せず、テキストどおりに読み上げます。',
    ttsCurrentDefaultFor: '{lang} の現在の既定:',
    ttsNotSet: '未設定',
    ttsSetDefaultForLang: '{lang} の既定の音色に設定',
    ttsDefaultsTabHint: '自動読み上げはテキストの言語に応じて下表の音色を使います。変更するには上のモデルタブで音色を選び「…の既定の音色に設定」を押してください。',
    ttsDefaultsColLang: '言語',
    ttsDefaultsColModel: 'モデル',
    ttsDefaultsColVoice: '音色',
    ttsDefaultsColStatus: '状態',
    ttsDefaultsColActions: '操作',
    ttsStatusCustom: 'カスタム',
    ttsStatusDefault: '既定',
    ttsRestoreDefault: '既定に戻す',
    ttsClearAllDefaults: 'すべて既定に戻す',
    ttsClearAllDone: 'すべての言語を既定の音色に戻しました',
    ttsDemoText: '音色の試聴用のサンプル文です。',
  },
}
