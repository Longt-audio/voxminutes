import type { Language } from '@/i18n/languages'

export interface SummaryPromptPreset {
  id: string
  name: string
  content: string
  builtin?: boolean
}

const STORAGE_KEY = 'voxminutes-summary-prompts'

/** localStorage 持久化结构：仅存用户新增 prompt 与对内置 prompt 的覆盖（改名/改内容）。 */
interface PromptStorage {
  customs: SummaryPromptPreset[]
  overrides: Record<string, { name?: string; content?: string }>
}

// 7 个内置会议纪要模板 × 4 种 UI 语言（en/zh/ko/ja）
// 模板内容本身不含输出语言限制；输出语言由「输出语言附加提示词」（见文件底部）统一控制。
const BUILTIN_PROMPTS: Record<string, Record<Language, { name: string; content: string }>> = {
  default: {
    en: {
      name: 'Meeting Minutes (Default)',
      content: `# Role and Task
You are a rigorous, efficient professional meeting-minutes expert. Transform the following raw ASR (automatic speech recognition) transcript into clearly structured, well-focused meeting minutes in Markdown format.

# Transcript Preprocessing (ASR-specific)
1. Filter out filler: remove verbal tics and filler words (e.g. "um", "uh", "you know"), meaningless repetitions, and small talk.
2. Fix recognition errors: use context to intelligently correct homophone mistakes, typos, and misrecognized proper nouns.
3. Consolidate: merge discussion of the same topic scattered across different parts of the meeting into one coherent section, removing redundancy and disordered narration.

# Minutes Writing Principles
- Be concise: no sentence-by-sentence restatement or long-winded prose; prefer short sentences and verb-first phrasing.
- Prioritize what matters: heavily compress digressions and back-and-forth debate; keep only the core arguments.
- No speculation: if basic information is missing from the transcript (e.g. no deadline was mentioned), leave it blank or write "Not specified" — never fabricate.

# Output Format (follow this Markdown structure exactly)

## 📅 Meeting Overview
- **Topic**: [meeting topic]
- **Date/Time**: [fill in; write "Not specified" if unknown]
- **Participants**: [list the key speakers/attendees]

## 🎯 Key Topics and Decisions
*(Give each major topic its own subheading, in discussion order or grouped by business area. Keep the wording tight and focused.)*

### 1. [Topic One]
- **Discussion points**: [summarize each side's core viewpoints/disagreements in 1-2 short sentences]
- **Decision**: [the final solution or conclusion reached]

### 2. [Topic Two]
- **Discussion points**: [brief summary]
- **Decision**: [the final solution or conclusion reached]

## 📋 Action Items
*(Must be presented as a Markdown table. If the transcript mentions no owner or deadline, fill in "TBD" — do not leave cells empty.)*

| Action Item | Owner | Deadline |
| :--- | :--- | :--- |
| Example: Revise the Project A business plan deck | John | Before next Wednesday EOD |
| | | |

## 🚀 Summary and Next Steps
- **Overall progress**: [one sentence capturing the core milestone this meeting achieved]
- **Next steps**: [briefly describe the immediate actions the team should start after the meeting]`,
    },
    zh: {
      name: '会议记录（默认）',
      content: `# 角色与任务
你是一个严谨、高效的专业会议纪要专家。请将以下由 ASR（语音识别）生成的原始转写文本，提炼转化为一份结构清晰、重点突出的 Markdown 格式会议纪要。

# 文本预处理要求（针对 ASR 特性）
1. 过滤废话：自动剔除口头禅、语气词（如"啊""吧""然后"）、无意义的重复以及寒暄。
2. 纠正错别字：结合上下文语境，智能修正音近字、错别字或专有名词识别错误。
3. 归纳合并：将参会人员在不同时间段分散讨论的同一议题进行跨文本整合，消除内容上的因果倒置或啰嗦反复。

# 会议纪要生成原则
- 极简高效：拒绝逐句翻译或长篇大论，多用短句和动词开头的祈使句。
- 详略得当：大幅压缩讨论过程中的扯皮、发散内容，只保留核心论点。
- 拒绝推测：若文本中缺乏某项基本信息（如未明确提及截止时间），直接留空或写"未明确"，绝不胡乱编造。

# 输出格式（请严格按以下 Markdown 结构输出）

## 📅 会议基本信息
- **会议主题**：[填写主题]
- **会议时间**：[填写时间，若无则写"未明确"]
- **参会人员**：[列出核心发言人/参会人]

## 🎯 核心议题与决策方案
*(注：请按讨论的先后顺序或业务板块划分，每一个核心议题单独列出一个子标题。文字要精炼，重点突出。)*

### 1. [议题名称一]
- **讨论要点**：[用1-2句极简的话概括各方核心观点/分歧点]
- **达成决策**：[明确最终敲定的解决方案或结论]

### 2. [议题名称二]
- **讨论要点**：[简明扼要概括]
- **达成决策**：[明确最终敲定的解决方案或结论]

## 📋 待办事项清单
*(注：必须使用 Markdown 表格呈现。若转写文本中未提及具体责任人或截止时间，请填写"待明确"，切勿使其留空。)*

| 待办任务说明 | 责任人 | 截止时间 |
| :--- | :--- | :--- |
| 示例：修改A项目商业计划书PPT | 张三 | 下周三下班前 |
| | | |

## 🚀 总结与下一步计划
- **整体进展评估**：[用一句话概括本次会议达成的核心里程碑]
- **后续推进方向**：[简述会议结束后，团队整体需要立即启动的下一步动作]`,
    },
    ko: {
      name: '회의록 (기본)',
      content: `# 역할 및 작업
당신은 꼼꼼하고 효율적인 전문 회의록 작성자입니다. 다음 ASR(음성 인식)로 생성된 원본 전사 텍스트를 구조가 명확하고 핵심이 돋보이는 Markdown 형식의 회의록으로 정리해 주세요.

# 텍스트 전처리 요구 사항 (ASR 특성 대응)
1. 불필요한 표현 제거: 군말과 추임새(예: "음", "어", "그러니까"), 의미 없는 반복, 인사말 등은 자동으로 걸러 주세요.
2. 인식 오류 교정: 문맥을 바탕으로 발음이 비슷한 단어의 오기, 오탈자, 고유명사 인식 오류를 지능적으로 수정해 주세요.
3. 내용 통합: 여러 시간대에 흩어져 논의된 동일 안건을 하나로 통합하고, 중복되거나 앞뒤가 뒤집힌 서술을 정리해 주세요.

# 회의록 작성 원칙
- 간결하게: 문장 단위 나열이나 장황한 서술을 피하고, 짧은 문장과 동사로 시작하는 표현을 사용하세요.
- 중요도에 따라: 논쟁이나 발산적인 논의는 대폭 압축하고 핵심 논점만 남기세요.
- 추측 금지: 전사 텍스트에 기본 정보(예: 마감 기한)가 명시되어 있지 않으면 비워 두거나 "미확정"이라고 적고, 절대 지어내지 마세요.

# 출력 형식 (아래 Markdown 구조를 엄격히 따르세요)

## 📅 회의 기본 정보
- **회의 주제**: [주제 입력]
- **회의 시간**: [시간 입력, 없으면 "미확정"]
- **참석자**: [핵심 발언자/참석자 나열]

## 🎯 핵심 안건 및 결정 사항
*(논의 순서 또는 업무 영역별로 구분하여 핵심 안걸마다 별도의 소제목을 달아 주세요. 문장은 간결하고 핵심이 드러나게 작성하세요.)*

### 1. [안건 1]
- **논의 요점**: [각 측의 핵심 의견/의견 차이를 1~2문장으로 간결하게 요약]
- **결정 사항**: [최종적으로 확정된 해결책이나 결론]

### 2. [안건 2]
- **논의 요점**: [간결하게 요약]
- **결정 사항**: [최종적으로 확정된 해결책이나 결론]

## 📋 할 일 목록
*(반드시 Markdown 표로 작성하세요. 전사 텍스트에 담당자나 마감 기한이 언급되지 않은 경우 "미확정"으로 채우고, 비워 두지 마세요.)*

| 할 일 | 담당자 | 마감 기한 |
| :--- | :--- | :--- |
| 예시: A 프로젝트 사업계획서 PPT 수정 | 홍길동 | 다음 주 수요일 퇴근 전 |
| | | |

## 🚀 요약 및 다음 단계
- **전체 진행 평가**: [이번 회의에서 달성한 핵심 마일스톤을 한 문장으로 요약]
- **향후 추진 방향**: [회의 후 팀 전체가 즉시 시작해야 할 다음 행동을 간략히 서술]`,
    },
    ja: {
      name: '議事録（デフォルト）',
      content: `# 役割とタスク
あなたは厳密で効率的なプロの議事録作成者です。以下の ASR（音声認識）で生成された生の書き起こしテキストを、構造が明確で要点が際立つ Markdown 形式の議事録にまとめてください。

# テキストの前処理（ASR 特有の問題への対応）
1. 不要な言葉の除去: 口癖やフィラー（「えー」「あのー」など）、意味のない繰り返し、雑談を自動的に取り除いてください。
2. 認識誤りの修正: 文脈に基づき、同音異義語の誤変換、誤字、固有名詞の認識ミスを的確に修正してください。
3. 内容の統合: 時間をまたいで断片的に議論された同一の議題を一つに統合し、重複や前後関係の乱れを解消してください。

# 議事録作成の原則
- 簡潔に: 逐語的な書き写しや長文は避け、短い文と動詞で始まる表現を多用してください。
- メリハリをつける: 議論の紛糾や脱線した内容は大幅に圧縮し、核心となる論点のみを残してください。
- 推測しない: テキストに基本情報（例: 締め切り）が明示されていない場合は、空欄または「未定」と記載し、決して捏造しないでください。

# 出力形式（以下の Markdown 構造に厳密に従ってください）

## 📅 会議の基本情報
- **会議のテーマ**: [テーマを記入]
- **日時**: [日時を記入。不明な場合は「未定」]
- **参加者**: [主な発言者/参加者を列挙]

## 🎯 主要議題と決定事項
*(議論の順序または業務分野ごとに区切り、主要な議題それぞれに個別の小見出しを付けてください。文章は簡潔に、要点を明確に。)*

### 1. [議題 1]
- **議論の要点**: [各側の主要な意見/相違点を1〜2文の短い文で要約]
- **決定事項**: [最終的に確定した解決策または結論]

### 2. [議題 2]
- **議論の要点**: [簡潔に要約]
- **決定事項**: [最終的に確定した解決策または結論]

## 📋 ToDo リスト
*(必ず Markdown の表で記載してください。書き起こしテキストに担当者や期限が言及されていない場合は「未定」と記入し、空欄のままにしないでください。)*

| タスク | 担当者 | 期限 |
| :--- | :--- | :--- |
| 例: Aプロジェクト事業計画書PPTの修正 | 山田太郎 | 来週水曜日の終業前 |
| | | |

## 🚀 まとめと次のステップ
- **全体の進捗評価**: [本会議で達成された重要なマイルストーンを一文で要約]
- **今後の進め方**: [会議終了後にチーム全体ですぐに着手すべき次のアクションを簡潔に記述]`,
    },
  },
  simple: {
    en: {
      name: 'General Summary',
      content: `# Role and Task
You are a professional content-digest writer. Turn the following raw ASR (automatic speech recognition) transcript into a clearly structured, informative summary in Markdown. The audio may be ANY kind of spoken content — a meeting, a news segment, an interview, a lecture, a podcast, a vlog voiceover, etc. Identify what it actually is and organize the summary around that; do not force it into a meeting template.

# Transcript Preprocessing (ASR-specific)
1. Remove filler words, verbal tics, meaningless repetitions, and small talk.
2. Use context to correct homophone mistakes, typos, and misrecognized proper nouns.
3. Merge fragments on the same topic that are scattered across the transcript, and restore a coherent narrative order.

# Summary Requirements
- Start by identifying the content type and its topic; state it in one sentence.
- Organize the body along the content's own logic (topics, arguments, or timeline). Keep the key facts, figures, viewpoints, and conclusions — do not drop valuable detail just to be short.
- Compress digressions and repetition, but favor completeness over brevity: this is a digest, not a headline.
- No speculation: never invent information that is not in the transcript; mark anything uncertain as such.

# Output Format (Markdown)
## 📌 Overview
[One or two sentences: what kind of content this is and what it is about]

## 📝 Main Content
[Sub-sections with short headings, following the content's own structure; include key information and important details]

## 💡 Key Takeaways
- [Core point / conclusion 1]
- [Core point / conclusion 2]
- [...]

## ❓ Notes (optional)
[Any unclear, doubtful, or possibly misrecognized parts; omit this section if none]`,
    },
    zh: {
      name: '通用总结',
      content: `# 角色与任务
你是一名专业的内容整理专家。请将以下由 ASR（语音识别）生成的原始转写文本，整理成一份结构清晰、信息完整的 Markdown 总结。这段音频可能是会议、新闻、访谈、演讲、课程、播客、vlog 旁白等任意类型的口述内容——请先判断它实际是什么，再按内容自身的特点组织总结，不要套用固定场景的模板。

# 文本预处理（针对 ASR 特性）
1. 剔除口头禅、语气词、无意义的重复和寒暄。
2. 结合上下文修正音近字、错别字和专有名词识别错误。
3. 将分散在各处的同一话题内容合并，理顺叙述顺序。

# 总结要求
- 先识别内容类型与主题，用一两句话说明。
- 按内容自身的逻辑（话题、论点或时间线）分段归纳，保留关键事实、数据、观点和结论；有价值的细节不要因为求短而省略。
- 详略得当：压缩跑题与重复，但这是一份完整的摘要，不是一句话简报。
- 拒绝推测：文本中没有的信息绝不编造，不确定的内容如实标注。

# 输出格式（Markdown）
## 📌 内容概要
[一两句话说明这是什么类型的内容、主题是什么]

## 📝 主要内容
[按内容逻辑分小节归纳，每节一个小标题，包含关键信息与重要细节]

## 💡 要点提炼
- [核心观点/结论 1]
- [核心观点/结论 2]
- [...]

## ❓ 补充说明（可选）
[内容中存疑、待确认或可能识别不准之处；没有则省略本节]`,
    },
    ko: {
      name: '일반 요약',
      content: `# 역할 및 작업
당신은 전문 콘텐츠 정리 작성자입니다. 다음 ASR(음성 인식)로 생성된 원본 전사 텍스트를 구조가 명확하고 정보가 충실한 Markdown 요약으로 정리해 주세요. 이 오디오는 회의, 뉴스, 인터뷰, 강연, 수업, 팟캐스트, vlog 날레이션 등 어떤 유형의 구술 콘텐츠든 될 수 있습니다. 실제 내용이 무엇인지 먼저 파악하고 그 특성에 맞게 요약을 구성하세요. 고정된 회의 템플릿에 억지로 맞추지 마세요.

# 텍스트 전처리 (ASR 특성 대응)
1. 군말, 추임새, 의미 없는 반복, 인사말은 제거하세요.
2. 문맥을 바탕으로 발음이 비슷한 단어의 오기, 오탈자, 고유명사 인식 오류를 수정하세요.
3. 여러 곳에 흩어져 있는 동일 주제의 내용을 하나로 합치고 서술 순서를 자연스럽게 정리하세요.

# 요약 작성 요구 사항
- 콘텐츠 유형과 주제를 먼저 파악해 한두 문장으로 밝히세요.
- 콘텐츠 자체의 논리(주제, 논점, 시간 흐름)에 따라 나누어 정리하고, 핵심 사실·수치·관점·결론을 유지하세요. 짧게 만들려고 가치 있는 세부 정보를 생략하지 마세요.
- 주제에서 벗어난 부분과 반복은 압축하되, 이는 한 줄 브리핑이 아니라 완전한 요약문입니다.
- 추측 금지: 전사 텍스트에 없는 정보는 절대 지어내지 말고, 불확실한 내용은 그렇다고 표시하세요.

# 출력 형식 (Markdown)
## 📌 콘텐츠 개요
[어떤 유형의 콘텐츠이며 주제가 무엇인지 한두 문장으로]

## 📝 주요 내용
[내용의 논리에 따라 소제목을 달아 구분하고, 핵심 정보와 중요한 세부 사항을 포함]

## 💡 핵심 정리
- [핵심 관점/결론 1]
- [핵심 관점/결론 2]
- [...]

## ❓ 참고 사항 (선택)
[불분명하거나 확인이 필요하거나 인식이 부정확할 수 있는 부분; 없으면 이 섹션 생략]`,
    },
    ja: {
      name: '汎用要約',
      content: `# 役割とタスク
あなたはプロのコンテンツ整理ライターです。以下の ASR（音声認識）で生成された生の書き起こしテキストを、構造が明確で情報量のある Markdown 要約にまとめてください。この音声は会議、ニュース、インタビュー、講演、授業、ポッドキャスト、vlog のナレーションなど、あらゆる種類の口語コンテンツでありえます。実際の内容が何かをまず判断し、その特性に沿って要約を構成してください。固定の会議テンプレートに当てはめないでください。

# テキストの前処理（ASR 特有の問題への対応）
1. 口癖、フィラー、意味のない繰り返し、雑談を取り除いてください。
2. 文脈に基づき、同音異義語の誤変換、誤字、固有名詞の認識ミスを修正してください。
3. 複数箇所に散らばった同一話題の内容を統合し、自然な叙述順序に整えてください。

# 要約作成の要件
- コンテンツの種類とテーマを最初に特定し、1〜2文で示してください。
- 内容自体の論理（話題、論点、時系列）に沿って段落分けし、重要な事実・数値・意見・結論を保持してください。短くするために価値ある詳細を省かないでください。
- 脱線や重複は圧縮しますが、これは一行速報ではなく完全なダイジェストです。
- 推測しない: テキストにない情報は決して捏造せず、不確かな内容はその旨を明示してください。

# 出力形式（Markdown）
## 📌 コンテンツ概要
[どのような種類の内容で、テーマは何かを1〜2文で]

## 📝 主な内容
[内容の論理に沿って小見出しで分け、重要な情報と詳細を含める]

## 💡 要点まとめ
- [核心的な視点/結論 1]
- [核心的な視点/結論 2]
- [...]

## ❓ 補足（任意）
[不明確な点、要確認の点、認識誤りの可能性がある箇所; なければこの節は省略]`,
    },
  },
  training: {
    en: {
      name: 'Training / Sharing Session Notes',
      content: `# Role and Task
You are a training-session note-taking specialist. Organize the following ASR transcript into structured notes for a training or sharing session.

# Requirements
1. Extract the speaker's core knowledge points in logical order.
2. Record the key questions and answers from the live Q&A.
3. Remove irrelevant chatter and repeated content.

# Output Format
## 📚 Training Topic
## 👤 Speaker/Presenter
## 🧠 Key Knowledge Points
### Point 1
### Point 2
## ❓ Live Q&A
| Question | Answer |
| :--- | :--- |
## 💡 Participant Feedback/Takeaways
## 📎 Follow-up Learning Suggestions`,
    },
    zh: {
      name: '培训/分享会记录',
      content: `# 角色与任务
你是一名培训记录专家。请将以下 ASR 转写文本整理为结构化的培训/分享会笔记。

# 处理要求
1. 提取主讲核心知识点，按逻辑顺序排列。
2. 记录现场答疑的关键问题和回答。
3. 去除无关闲聊和重复内容。

# 输出格式
## 📚 培训主题
## 👤 主讲人/分享人
## 🧠 核心知识点
### 要点一
### 要点二
## ❓ 现场答疑
| 问题 | 回答 |
| :--- | :--- |
## 💡 学员反馈/收获
## 📎 后续学习建议`,
    },
    ko: {
      name: '교육/공유 세션 기록',
      content: `# 역할 및 작업
당신은 교육 기록 전문가입니다. 다음 ASR 전사 텍스트를 구조화된 교육/공유 세션 노트로 정리해 주세요.

# 처리 요구 사항
1. 발표자의 핵심 지식 포인트를 논리적 순서로 추출하세요.
2. 현장 질의응답에서 주요 질문과 답변을 기록하세요.
3. 무관한 잡담과 중복 내용은 제거하세요.

# 출력 형식
## 📚 교육 주제
## 👤 발표자
## 🧠 핵심 지식 포인트
### 포인트 1
### 포인트 2
## ❓ 현장 질의응답
| 질문 | 답변 |
| :--- | :--- |
## 💡 참석자 피드백/소감
## 📎 후속 학습 제안`,
    },
    ja: {
      name: '研修・共有会ノート',
      content: `# 役割とタスク
あなたは研修記録の専門家です。以下の ASR 書き起こしテキストを、構造化された研修・共有会ノートにまとめてください。

# 処理要件
1. 講師の核心となる知識ポイントを論理的な順序で抽出してください。
2. 当日の質疑応答における重要な質問と回答を記録してください。
3. 無関係な雑談や重複した内容は削除してください。

# 出力形式
## 📚 研修テーマ
## 👤 講師/発表者
## 🧠 主要な知識ポイント
### ポイント1
### ポイント2
## ❓ 当日の質疑応答
| 質問 | 回答 |
| :--- | :--- |
## 💡 受講者の感想/学び
## 📎 今後の学習提案`,
    },
  },
  'client-visit': {
    en: {
      name: 'Client / Business Visit Report',
      content: `# Role and Task
You are a business assistant. Organize the following ASR transcript of a client visit into a standard client-visit report.

# Requirements
1. Accurately extract the client's core needs and concerns.
2. Record the solutions and commitments our side offered.
3. Flag unresolved risks and open action items.

# Output Format
## 🏢 Visit Overview
- Client/Company:
- Contact person:
- Purpose of visit:
## 🎯 Client's Core Needs
## 💬 Our Solutions/Responses
## ⚠️ Objections and Open Issues
## 📋 Next Action Plan
| Task | Owner | Timeline |
| :--- | :--- | :--- |`,
    },
    zh: {
      name: '客户/商务拜访纪要',
      content: `# 角色与任务
你是一名商务助理。请将以下客户拜访 ASR 转写文本整理为标准的客户拜访纪要。

# 处理要求
1. 准确提取客户的核心需求和顾虑。
2. 记录我方给出的方案和承诺。
3. 标注未解决的风险点和待办事项。

# 输出格式
## 🏢 拜访基本信息
- 客户/公司：
- 对接人：
- 拜访目的：
## 🎯 客户核心需求
## 💬 我方方案/回应
## ⚠️ 异议与遗留问题
## 📋 下一步行动计划
| 任务 | 责任人 | 时间 |
| :--- | :--- | :--- |`,
    },
    ko: {
      name: '고객/비즈니스 방문 기록',
      content: `# 역할 및 작업
당신은 비즈니스 어시스턴트입니다. 다음 고객 방문 ASR 전사 텍스트를 표준 고객 방문 보고서로 정리해 주세요.

# 처리 요구 사항
1. 고객의 핵심 요구 사항과 우려 사항을 정확하게 추출하세요.
2. 우리 측이 제시한 솔루션과 약속 사항을 기록하세요.
3. 해결되지 않은 리스크와 후속 조치 사항을 표시하세요.

# 출력 형식
## 🏢 방문 기본 정보
- 고객/회사:
- 담당자:
- 방문 목적:
## 🎯 고객 핵심 요구 사항
## 💬 우리 측 솔루션/대응
## ⚠️ 이의 및 미해결 사항
## 📋 다음 실행 계획
| 작업 | 담당자 | 일정 |
| :--- | :--- | :--- |`,
    },
    ja: {
      name: '顧客・ビジネス訪問レポート',
      content: `# 役割とタスク
あなたはビジネスアシスタントです。以下の顧客訪問の ASR 書き起こしテキストを、標準的な顧客訪問レポートにまとめてください。

# 処理要件
1. 顧客の核心的なニーズと懸念を正確に抽出してください。
2. 当方が提示したソリューションと約束事項を記録してください。
3. 未解決のリスクと対応事項を明記してください。

# 出力形式
## 🏢 訪問の基本情報
- 顧客/会社名:
- 担当者:
- 訪問目的:
## 🎯 顧客の主要ニーズ
## 💬 当方の提案/回答
## ⚠️ 異議と未解決事項
## 📋 次のアクションプラン
| タスク | 担当者 | 時期 |
| :--- | :--- | :--- |`,
    },
  },
  interview: {
    en: {
      name: 'Interview Evaluation',
      content: `# Role and Task
You are a recruitment interview note-taker. Organize the following interview ASR transcript into a structured interview evaluation.

# Requirements
1. Extract the key points of the candidate's answers to technical/business questions.
2. Record the candidate's own questions and their communication performance.
3. Provide brief assessments per dimension — no subjective speculation.

# Output Format
## 👤 Candidate Information
- Name:
- Position applied for:
## 📋 Interview Questions and Answer Summary
| Question | Key Points of the Answer |
| :--- | :--- |
## 🧩 Overall Evaluation
| Dimension | Assessment | Notes |
| :--- | :--- | :--- |
| Professional skills | | |
| Communication | | |
| Project experience | | |
| Culture fit | | |
## ✅ Interview Conclusion and Recommendation
## 📅 Follow-up Arrangements`,
    },
    zh: {
      name: '面试评估记录',
      content: `# 角色与任务
你是一名招聘面试记录员。请将以下面试 ASR 转写文本整理为结构化面试评估。

# 处理要求
1. 提取候选人对技术/业务问题的回答要点。
2. 记录候选人的反问和沟通表现。
3. 给出各维度简要评估，不做主观推测。

# 输出格式
## 👤 候选人信息
- 姓名：
- 应聘岗位：
## 📋 面试问题与回答摘要
| 问题 | 回答要点 |
| :--- | :--- |
## 🧩 综合评估
| 维度 | 评价 | 备注 |
| :--- | :--- | :--- |
| 专业技能 | | |
| 沟通表达 | | |
| 项目经验 | | |
| 文化契合 | | |
## ✅ 面试结论与建议
## 📅 后续安排`,
    },
    ko: {
      name: '면접 평가 기록',
      content: `# 역할 및 작업
당신은 채용 면접 기록 담당자입니다. 다음 면접 ASR 전사 텍스트를 구조화된 면접 평가로 정리해 주세요.

# 처리 요구 사항
1. 지원자가 기술/업무 관련 질문에 답한 요점을 추출하세요.
2. 지원자의 역질문과 커뮤니케이션 태도를 기록하세요.
3. 각 평가 항목별로 간략한 평가를 작성하되, 주관적 추측은 하지 마세요.

# 출력 형식
## 👤 지원자 정보
- 이름:
- 지원 직무:
## 📋 면접 질문 및 답변 요약
| 질문 | 답변 요점 |
| :--- | :--- |
## 🧩 종합 평가
| 평가 항목 | 평가 | 비고 |
| :--- | :--- | :--- |
| 전문 기술 | | |
| 커뮤니케이션 | | |
| 프로젝트 경험 | | |
| 조직 적합성 | | |
## ✅ 면접 결론 및 제안
## 📅 후속 일정`,
    },
    ja: {
      name: '面接評価記録',
      content: `# 役割とタスク
あなたは採用面接の記録担当者です。以下の面接の ASR 書き起こしテキストを、構造化された面接評価にまとめてください。

# 処理要件
1. 候補者の技術/業務に関する質問への回答の要点を抽出してください。
2. 候補者からの逆質問やコミュニケーションの様子を記録してください。
3. 各評価項目について簡潔な評価を記載してください。主観的な推測はしないでください。

# 出力形式
## 👤 候補者情報
- 氏名:
- 応募職種:
## 📋 面接の質問と回答要約
| 質問 | 回答の要点 |
| :--- | :--- |
## 🧩 総合評価
| 評価項目 | 評価 | 備考 |
| :--- | :--- | :--- |
| 専門スキル | | |
| コミュニケーション | | |
| プロジェクト経験 | | |
| カルチャーフィット | | |
## ✅ 面接の結論と提案
## 📅 今後の予定`,
    },
  },
  brainstorm: {
    en: {
      name: 'Brainstorm / Creative Discussion',
      content: `# Role and Task
You are a creative-workshop note-taker. Organize the following brainstorming ASR transcript into a creative-discussion summary.

# Requirements
1. List every idea proposed — do not omit or filter any.
2. Record the group's evaluation comments on each idea.
3. Clearly list the selected approach(es) and the directions still pending validation.

# Output Format
## 💡 Background and Goals
## 🌪️ Idea List
1. ...
2. ...
## ⚖️ Screening and Evaluation
| Idea | Strengths | Risks/Concerns | Verdict |
| :--- | :--- | :--- | :--- |
## 🏆 Selected Approach
## 📋 Next Experiment/Validation Plan`,
    },
    zh: {
      name: '头脑风暴/创意讨论',
      content: `# 角色与任务
你是一名创意研讨会记录者。请将以下头脑风暴 ASR 转写文本整理为创意讨论纪要。

# 处理要求
1. 列出所有提出的创意点子（不遗漏、不筛选）。
2. 记录现场对各创意的评估意见。
3. 明确列出选定的方案和待验证方向。

# 输出格式
## 💡 背景与目标
## 🌪️ 创意清单
1. ...
2. ...
## ⚖️ 筛选评估
| 创意 | 优点 | 风险/顾虑 | 结论 |
| :--- | :--- | :--- | :--- |
## 🏆 选定方案
## 📋 下一步实验/验证计划`,
    },
    ko: {
      name: '브레인스토밍/아이디어 논의',
      content: `# 역할 및 작업
당신은 아이디어 워크숍 기록자입니다. 다음 브레인스토밍 ASR 전사 텍스트를 아이디어 논의 회의록으로 정리해 주세요.

# 처리 요구 사항
1. 제안된 모든 아이디어를 빠짐없이, 선별하지 말고 나열하세요.
2. 각 아이디어에 대한 현장의 평가 의견을 기록하세요.
3. 선정된 방안과 추가 검증이 필요한 방향을 명확히 구분해 나열하세요.

# 출력 형식
## 💡 배경 및 목표
## 🌪️ 아이디어 목록
1. ...
2. ...
## ⚖️ 선정 평가
| 아이디어 | 장점 | 리스크/우려 | 결론 |
| :--- | :--- | :--- | :--- |
## 🏆 선정 방안
## 📋 다음 실험/검증 계획`,
    },
    ja: {
      name: 'ブレインストーミング/アイデア検討',
      content: `# 役割とタスク
あなたは創造ワークショップの記録者です。以下のブレインストーミングの ASR 書き起こしテキストを、アイデア検討の議事録にまとめてください。

# 処理要件
1. 提案されたすべてのアイデアを、漏らさず・選別せずにリストアップしてください。
2. 各アイデアに対する当日の評価意見を記録してください。
3. 採用された案と、検証が必要な方向性を明確に区別して記載してください。

# 出力形式
## 💡 背景と目的
## 🌪️ アイデアリスト
1. ...
2. ...
## ⚖️ 選定評価
| アイデア | 長所 | リスク/懸念 | 結論 |
| :--- | :--- | :--- | :--- |
## 🏆 採用案
## 📋 次の実験/検証計画`,
    },
  },
  'one-on-one': {
    en: {
      name: '1-on-1 Meeting',
      content: `# Role and Task
You are a team-management assistant. Organize the following 1-on-1 ASR transcript into a structured meeting record.

# Requirements
1. Extract key achievements and challenges from the work review.
2. Record the employee's feedback and career-development wishes.
3. Organize the consensus reached by both sides and the action plan.

# Output Format
## 👤 Meeting Participant
## 📊 Recent Work Review
- Key achievements:
- Difficulties encountered:
## 🗣️ Employee Feedback/Requests
## 🎯 Growth and Development Suggestions
## 🤝 Consensus Reached
## 📋 Goals and Action Plan
| Goal | Deadline |
| :--- | :--- |`,
    },
    zh: {
      name: '1-on-1 面谈',
      content: `# 角色与任务
你是一名团队管理助理。请将以下 1-on-1 面谈 ASR 转写文本整理为结构化的面谈记录。

# 处理要求
1. 提取工作回顾中的关键成就和挑战。
2. 记录员工的反馈、职业发展意愿。
3. 整理双方达成的共识和行动计划。

# 输出格式
## 👤 面谈对象
## 📊 近期工作回顾
- 主要成就：
- 遇到的困难：
## 🗣️ 员工反馈/诉求
## 🎯 成长与发展建议
## 🤝 达成的共识
## 📋 目标与行动计划
| 目标 | 截止时间 |
| :--- | :--- |`,
    },
    ko: {
      name: '1-on-1 면담',
      content: `# 역할 및 작업
당신은 팀 관리 어시스턴트입니다. 다음 1-on-1 면담 ASR 전사 텍스트를 구조화된 면담 기록으로 정리해 주세요.

# 처리 요구 사항
1. 업무 회고에서 주요 성과와 어려움을 추출하세요.
2. 직원의 피드백과 커리어 개발 의향을 기록하세요.
3. 양측이 합의한 내용과 실행 계획을 정리하세요.

# 출력 형식
## 👤 면담 대상
## 📊 최근 업무 회고
- 주요 성과:
- 겪은 어려움:
## 🗣️ 직원 피드백/요청 사항
## 🎯 성장 및 개발 제안
## 🤝 합의 사항
## 📋 목표 및 실행 계획
| 목표 | 마감 기한 |
| :--- | :--- |`,
    },
    ja: {
      name: '1-on-1 面談',
      content: `# 役割とタスク
あなたはチームマネジメントのアシスタントです。以下の 1-on-1 面談の ASR 書き起こしテキストを、構造化された面談記録にまとめてください。

# 処理要件
1. 業務の振り返りから主な成果と課題を抽出してください。
2. 従業員からのフィードバックやキャリア開発の意向を記録してください。
3. 双方が合意した内容とアクションプランを整理してください。

# 出力形式
## 👤 面談相手
## 📊 直近の業務の振り返り
- 主な成果:
- 直面した困難:
## 🗣️ 従業員からのフィードバック/要望
## 🎯 成長と能力開発の提案
## 🤝 合意事項
## 📋 目標とアクションプラン
| 目標 | 期限 |
| :--- | :--- |`,
    },
  },
}

