// ttsVoices.ts
//
// 远程 TTS 的音色表与「语言感知默认音色」。
//
// MiMo-V2.5-TTS 官方音色（2026-09-22 依据官方文档整理，共 9 个 `audio.voice` 取值；
// 中文音色 id 就是中文字符串、英文音色 id 大小写敏感）：
//   中文：冰糖 / 茉莉 / 苏打 / 白桦 + 别名 mimo_default（按集群自动映射到冰糖或 Mia）
//   英文：Mia / Chloe / Milo / Dean
// 官方未提供 speed / pitch / emotion 等数值参数——风格、情感、语速、方言、角色扮演
// 都写在 user 消息（自然语言指令）或正文里的音频标签（如 (唱歌)、[停顿]）中。

/** Supertonic 3 的 10 个音色（sid 0-9；0-4 女声、5-9 男声）。
 *
 *  ⚠️ 这是**回退清单**：正常应使用网关随 `/v1/models` 下发的 `voices` 字段
 *  （见 gateway/src/db.ts 的 supertonic 种子行）。回退存在的理由：客户端可能已缓存了
 *  升级前不含 voices 的模型目录；也防止网关漏配导致音色区一片空白。
 *  sid 与语言无关——发音由请求里的 language 决定，sid 只决定音色。
 *  实测依据：越界 sid（10/50/-1）**不报错**，会静默产出一段内容错误的音频，
 *  所以网关侧另有校验（见 speech.ts），这里只负责列出合法值。 */
export const SUPERTONIC_FALLBACK_VOICES: Array<{ id: string; lang?: string; gender?: string }> =
  Array.from({ length: 10 }, (_, i) => ({
    id: String(i),
    lang: '*',
    gender: i <= 4 ? 'female' : 'male',
  }))

/** 各供应商的可用音色（与语音合成页的音色卡片/建议保持一致）。 */
export const PROVIDER_VOICES: Record<string, string[]> = {
  qwen: ['longanhuan_v3.6'],
  mimo: ['mimo_default', '冰糖', '茉莉', '苏打', '白桦', 'Mia', 'Chloe', 'Milo', 'Dean'],
}

export interface TtsVoiceInfo {
  /** 传给上游的 `audio.voice` 取值 */
  id: string
  /** 语言 */
  lang: 'zh' | 'en'
  gender: 'female' | 'male'
  /** 风格描述（中文 / 英文各一份，按界面语言选） */
  styleZh: string
  styleEn: string
}

/** MiMo-V2.5-TTS 官方预置音色。 */
export const MIMO_TTS_VOICES: TtsVoiceInfo[] = [
  { id: '冰糖', lang: 'zh', gender: 'female', styleZh: '活泼少女', styleEn: 'Lively girl' },
  { id: '茉莉', lang: 'zh', gender: 'female', styleZh: '知性女声', styleEn: 'Intellectual female' },
  { id: '苏打', lang: 'zh', gender: 'male', styleZh: '阳光少年', styleEn: 'Sunny boy' },
  { id: '白桦', lang: 'zh', gender: 'male', styleZh: '成熟男声', styleEn: 'Mature male' },
  { id: 'Mia', lang: 'en', gender: 'female', styleZh: '灵动女声', styleEn: 'Lively girl' },
  { id: 'Chloe', lang: 'en', gender: 'female', styleZh: '甜美梦幻', styleEn: 'Sweet dreamy' },
  { id: 'Milo', lang: 'en', gender: 'male', styleZh: '阳光少年', styleEn: 'Sunny boy' },
  { id: 'Dean', lang: 'en', gender: 'male', styleZh: '沉稳温和', styleEn: 'Steady gentle' },
  { id: 'mimo_default', lang: 'zh', gender: 'female', styleZh: '默认（按集群自动映射）', styleEn: 'Default (auto-mapped)' },
]

/**
 * 按文本语言挑默认音色。
 * @param ownedBy  网关模型的 owned_by（供应商 id，如 mimo / qwen）
 * @param isEnglish 文本是否为英文
 * @returns 音色名；没有更合适的就返回 undefined（走供应商默认）
 */
export function defaultVoiceForText(ownedBy: string | undefined, isEnglish: boolean): string | undefined {
  if (ownedBy === 'mimo') {
    // mimo 的默认音色是中文音色；读英文用原生英文音色 Mia
    return isEnglish ? 'Mia' : undefined
  }
  // qwen 的 longanhuan_v3.6 中英均可，无需切换
  return undefined
}

/** 粗略判断文本是否为英文（非空白字符里几乎没有 CJK 即视为英文）。 */
export function isEnglishText(text: string): boolean {
  const chars = [...text.replace(/\s/g, '')]
  if (chars.length === 0) return false
  const cjk = chars.filter((c) => {
    const u = c.codePointAt(0) ?? 0
    return (u >= 0x4e00 && u <= 0x9fff) || (u >= 0x3400 && u <= 0x4dbf)
  }).length
  return cjk * 10 <= chars.length * 3
}
