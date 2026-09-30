#!/usr/bin/env node
/**
 * VoxMinutes UI 冒烟测试（免手动点击，专抓「整页白屏 / 页面级崩溃」这类只在
 * 客户端渲染期才暴露的问题）。
 *
 * 为什么需要它：2026-09-26 的「翻译页齿轮 → 白屏」事故在 Rust 日志里**没有任何痕迹**
 * （前端 console 当时也没接进日志），只能靠一个真实浏览器加载页面才能复现。
 * 本脚本用 CDP 驱动无头 Chrome：注入一份最小的 Tauri IPC mock（否则 invoke 全 reject，
 * 页面行为与真机不同），逐页加载 + 模拟客户端路由跳转，检查 DOM 节点数 / 文本长度 /
 * 渲染期异常，任一页异常即非 0 退出。
 *
 * 用法（需先起 dev server）：
 *   cd ~/vox/voxminutes/frontend && pnpm dev            # 另开一个终端
 *   node ~/vox/voxminutes/scripts/ui-smoke.mjs          # 默认 http://127.0.0.1:3118
 *   BASE=http://127.0.0.1:3118 node scripts/ui-smoke.mjs
 *
 * 说明：
 *  - 会自动用 --headless=new 起一个临时 Chrome（端口 9223，profile 在 $TMPDIR），跑完关掉。
 *    如需自定义浏览器路径：CHROME=/path/to/Chrome。
 *  - 非 Tauri 环境里所有 IPC 都被 mock 成"成功但空数据"，所以它验证的是**渲染健壮性**
 *    （空数据不崩、路由不崩），不是业务功能。
 */