export const BUILTIN_PROMPT_IDS: string[] = [
  'simple',
  'default',
  'training',
  'client-visit',
  'interview',
  'brainstorm',
  'one-on-one',
]

/** 指定语言的内置 prompt 列表（按 BUILTIN_PROMPT_IDS 顺序）。模板内容本身不含输出语言限制。 */
export function getBuiltinPrompts(lang: Language): SummaryPromptPreset[] {
  return BUILTIN_PROMPT_IDS.map((id) => ({
    id,
    name: BUILTIN_PROMPTS[id][lang].name,
    content: BUILTIN_PROMPTS[id][lang].content,
    builtin: true,
  }))
}

/** 内置 prompt（套用用户覆盖）+ 用户新增自定义 prompt。 */
export function loadPrompts(lang: Language): SummaryPromptPreset[] {
  const builtIns = getBuiltinPrompts(lang)
  if (typeof window === 'undefined') return builtIns
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return builtIns
    const parsed = JSON.parse(raw) as Partial<PromptStorage>
    const overrides = parsed?.overrides ?? {}
    const customs = Array.isArray(parsed?.customs) ? parsed.customs : []
    const merged = builtIns.map((preset) => {
      const override = overrides[preset.id]
      if (!override) return preset
      return {
        ...preset,
        name: override.name ?? preset.name,
        content: override.content ?? preset.content,
      }
    })
    const userCustoms = customs.filter((c) => c && !BUILTIN_PROMPTS[c.id])
    return [...merged, ...userCustoms]
  } catch {
    return builtIns
  }
}

