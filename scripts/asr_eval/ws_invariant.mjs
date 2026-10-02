// 校验网关流式 ASR 的「累计串一致性」不变量：
//   · 每个 final 必须以前一个 final 为前缀（定稿只增不回退）
//   · 每条 partial 必须以上一个 final 为前缀（否则客户端会把已提交文本回滚 → 重复出句）
// 这正是 2026-09-22「豆包大量重复文本」的判据。
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const MODEL = process.argv[2] || 'doubao-asr-streaming-2.0';
const SECONDS = Number(process.argv[3] || 45);
const BASE = 'https://api.voxmin.top/v1';

// 测试音频：用 VOX_TEST_AUDIO 指定任意本地音频（ffmpeg 能解码即可）
const rec = process.env.VOX_TEST_AUDIO;
if (!rec) {
  console.log('请用 VOX_TEST_AUDIO=/path/to/audio.mp4 指定测试音频后重跑。');
  process.exit(0);
}
// 解码为 16k mono PCM16
const pcm = execFileSync('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-i', rec, '-ac', '1', '-ar', '16000', '-f', 's16le', 'pipe:1'], { maxBuffer: 1 << 30 });
console.log(`pcm ${(pcm.length / 32000).toFixed(1)}s`);

const reg = await fetch(`${BASE}/register`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ device_id: 'ws-invariant-' + Math.random().toString(36).slice(2, 10) }) });
const { api_key } = await reg.json();

const wsUrl = `wss://api.voxmin.top/v1/audio/realtime-asr?model=${encodeURIComponent(MODEL)}`;
const ws = new WebSocket(wsUrl, { headers: { Authorization: `Bearer ${api_key}` } });

let lastFinal = '';
let finals = 0, partials = 0, violations = [];
const t0 = Date.now();

ws.addEventListener('open', () => {
  console.log('ws open');
  let offset = 0;
  const CHUNK = 16000 * 2 * 0.6; // 0.6s
  const timer = setInterval(() => {
    if (offset >= pcm.length || (Date.now() - t0) / 1000 > SECONDS) {
      clearInterval(timer);
      try { ws.send(JSON.stringify({ type: 'end' })); } catch {}
      setTimeout(() => { try { ws.close(); } catch {} }, 6000);
      return;
    }
    const end = Math.min(offset + CHUNK, pcm.length);
    ws.send(pcm.subarray(offset, end));
    offset = end;
  }, 600);
});

ws.addEventListener('message', (ev) => {
  let m; try { m = JSON.parse(String(ev.data)); } catch { return; }
  if (m.type === 'partial') {
    partials++;
    if (lastFinal && !String(m.text).startsWith(lastFinal)) {
      violations.push({ kind: 'partial-not-prefixed-by-final', final: lastFinal.slice(-60), got: String(m.text).slice(0, 120), at: ((Date.now() - t0) / 1000).toFixed(1) });
    }
  } else if (m.type === 'final') {
    finals++;
    const text = String(m.text || '');
    if (lastFinal && !text.startsWith(lastFinal)) {
      violations.push({ kind: 'final-not-prefixed-by-prev-final', prev: lastFinal.slice(-60), got: text.slice(0, 120), at: ((Date.now() - t0) / 1000).toFixed(1) });
    }
    if (text.length > lastFinal.length) lastFinal = text;
  } else if (m.type === 'error') {
    console.log('error:', JSON.stringify(m).slice(0, 200));
  }
});

ws.addEventListener('close', () => {
  console.log(`\nmodel=${MODEL} finals=${finals} partials=${partials} lastFinal=${lastFinal.length} chars`);
  console.log(`violations=${violations.length}`);
  for (const v of violations.slice(0, 8)) console.log('  ✗', JSON.stringify(v));
  console.log('final text head:', JSON.stringify(lastFinal.slice(0, 200)));
  process.exit(violations.length ? 2 : 0);
});

ws.addEventListener('error', (e) => console.log('ws error', String(e?.message || e)));
setTimeout(() => { console.log('timeout'); process.exit(3); }, (SECONDS + 30) * 1000);
