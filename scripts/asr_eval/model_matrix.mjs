// 翻译模型 × 语言方向 矩阵测试
// 目标：验证「新提示词（不夹带前文）」下，所有可用翻译模型都不会把前几段内容翻进本段译文。
// 同时用旧提示词跑一遍 en→zh 作为对照（证明问题真实存在、且已消除）。
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';

const BASE = 'https://api.voxmin.top/v1';
const KEY_PATH = '/tmp/asr_cmp/key.txt';

/** 取测试授权码：有缓存就用缓存，否则自助注册一个（送 200 积分，够跑完整矩阵）。 */
async function getKey() {
  if (existsSync(KEY_PATH)) {
    const k = readFileSync(KEY_PATH, 'utf8').trim();
    if (k) return k;
  }
  const r = await fetch(`${BASE}/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ device_id: 'model-matrix-' + Math.random().toString(36).slice(2, 10) }),
  });
  const j = await r.json();
  if (!j.api_key) throw new Error('注册测试授权码失败: ' + JSON.stringify(j).slice(0, 200));
  mkdirSync('/tmp/asr_cmp', { recursive: true });
  writeFileSync(KEY_PATH, j.api_key);
  console.log(`（已注册测试授权码 ${j.api_key}，剩余积分 ${j.credits}）`);
  return j.api_key;
}

const KEY = await getKey();

const CHAT_PREFIX = '<\uFF5Chy_begin\u2581of\u2581sentence\uFF5C><\uFF5Chy_User\uFF5C>';
const CHAT_SUFFIX = '<\uFF5Chy_Assistant\uFF5C>';
const LANG_EN = { zh: 'Chinese', en: 'English', ja: 'Japanese' };

const OLD_INSTR = `将以下{SRC}语音转录文本翻译为{TGT}。\n要求：\n1. 只输出{TGT}译文，严禁输出原文、双语对照、原文片段或重复原文；\n2. 直接开始翻译，不要写“翻译：”“{TGT}：”等任何前缀；\n3. 修正识别错误和同音词；\n4. 省略语气词；\n5. 输出流畅自然的口语翻译；\n6. 不要解释，不要备注。`;
const NEW_INSTR = `把 <source> 标签内的{SRC}翻译成{TGT}。\n硬性要求：\n1. 只输出 <source> 内容的{TGT}译文，不得输出原文、双语对照、标签本身、前文或任何说明文字；\n2. 不要写「译文：」「{TGT}：」「来源：」等任何前缀；\n3. 修正语音识别错误与同音词，省略语气词，输出流畅自然的口语翻译；\n4. 不要解释、不要备注、不要复述前文。`;

function buildPrompt(text, src, tgt, useOld, prevUnits) {
  const instr = (useOld ? OLD_INSTR : NEW_INSTR).replaceAll('{SRC}', LANG_EN[src]).replaceAll('{TGT}', LANG_EN[tgt]);
  const body = useOld
    ? `${instr}\n\n${prevUnits.length ? `前文（仅供理解上下文，不要翻译）：${prevUnits.join(' ')}\n\n` : ''}Source: ${text}\n\nTarget (${LANG_EN[tgt]}):`
    : `${instr}\n\n<source>\n${text}\n</source>`;
  return CHAT_PREFIX + body + CHAT_SUFFIX;
}

// ── 测试语料：3 段（第 2、3 段用来检测是否混入前文） ──────────────────────────
const CASES = {
  'en->zh': {
    src: 'en', tgt: 'zh',
    units: [
      'Distinguished group of people that have traveled from around the world to be a part of the important conversation.',
      "We're grateful to you. I also want thank members of the president's team and cabinet that are here today, our FBI director, Cash Patel, who's here with us.",
      'Thank you. Our secretary of education, Linda McMahon, has joined us as well.',
    ],
    // 越靠后的段落输出里出现这些词 = 混入了前面的内容
    markers: { 1: ['杰出人士', '齐聚一堂'], 2: ['联邦调查局', '帕特尔', '内阁', 'FBI'] },
  },
  'zh->en': {
    src: 'zh', tgt: 'en',
    units: [
      '来自世界各地的杰出人士齐聚一堂，参与这场重要的对话。',
      '我们非常感谢你们，也要感谢今天到场的总统团队和内阁成员，以及联邦调查局局长卡什·帕特尔。',
      '谢谢。我们的教育部长琳达·麦克马洪也加入了我们。',
    ],
    markers: { 1: ['distinguished', 'gathered', 'important conversation'], 2: ['FBI', 'Patel', 'cabinet', 'president'] },
  },
  'en->ja': {
    src: 'en', tgt: 'ja',
    units: [
      'Distinguished group of people that have traveled from around the world to be a part of the important conversation.',
      "We're grateful to you. I also want thank members of the president's team and cabinet that are here today, our FBI director, Cash Patel, who's here with us.",
      'Thank you. Our secretary of education, Linda McMahon, has joined us as well.',
    ],
    markers: { 1: ['世界中', '対話'], 2: ['FBI', 'パテル', '内閣'] },
  },
};

const ECHO_BAD = ['来源：', 'Source:', 'Translation:', '译文：', '<source>', '</source>', '前文', 'Context:'];

async function translate(model, prompt, src, tgt, rawText) {
  const r = await fetch(`${BASE}/chat/completions`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${KEY}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model, feature: 'translate', messages: [{ role: 'user', content: prompt }],
      source_text: rawText, source_lang: src, target_lang: tgt,
      stream: false, temperature: 0.3, max_tokens: 512,
    }),
    signal: AbortSignal.timeout(120000),
  });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) return { err: `HTTP ${r.status} ${JSON.stringify(j).slice(0, 100)}` };
  return { text: (j?.choices?.[0]?.message?.content ?? '').trim() };
}

async function runCase(model, caseName, { useOld }) {
  const c = CASES[caseName];
  const outs = [];
  const issues = [];
  for (let i = 0; i < c.units.length; i++) {
    const prev = useOld ? c.units.slice(Math.max(0, i - 2), i) : [];
    const prompt = buildPrompt(c.units[i], c.src, c.tgt, useOld, prev);
    const res = await translate(model, prompt, c.src, c.tgt, c.units[i]);
    if (res.err) { issues.push(`单位${i + 1} 请求失败: ${res.err}`); outs.push(''); continue; }
    const out = res.text;
    outs.push(out);
    const leaked = [];
    for (let u = 1; u <= i; u++) for (const m of (c.markers[u] || [])) if (out.includes(m)) leaked.push(m);
    if (leaked.length) issues.push(`单位${i + 1} 混入前文: ${[...new Set(leaked)].join('/')}`);
    const echoed = ECHO_BAD.filter((e) => out.includes(e));
    if (echoed.length) issues.push(`单位${i + 1} 回声: ${echoed.join('/')}`);
    if (!out) issues.push(`单位${i + 1} 空输出`);
  }
  return { outs, issues };
}

const MODELS = ['qwen-flash', 'qwen-plus', 'qwen-max', 'deepseek-chat', 'doubao-mt', 'mimo-v2.5'];
const rows = [];
for (const model of MODELS) {
  for (const caseName of Object.keys(CASES)) {
    for (const useOld of caseName === 'en->zh' ? [true, false] : [false]) {
      const { outs, issues } = await runCase(model, caseName, { useOld });
      const label = `${model} | ${caseName} | ${useOld ? '旧提示词' : '新提示词'}`;
      const lens = outs.map((o) => o.length).join('/');
      rows.push({ label, lens, issues });
      console.log(`${issues.length ? '❌' : '✅'} ${label.padEnd(38)} 长度=${lens.padEnd(12)} ${issues.length ? issues.join(' ; ') : ''}`);
    }
  }
}

console.log('\n===== 汇总 =====');
const bad = rows.filter((r) => r.issues.length);
console.log(`共 ${rows.length} 组，问题组 ${bad.length}`);
for (const b of bad) console.log(`  ❌ ${b.label}: ${b.issues.join(' ; ')}`);
// 新提示词单独统计
const newRows = rows.filter((r) => r.label.includes('新提示词'));
console.log(`新提示词：${newRows.length} 组，问题 ${newRows.filter((r) => r.issues.length).length} 组`);