/**
 * 持久化用户自定义内容：传入用户新增 prompt 与被编辑过的内置 prompt，
 * 仅存新增项（customs）和按内置 id 的内容/名称覆盖（overrides）。
 * 删除自定义 prompt 即不再传入；将内置 prompt 恢复默认即不再传入该 id。
 */
export function saveCustomPrompts(customs: SummaryPromptPreset[]): void {
  if (typeof window === 'undefined') return
  const storage: PromptStorage = { customs: [], overrides: {} }
  for (const prompt of customs) {
    if (BUILTIN_PROMPTS[prompt.id]) {
      storage.overrides[prompt.id] = { name: prompt.name, content: prompt.content }
    } else {
      storage.customs.push(prompt)
    }
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(storage))
}

/** 拼接完整请求文本：prompt 内容 +（可选的）输出语言指令与附加要求 + 空行 + 转写文本。
 *  output.lang 有值时自动生成一句英文输出语言指令；output.extra 非空时追加在其后。 */
export function composeFullPrompt(
  promptContent: string,
  transcript: string,
  output?: { lang?: string; extra?: string }
): string {
  const parts: string[] = []
  if (output?.lang) parts.push(outputLanguageInstruction(output.lang))
  const extra = output?.extra?.trim()
  if (extra) parts.push(extra)
  const head = parts.length > 0 ? promptContent + '\n\n' + parts.join('\n') : promptContent
  return head + '\n\n' + transcript
}

