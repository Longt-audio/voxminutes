// A/B：旧提示词（夹带前文）vs 新提示词（<source> 标签、不带前文）
// 目的：验证「译文把前 1~2 段也翻出来」的重复是否消失。
// 用用户实测的真实句子（单位 1~5），逐句翻译并检查是否混入前几段内容。
const BASE = 'https://api.voxmin.top/v1';

const UNITS = [
  'Distinguished group of people that have traveled from around the world to be a part of the important conversation.',
  "We're grateful to you. I also want thank members of the president's team and cabinet that are here today, our FBI director, Cash Patel, who's here with us.",
  'Thank you. Our secretary of education, Linda McMahon, has joined us as well.',
  "You'll hear shortly from Steven Miller, the president's deputy chief of staff and one of his top advisers on homeland security issues as well.",
  'And, of course, our secretary of treasury, Scott Besson, is here.',
];

const OLD_INSTR = `将以下English语音转录文本翻译为Chinese。
要求：
1. 只输出Chinese译文，严禁输出原文、双语对照、原文片段或重复原文；
2. 直接开始翻译，不要写“翻译：”“Chinese：”等任何前缀；
3. 修正识别错误和同音词；
4. 省略语气词；
5. 输出流畅自然的口语翻译；
6. 不要解释，不要备注。`;

const NEW_INSTR = `把 <source> 标签内的English翻译成Chinese。
硬性要求：
1. 只输出 <source> 内容的Chinese译文，不得输出原文、双语对照、标签本身、前文或任何说明文字；
2. 不要写「译文：」「Chinese：」「来源：」等任何前缀；
3. 修正语音识别错误与同音词，省略语气词，输出流畅自然的口语翻译；
4. 不要解释、不要备注、不要复述前文。`;

function oldPrompt(text, prev2) {
  const ctx = prev2.filter(Boolean);
  const ctxBlock = ctx.length ? `前文（仅供理解上下文，不要翻译）：${ctx.join(' ')}\n\n` : '';
  return `${OLD_INSTR}\n\n${ctxBlock}Source: ${text}\n\nTarget (Chinese):`;
}
function newPrompt(text) {
  return `${NEW_INSTR}\n\n<source>\n${text}\n</source>`;
}

// 标记词：越靠后的段落里出现这些词，说明混入了前面的内容
const LEAK_MARKERS = {
  1: ['杰出人士', '齐聚一堂'],
  2: ['联邦调查局', '卡什', '帕特尔', '内阁'],
  3: ['教育部长', '麦克马洪'],
  4: ['米勒', '办公厅'],
};

const reg = await fetch(`${BASE}/register`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ device_id: 'prompt-ab-' + Math.random().toString(36).slice(2, 10) }) });
const { api_key } = await reg.json();

async function translate(prompt) {
  const r = await fetch(`${BASE}/chat/completions`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${api_key}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: process.env.AB_MODEL || 'qwen-flash', feature: 'translate', messages: [{ role: 'user', content: prompt }], stream: false, temperature: 0.3, max_tokens: 512 }),
    signal: AbortSignal.timeout(120000),
  });
  const j = await r.json().catch(() => ({}));
  return j?.choices?.[0]?.message?.content ?? JSON.stringify(j).slice(0, 200);
}

for (const [name, make] of [['旧（夹带前文）', oldPrompt], ['新（<source>，无前文）', newPrompt]]) {
  console.log(`\n===== ${name} =====`);
  const outs = [];
  for (let i = 0; i < UNITS.length; i++) {
    // ⚠️ 关键：客户端传的「前文」是前 2 段的**英文原文**（flow.rs 的 unit_history 存的是
    // 已闭合单元原文），不是译文。第一次 A/B 我传成译文，所以没复现。
    const prev2 = UNITS.slice(Math.max(0, i - 2), i);
    const prompt = name.startsWith('旧') ? make(UNITS[i], prev2) : make(UNITS[i]);
    const out = await translate(prompt);
    outs.push(out);
    const leaked = [];
    for (let u = 1; u < i + 1; u++) {
      for (const m of LEAK_MARKERS[u] || []) if (out.includes(m)) leaked.push(m);
    }
    console.log(`\n[单位 ${i + 1}] 长度=${out.length}${leaked.length ? `  ⚠️ 混入前文标记: ${[...new Set(leaked)].join('/')}` : '  ✅ 无前文混入'}`);
    console.log('  ' + out.replace(/\n/g, ' ').slice(0, 220));
  }
  const withLeak = outs.filter((o, i) => i > 0 && Object.entries(LEAK_MARKERS).some(([u, ms]) => Number(u) <= i && ms.some((m) => o.includes(m)))).length;
  console.log(`\n>>> ${name}：${outs.length} 段中有 ${withLeak} 段混入了前文内容`);
}
