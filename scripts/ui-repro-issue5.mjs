#!/usr/bin/env node
/**
 * 问题 5 复现脚本：历史页录音详情「Maximum update depth exceeded」。
 * 在 ui-smoke.mjs 的基础上把 IPC mock 换成「有离线识别结果的录音」，
 * 选中录音 → 切实时/离线 tab → 双击编辑片段 → 开关会议总结弹窗 → 换选录音，
 * 全程收集 console error / 未捕获异常，出现 Maximum update depth 即非 0 退出。
 *
 * 用法（需先起 dev server）：
 *   cd ~/vox/voxminutes/frontend && pnpm dev
 *   node ~/vox/voxminutes/scripts/ui-repro-issue5.mjs
 */
import { spawn } from 'node:child_process'
import { rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const BASE = process.env.BASE || 'http://127.0.0.1:3118'
const PORT = Number(process.env.CDP_PORT || 9224)
const CHROME = process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const PROFILE = join(tmpdir(), `vox-ui-repro5-${process.pid}`)

// 一条带离线识别结果（说话人分离）+ 实时结果的录音；另一条无结果的普通录音
const MOCK = `
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
  transformCallback: function (cb) { var id = Math.floor(Math.random()*1e9); window['_'+id]=cb; return id; },
  invoke: function (cmd, args) {
    // 捕获事件监听注册，便于后面模拟后端推送 retranscription-* 事件
    if (cmd === 'plugin:event|listen' && args && args.event) {
      window.__LISTENERS = window.__LISTENERS || {};
      (window.__LISTENERS[args.event] = window.__LISTENERS[args.event] || []).push(args.handler);
    }
    var segs = [];
    for (var i = 0; i < 30; i++) {
      segs.push({ id: 'rt-' + i, recording_id: 'rec-offline', start_ms: i * 2000, end_ms: i * 2000 + 1500, text: 'realtime segment ' + i, source: 'Audio', created_at: '2026-01-01T00:00:00Z' });
      segs.push({ id: 'off-' + i, recording_id: 'rec-offline', start_ms: i * 2000, end_ms: i * 2000 + 1500, text: 'offline segment ' + i, source: 'offline_asr', speaker: 'SPEAKER_0' + (i % 3), created_at: '2026-01-01T00:00:00Z' });
    }
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
      case 'summary_load': return Promise.resolve(null);
      case 'list_remote_models': return Promise.resolve({ models: [], updatedAt: 'repro' });
      case 'get_speaker_names': return Promise.resolve({});
      case 'get_offline_recognition_info': return Promise.resolve({ model: 'deepgram-nova-3', duration_seconds: 120.5, elapsed_seconds: 6.7, warnings: [] });
      case 'api_get_recordings': return Promise.resolve([
        { id: 'rec-offline', title: 'with offline result', created_at: '2026-01-01T00:00:00Z', duration_ms: 60000, folder_path: '/tmp/rec-offline', source: 'record', status: 'completed' },
        { id: 'rec-plain', title: 'plain recording', created_at: '2026-01-02T00:00:00Z', duration_ms: 30000, folder_path: '/tmp/rec-plain', source: 'record', status: 'completed' }
      ]);
      case 'api_get_recording': return Promise.resolve({
        id: args && args.recordingId || 'rec-offline',
        title: 'with offline result',
        segments: (args && args.recordingId === 'rec-plain') ? segs.filter(function(s){return s.source !== 'offline_asr'}) : segs,
        created_at: '2026-01-01T00:00:00Z', duration_ms: 60000,
        folder_path: '/tmp/rec-offline', source: 'record', status: 'completed', asr_engine: 'x-asr-480ms'
      });
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

async function evalJs(ws, state, expression) {
  const res = await rpc(ws, state, 'Runtime.evaluate', { expression, returnByValue: true })
  return res.result?.result?.value
}

// 按可见文本点击按钮/行（返回是否点到了）
async function clickByText(ws, state, text, selector = 'button, [role="button"]') {
  return evalJs(ws, state, `(() => {
    const els = [...document.querySelectorAll(${JSON.stringify(selector)})]
    const el = els.find((e) => (e.innerText || '').trim().includes(${JSON.stringify(text)}))
    if (!el) return false
    el.click()
    return true
  })()`)
}

async function main() {
  try {
    const r = await fetch(BASE + '/', { method: 'GET' })
    if (!r.ok) throw new Error('HTTP ' + r.status)
  } catch (e) {
    console.error(`❌ dev server 不可达：${BASE}\n   ${e.message}`)
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
  let failed = 0
  try {
    let version
    for (let i = 0; i < 40; i++) {
      try {
        version = await (await fetch(`http://127.0.0.1:${PORT}/json/version`)).json()
        break
      } catch { await sleep(250) }
    }
    if (!version) throw new Error('无头 Chrome 启动失败')

    const target = await (await fetch(`http://127.0.0.1:${PORT}/json/new?about:blank`, { method: 'PUT' })).json()
    ws = new WebSocket(target.webSocketDebuggerUrl)
    await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej })

    const state = { id: 0 }
    let errors = []
    ws.addEventListener('message', (ev) => {
      const m = JSON.parse(ev.data)
      if (m.method === 'Runtime.consoleAPICalled' && (m.params.type === 'error' || m.params.type === 'warning')) {
        const text = (m.params.args || []).map((a) => String(a.value ?? a.description ?? '')).join(' ')
        errors.push(`[console.${m.params.type}] ` + text.split('\n')[0].slice(0, 220))
      } else if (m.method === 'Runtime.exceptionThrown') {
        const d = m.params.exceptionDetails
        errors.push('EXCEPTION ' + String((d.exception && d.exception.description) || d.text).split('\n')[0].slice(0, 220))
      }
    })

    await rpc(ws, state, 'Runtime.enable', {})
    await rpc(ws, state, 'Page.enable', {})
    await rpc(ws, state, 'Page.addScriptToEvaluateOnNewDocument', { source: MOCK })
    await rpc(ws, state, 'Page.navigate', { url: BASE + '/' })
    await sleep(1500)
    await evalJs(ws, state, 'localStorage.clear()')

    const step = async (label, fn, settleMs = 2500) => {
      const before = errors.length
      await fn()
      await sleep(settleMs)
      const fresh = errors.slice(before)
      const fatal = fresh.filter((e) => /Maximum update depth|too much recursion|Minified React error/i.test(e))
      const ok = fatal.length === 0
      if (!ok) failed++
      console.log(`${ok ? '✅' : '❌'} ${label}`)
      for (const e of [...new Set(fresh)].slice(0, 6)) console.log(`      ⚠️  ${e}`)
    }

    await step('打开 /history', async () => {
      await rpc(ws, state, 'Page.navigate', { url: BASE + '/history' })
    }, 4000)

    await step('选中「有离线识别结果」的录音', async () => {
      const ok = await clickByText(ws, state, 'with offline result')
      if (!ok) console.log('      ⚠️  没找到录音行')
    }, 4000)

    // 模拟后端推送：进度 → 增量结果 → 完成（走 completionTick → refresh + loadDetails 全链路）
    const emit = (event, payload) => evalJs(ws, state, `(() => {
      const ids = (window.__LISTENERS && window.__LISTENERS[${JSON.stringify(event)}]) || []
      ids.forEach((id) => window['_' + id] && window['_' + id]({ event: ${JSON.stringify(event)}, payload: ${JSON.stringify(payload)} }))
      return ids.length
    })()`)

    await step('后端事件：retranscription progress/partial/complete', async () => {
      const n1 = await emit('retranscription-progress', { meeting_id: 'rec-offline', progress_percentage: 50, message: '识别中', stage: 'transcribing' })
      const n2 = await emit('retranscription-partial', { meeting_id: 'rec-offline', chunk_index: 0, chunks_total: 2, start_ms: 0, text: 'partial chunk' })
      const n3 = await emit('retranscription-complete', { meeting_id: 'rec-offline', segments_count: 30, duration_seconds: 120.5, elapsed_seconds: 6.7, warnings: [] })
      console.log(`      listeners: progress=${n1} partial=${n2} complete=${n3}`)
    }, 4000)

    await step('切到实时 tab', async () => {
      await clickByText(ws, state, 'Real-time')
    })

    await step('切回离线 tab', async () => {
      await clickByText(ws, state, 'Offline')
    })

    await step('双击片段进入编辑 → 取消', async () => {
      await evalJs(ws, state, `(() => {
        const row = document.querySelector('.flex-1.min-h-0.overflow-y-auto [title]')
        if (row) row.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))
      })()`)
      await sleep(800)
      await clickByText(ws, state, 'Cancel')
    })

    await step('打开「会议总结」弹窗 → 关闭', async () => {
      await clickByText(ws, state, 'Meeting Summary')
      await sleep(2000)
      await clickByText(ws, state, 'Close')
    }, 3000)

    await step('换选另一条录音（无离线结果）', async () => {
      await clickByText(ws, state, 'plain recording')
    }, 4000)

    await step('再选回「有离线识别结果」的录音', async () => {
      await clickByText(ws, state, 'with offline result')
    }, 4000)

    await step('空闲观察 10s（轮询/延迟效应）', async () => {}, 10000)

    const nodes = await evalJs(ws, state, `document.querySelectorAll('*').length`)
    console.log(`\nDOM 节点数：${nodes}（>60 即未白屏）`)
    console.log(failed === 0 ? '未复现 Maximum update depth / 无渲染期异常' : `\n${failed} 个步骤出现致命渲染错误`)
    process.exitCode = failed === 0 ? 0 : 1
  } finally {
    try { ws?.close() } catch {}
    chrome.kill('SIGKILL')
    // Chrome 退出后可能还在写 profile：稍等再清理，失败不致命
    for (let i = 0; i < 5; i++) {
      try {
        rmSync(PROFILE, { recursive: true, force: true })
        break
      } catch {
        await sleep(500)
      }
    }
  }
}

main().catch((e) => { console.error('REPRO FAILED:', e); process.exit(1) })