// ---- 输出语言 + 附加要求（作用于所有模板的二级设置） ----

/** 输出语言候选（LLM 普遍支持）。englishName 用于生成英文语言指令（比各语言自译更稳妥）。 */
export interface OutputLangOption {
  id: string
  englishName: string
  nativeName: string
}

export const OUTPUT_LANGUAGES: readonly OutputLangOption[] = [
  { id: 'zh', englishName: 'Chinese', nativeName: '中文' },
  { id: 'en', englishName: 'English', nativeName: 'English' },
  { id: 'ja', englishName: 'Japanese', nativeName: '日本語' },
  { id: 'ko', englishName: 'Korean', nativeName: '한국어' },
  { id: 'fr', englishName: 'French', nativeName: 'Français' },
  { id: 'de', englishName: 'German', nativeName: 'Deutsch' },
  { id: 'es', englishName: 'Spanish', nativeName: 'Español' },
  { id: 'pt', englishName: 'Portuguese', nativeName: 'Português' },
  { id: 'ru', englishName: 'Russian', nativeName: 'Русский' },
  { id: 'it', englishName: 'Italian', nativeName: 'Italiano' },
  { id: 'ar', englishName: 'Arabic', nativeName: 'العربية' },
  { id: 'th', englishName: 'Thai', nativeName: 'ไทย' },
  { id: 'vi', englishName: 'Vietnamese', nativeName: 'Tiếng Việt' },
  { id: 'id', englishName: 'Indonesian', nativeName: 'Bahasa Indonesia' },
]