import { spawn } from 'node:child_process'
import { rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const BASE = process.env.BASE || 'http://127.0.0.1:3118'
const PORT = Number(process.env.CDP_PORT || 9223)
const CHROME = process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const PROFILE = join(tmpdir(), `vox-ui-smoke-${process.pid}`)

/** 按页面加载顺序检查：'→' 表示用点击链接做客户端路由跳转（更容易暴露渲染期问题）。 */
const STEPS = [
  { url: '/' },
  { url: '/history' },
  { url: '/translate' },
  { click: 'a[href*="translate/models"]', expect: '/translate/models' },
  { url: '/tts' },
  { url: '/settings' },
  { url: '/account' },
]

const MOCK = `
window.__CMDS = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
  transformCallback: function (cb) { var id = Math.floor(Math.random()*1e9); window['_'+id]=cb; return id; },
  invoke: function (cmd, args) {
    window.__CMDS.push(cmd);
    switch (cmd) {
      case 'plugin:event|listen': return Promise.resolve(1);
      case 'plugin:event|unlisten': return Promise.resolve(null);
      case 'plugin:app|version': return Promise.resolve('0.1.0');
      case 'plugin:window|is_maximized': return Promise.resolve(false);
      case 'get_remote_enabled': return Promise.resolve(true);
      case 'get_translation_engine': return Promise.resolve('hymt2');
      case 'get_downloadable_models': return Promise.resolve([]);
      case 'sherpa_onnx_get_models': return Promise.resolve([]);
      case 'summary_get_config': return Promise.resolve(null);
      case 'list_remote_models': return Promise.resolve({ models: [], updatedAt: 'smoke' });
      case 'api_get_recordings': return Promise.resolve([]);
      case 'api_get_recording_segments': return Promise.resolve({ segments: [], total: 0, hasMore: false });
      case 'api_get_recording': return Promise.resolve({ id: 'smoke', title: 'smoke', segments: [], created_at: '2026-01-01T00:00:00Z', duration_ms: 0, folder_path: null, source: 'local', status: 'completed' });
      default: return Promise.resolve(null);
    }
  }
};
window.__TAURI__ = { core: { invoke: window.__TAURI_INTERNALS__.invoke } };
`

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

function rpc(ws, state, method, params) {
  const id = ++state.id
  return new Promise((resolve) => {
    const onMsg = (ev) => {
      const m = JSON.parse(ev.data)
      if (m.id === id) { ws.removeEventListener('message', onMsg); resolve(m) }
    }
    ws.addEventListener('message', onMsg)
    ws.send(JSON.stringify({ id, method, params }))
  })
}

async function main() {
  // dev server 是否就绪（否则 Next 返回 500/拒连，误报）
  try {
    const r = await fetch(BASE + '/', { method: 'GET' })
    if (!r.ok) throw new Error('HTTP ' + r.status)
  } catch (e) {
    console.error(`❌ dev server 不可达：${BASE}（先在 frontend 目录跑 pnpm dev）\n   ${e.message}`)
    process.exit(2)
  }

  const chrome = spawn(CHROME, [
    '--headless=new',
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${PROFILE}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-gpu',
    'about:blank',
  ], { stdio: 'ignore' })

  let ws
  try {
    let version
    for (let i = 0; i < 40; i++) {
      try {
        version = await (await fetch(`http://127.0.0.1:${PORT}/json/version`)).json()
        break
      } catch { await sleep(250) }
    }
    if (!version) throw new Error('无头 Chrome 启动失败（CDP 端口未就绪）')

    const target = await (await fetch(`http://127.0.0.1:${PORT}/json/new?about:blank`, { method: 'PUT' })).json()
    ws = new WebSocket(target.webSocketDebuggerUrl)
    await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej })

    const state = { id: 0 }
    let errors = []
    ws.addEventListener('message', (ev) => {
      const m = JSON.parse(ev.data)
      if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') {
        const text = (m.params.args || []).map((a) => String(a.value ?? a.description ?? '')).join(' ')
        errors.push(text.split('\n')[0].slice(0, 180))
      } else if (m.method === 'Runtime.exceptionThrown') {
        const d = m.params.exceptionDetails
        errors.push('EXCEPTION ' + String((d.exception && d.exception.description) || d.text).split('\n')[0].slice(0, 180))
      }
    })

    await rpc(ws, state, 'Runtime.enable', {})
    await rpc(ws, state, 'Page.enable', {})
    await rpc(ws, state, 'Page.addScriptToEvaluateOnNewDocument', { source: MOCK })
    // 干净起点：清掉上一次跑残留的 localStorage（脏 store 会造成误报）
    await rpc(ws, state, 'Page.navigate', { url: BASE + '/' })
    await sleep(1500)
    await rpc(ws, state, 'Runtime.evaluate', { expression: 'localStorage.clear()' })

    let failed = 0
    for (const step of STEPS) {
      errors = []
      const label = step.url || `点击 ${step.click}`
      if (step.url) await rpc(ws, state, 'Page.navigate', { url: BASE + step.url })
      else await rpc(ws, state, 'Runtime.evaluate', { expression: `document.querySelector(${JSON.stringify(step.click)}).click()` })
      await sleep(step.url ? 4000 : 5000)

      const res = await rpc(ws, state, 'Runtime.evaluate', {
        expression: `JSON.stringify({
          path: location.pathname,
          nodes: document.querySelectorAll('*').length,
          boundary: (document.body ? document.body.innerText : '').includes('ran into a problem'),
          len: (document.body ? document.body.innerText : '').replace(/\\s+/g, ' ').trim().length
        })`,
        returnByValue: true,
      })
      const v = JSON.parse(res.result.result.value)
      const wrongRoute = step.expect && !v.path.startsWith(step.expect)
      const ok = !wrongRoute && !v.boundary && v.nodes >= 60 && v.len >= 80
      if (!ok) failed++
      console.log(`${ok ? '✅' : '❌'} ${label.padEnd(34)} path=${v.path.padEnd(18)} nodes=${String(v.nodes).padStart(4)} textLen=${String(v.len).padStart(5)}${v.boundary ? ' [错误边界]' : ''}`)
      for (const e of [...new Set(errors)].slice(0, 4)) console.log(`      ⚠️  ${e}`)
    }

    console.log(failed === 0 ? '\n全部页面通过（无白屏/无渲染期异常）' : `\n${failed} 个步骤失败`)
    process.exitCode = failed === 0 ? 0 : 1
  } finally {
    try { ws?.close() } catch {}
    chrome.kill('SIGKILL')
    rmSync(PROFILE, { recursive: true, force: true })
  }
}

main().catch((e) => { console.error('SMOKE FAILED:', e); process.exit(1) })
