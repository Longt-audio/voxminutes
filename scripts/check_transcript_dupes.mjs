// scripts/check_transcript_dupes.mjs
//
// 录音转写质量体检：扫一条（或全部）录音的 transcripts.json，报告三类已知缺陷：
//   ① 尾部重复：相邻两段里，后一段是前一段**尾巴**的重复（2026-09-24 豆包 definite
//      边界晚于客户端寿命兜底闭合 → 被误判成新会话 → 尾巴再发一遍；已修，
//      回归验证用这个脚本）；
//   ② 巨型段落：单段字数超过阈值（修前实测出现过 734 / 1428 字）；
//   ③ 中文混入：英文转写里出现中文串（豆包模型特性，如 "哈哈哈"）。
//
// 用法：
//   node scripts/check_transcript_dupes.mjs                      # 扫 ~/recordings 下最近 10 条
//   node scripts/check_transcript_dupes.mjs --n 30               # 最近 30 条
//   node scripts/check_transcript_dupes.mjs --dir ~/recordings/录音_xxx
//   node scripts/check_transcript_dupes.mjs --files transcripts.json,transcripts_offline.json
//
// 退出码：0 = 没发现问题；1 = 有问题（可挂到回归里）。

import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const arg = (n, d) => {
  const i = process.argv.indexOf(`--${n}`);
  if (i < 0) return d;
  const v = process.argv[i + 1];
  return v && !v.startsWith('--') ? v : true;
};

const ROOT = String(arg('dir', join(homedir(), 'recordings')));
const TOP_N = Number(arg('n', 10));
const FILES = String(arg('files', 'transcripts.json,transcripts_offline.json')).split(',');
const GIANT_CHARS = Number(arg('giant', 400)); // 英文按字符数（~400 字 ≈ 30s 以上连续语音）

const norm = (s) =>
  String(s || '')
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fff]+/g, '');

/**
 * 后一段是否重复了前一段的**尾部内容**。
 *
 * 实测形态（2026-09-24 08:23）：段15 尾巴是
 *   "…be in this movie i'm really glad that i'm that girl right there excuse me don't"
 * 而段16 是 "Be in this movie i'm really glad that I'm that girl right there."
 * —— 重复的那句在段15 里**不在末尾**（后面还有 "excuse me don't"），
 * 所以不能只判 endsWith，要在「前段后半部分」里找子串。
 */
function tailRepeat(prev, cur) {
  const a = norm(prev);
  const b = norm(cur);
  if (b.length < 18 || a.length < b.length) return null;
  const zone = a.slice(Math.max(0, a.length - Math.max(b.length * 2, 80)));
  if (zone.includes(b)) return { overlap: b.length, kind: '后段整句在前段尾部出现过' };
  // 允许后段少几个字（定稿边界差一两个词）
  for (let drop = 1; drop <= Math.min(10, b.length - 18); drop++) {
    const b2 = b.slice(0, b.length - drop);
    if (zone.includes(b2)) return { overlap: b2.length, kind: `后段与前段尾部重合（少 ${drop} 字）` };
  }
  return null;
}

function checkFile(path) {
  let segs;
  try {
    const j = JSON.parse(readFileSync(path, 'utf8'));
    segs = j.segments || j;
  } catch {
    return null;
  }
  if (!Array.isArray(segs)) return null;
  const issues = [];
  const hints = [];
  segs.forEach((s, i) => {
    const text = String(s.text || '');
    if (text.length >= GIANT_CHARS) issues.push({ i, type: '巨型段落', detail: `${text.length} 字` });
    // 中文混入：只在「以拉丁字母为主」的段落里找中文串 —— 纯中文段落（中文会议/中文新闻）
    // 本来就该是中文，不能报（v1 版按「出现中文串」判，把中文新闻误报了一片）。
    const cjkRuns = text.match(/[\u4e00-\u9fff]{2,}/g);
    const latinCount = (text.match(/[A-Za-z]/g) || []).length;
    const cjkCount = (text.match(/[\u4e00-\u9fff]/g) || []).length;
    // 只作为**提示**，不计入失败：新闻/双语内容里中英混排是正常的
    // （实测中文新闻里 "Ladies and gentlemen 女士们，先生们" 会被这条命中）。
    if (cjkRuns && latinCount > cjkCount * 2 && latinCount >= 20) {
      hints.push(`段#${i} 疑似中文混入 ${JSON.stringify(cjkRuns.slice(0, 2))}`);
    }
    if (i > 0) {
      const rep = tailRepeat(segs[i - 1].text, text);
      if (rep) issues.push({ i, type: '尾部重复', detail: `与前段重合 ${rep.overlap} 字（${rep.kind}）` });
    }
  });
  return { count: segs.length, issues, hints };
}

const dirs = [];
if (String(arg('dir', '')) ) {
  dirs.push(ROOT);
} else {
  const all = readdirSync(ROOT)
    .map((d) => ({ d, p: join(ROOT, d) }))
    .filter((x) => {
      try { return statSync(x.p).isDirectory(); } catch { return false; }
    })
    .map((x) => ({ ...x, m: statSync(x.p).mtimeMs }))
    .sort((a, b) => b.m - a.m)
    .slice(0, TOP_N);
  dirs.push(...all.map((x) => x.p));
}

let bad = 0;
let checked = 0;
for (const dir of dirs) {
  for (const f of FILES) {
    const p = join(dir, f);
    if (!existsSync(p)) continue;
    const r = checkFile(p);
    if (!r) continue;
    checked++;
    const name = dir.split('/').pop();
    if (r.issues.length === 0) {
      console.log(`✅ ${name}/${f}   ${r.count} 段，未发现重复/巨型段落`);
      for (const h of r.hints) console.log(`      ℹ️  ${h}（中英混排属正常，仅供参考）`);
      continue;
    }
    bad++;
    console.log(`⚠️  ${name}/${f}   ${r.count} 段，${r.issues.length} 处问题：`);
    for (const it of r.issues.slice(0, 6)) {
      console.log(`      · 段#${it.i}  ${it.type}  ${it.detail}`);
    }
    if (r.issues.length > 6) console.log(`      · …还有 ${r.issues.length - 6} 处`);
  }
}
console.log(`\n检查 ${checked} 个转写文件：${bad === 0 ? '全部干净 ✅' : `${bad} 个有问题`}`);
process.exit(bad ? 1 : 0);