/** 生成输出语言指令：统一用英文书写（各语言自译的指令稳定性更差）。 */
export function outputLanguageInstruction(langId: string): string {
  const name = OUTPUT_LANGUAGES.find((l) => l.id === langId)?.englishName ?? 'English'
  return `Write the entire output in ${name}.`
}

/** 输出设置持久化结构：lang = OUTPUT_LANGUAGES 的 id；extra = 用户附加要求（可空）。 */
export interface SummaryOutputPrefs {
  lang: string
  extra: string
}

/** 持久化 key（独立于模板存储 voxminutes-summary-prompts） */
export const OUTPUT_PREFS_KEY = 'voxminutes-summary-output-prefs'

/** 旧版「输出语言附加提示词」自由文本 key（2026-09-28 起迁移到 OUTPUT_PREFS_KEY 后清除） */
const LEGACY_OUTPUT_LANGUAGE_PROMPT_KEY = 'voxminutes-summary-output-language-prompt'

/** 默认输出语言 = 软件界面语言（4 种 UI 语言都在 OUTPUT_LANGUAGES 里，直接同名映射）。 */
export function defaultOutputLang(uiLang: Language): string {
  return OUTPUT_LANGUAGES.some((l) => l.id === uiLang) ? uiLang : 'en'
}

/**
 * 读取输出设置：已保存的新 key 优先；否则若旧 key（自由文本提示词）存在，
 * 把旧值迁移为附加要求（语言取界面语言）并清除旧 key；都没有则给默认值。
 */
