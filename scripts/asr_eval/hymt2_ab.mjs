// 本地 Hy-MT2 引擎的提示词 A/B（走 llama-helper 的 stdin/stdout JSON 协议）
// 目的：确认新的 <source> 提示词不会让本地小模型出现「混入前文 / 回显标签」问题。
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';

const HELPER = '/Users/longteng/vox/voxminutes/frontend/src-tauri/binaries/llama-helper-aarch64-apple-darwin';
const MODEL = '/Users/longteng/Library/Application Support/com.voxminutes.app/models/hy-mt2-1.8b/Hy-MT2-1.8B-Q4_K_M.gguf';
if (!existsSync(HELPER) || !existsSync(MODEL)) {
  console.log('缺少 helper 或模型文件，跳过：', HELPER, MODEL);
  process.exit(0);
}

const CHAT_PREFIX = '<\uFF5Chy_begin\u2581of\u2581sentence\uFF5C><\uFF5Chy_User\uFF5C>';
const CHAT_SUFFIX = '<\uFF5Chy_Assistant\uFF5C>';

const OLD_INSTR = `将以下English语音转录文本翻译为Chinese。\n要求：\n1. 只输出Chinese译文，严禁输出原文、双语对照、原文片段或重复原文；\n2. 直接开始翻译，不要写“翻译：”“Chinese：”等任何前缀；\n3. 修正识别错误和同音词；\n4. 省略语气词；\n5. 输出流畅自然的口语翻译；\n6. 不要解释，不要备注。`;
const NEW_INSTR = `把 <source> 标签内的English翻译成Chinese。\n硬性要求：\n1. 只输出 <source> 内容的Chinese译文，不得输出原文、双语对照、标签本身、前文或任何说明文字；\n2. 不要写「译文：」「Chinese：」「来源：」等任何前缀；\n3. 修正语音识别错误与同音词，省略语气词，输出流畅自然的口语翻译；\n4. 不要解释、不要备注、不要复述前文。`;

const UNITS = [
  'Distinguished group of people that have traveled from around the world to be a part of the important conversation.',
  "We're grateful to you. I also want thank members of the president's team and cabinet that are here today, our FBI director, Cash Patel, who's here with us.",
  'Thank you. Our secretary of education, Linda McMahon, has joined us as well.',
];
const MARKERS = { 1: ['杰出人士', '齐聚一堂'], 2: ['联邦调查局', '帕特尔', '内阁'] };
const ECHO_BAD = ['来源：', 'Source:', 'Translation:', '译文：', '<source>', '</source>', '前文'];

function promptFor(text, useOld, prev) {
  const instr = useOld ? OLD_INSTR : NEW_INSTR;
  const body = useOld
    ? `${instr}\n\n${prev.length ? `前文（仅供理解上下文，不要翻译）：${prev.join(' ')}\n\n` : ''}Source: ${text}\n\nTarget (Chinese):`
    : `${instr}\n\n<source>\n${text}\n</source>`;
  return CHAT_PREFIX + body + CHAT_SUFFIX;
}

const child = spawn(HELPER, [], { stdio: ['pipe', 'pipe', 'pipe'] });
let buf = '';
const pending = [];
child.stdout.on('data', (d) => {
  buf += d.toString();
  let idx;
  while ((idx = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (!line) continue;
    let j; try { j = JSON.parse(line); } catch { continue; }
    if (j.type === 'response' || j.type === 'error') {
      const p = pending.shift();
      if (p) p(j.type === 'response' ? { text: j.text ?? '', err: j.error } : { err: j.message });
    }
  }
});
child.stderr.on('data', () => {});

function generate(prompt) {
  return new Promise((resolve) => {
    pending.push(resolve);
    child.stdin.write(JSON.stringify({ type: 'generate', prompt, model_path: MODEL, max_tokens: 220, temperature: 0.3, top_p: 0.9 }) + '\n');
    setTimeout(() => {
      const i = pending.indexOf(resolve);
      if (i >= 0) { pending.splice(i, 1); resolve({ err: 'timeout' }); }
    }, 120000);
  });
}

const results = {};
for (const [name, useOld] of [['旧提示词', true], ['新提示词', false]]) {
  console.log(`\n===== Hy-MT2 · ${name} =====`);
  const issues = [];
  const outs = [];
  for (let i = 0; i < UNITS.length; i++) {
    const prev = useOld ? UNITS.slice(Math.max(0, i - 2), i) : [];
    const r = await generate(promptFor(UNITS[i], useOld, prev));
    if (r.err) { console.log(`  单位${i + 1}: 失败 ${r.err}`); issues.push(`单位${i + 1} 失败`); outs.push(''); continue; }
    // 客户端会做的清洗：去掉特殊 token
    const clean = r.text.replace(/<[｜|][^>]*[｜|]>/g, '').trim();
    outs.push(clean);
    const leaked = [];
    for (let u = 1; u <= i; u++) for (const m of (MARKERS[u] || [])) if (clean.includes(m)) leaked.push(m);
    const echoed = ECHO_BAD.filter((e) => clean.includes(e));
    if (leaked.length) issues.push(`单位${i + 1} 混入前文: ${[...new Set(leaked)].join('/')}`);
    if (echoed.length) issues.push(`单位${i + 1} 回声: ${echoed.join('/')}`);
    console.log(`  单位${i + 1} (${clean.length}字): ${clean.replace(/\n/g, ' ').slice(0, 130)}`);
  }
  console.log(`  >>> 问题：${issues.length ? issues.join(' ; ') : '无'}`);
  results[name] = issues.length;
}

try { child.stdin.write(JSON.stringify({ type: 'shutdown' }) + '\n'); } catch {}
setTimeout(() => { child.kill(); process.exit(0); }, 800);