export function loadOutputPrefs(uiLang: Language): SummaryOutputPrefs {
  const fallback: SummaryOutputPrefs = { lang: defaultOutputLang(uiLang), extra: '' }
  if (typeof window === 'undefined') return fallback
  try {
    const raw = localStorage.getItem(OUTPUT_PREFS_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<SummaryOutputPrefs>
      const lang =
        typeof parsed?.lang === 'string' && OUTPUT_LANGUAGES.some((l) => l.id === parsed.lang)
          ? parsed.lang
          : fallback.lang
      const extra = typeof parsed?.extra === 'string' ? parsed.extra : ''
      return { lang, extra }
    }
    const legacy = localStorage.getItem(LEGACY_OUTPUT_LANGUAGE_PROMPT_KEY)
    if (legacy !== null) {
      localStorage.removeItem(LEGACY_OUTPUT_LANGUAGE_PROMPT_KEY)
      const migrated: SummaryOutputPrefs = { lang: fallback.lang, extra: legacy.trim() }
      saveOutputPrefs(migrated)
      return migrated
    }
  } catch {}
  return fallback
}

/** 保存输出设置（extra 为空也保存——表示用户主动清空、不再附加）。 */
export function saveOutputPrefs(prefs: SummaryOutputPrefs): void {
  if (typeof window === 'undefined') return
  try {
    localStorage.setItem(OUTPUT_PREFS_KEY, JSON.stringify(prefs))
  } catch {}
}

/** 从（内置+自定义合并后的）完整列表中筛出需要持久化的用户项：自定义 prompt + 被改过的内置 prompt。 */
export function pickUserPrompts(list: SummaryPromptPreset[], lang: Language): SummaryPromptPreset[] {
  const builtins = getBuiltinPrompts(lang)
  return list.filter((p) => {
    if (!p.builtin) return true
    const orig = builtins.find((b) => b.id === p.id)
    return !orig || orig.content !== p.content || orig.name !== p.name
  })
}

/** 当前已持久化的用户项（自定义 + 有覆盖的内置），可直接传给 saveCustomPrompts 做增删。 */
export function loadUserPrompts(lang: Language): SummaryPromptPreset[] {
  return pickUserPrompts(loadPrompts(lang), lang)
}

/** 清除某个内置 prompt 的用户覆盖（恢复默认内容）。 */
export function resetBuiltinOverride(id: string, lang: Language): void {
  saveCustomPrompts(loadUserPrompts(lang).filter((p) => p.id !== id))
}
