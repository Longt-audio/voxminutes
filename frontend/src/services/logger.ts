/**
 * Frontend logging bridge.
 *
 * Overrides the global `console` methods so that log/warn/error messages are
 * also forwarded to the Tauri Rust layer, where they are written into the
 * single application log file. This makes web-side errors available in the
 * support bundle without relying on the DevTools console.
 *
 * The implementation is defensive:
 * - If Tauri is not available (e.g. browser dev preview) it falls back to the
 *   native console.
 * - Messages are buffered and flushed periodically to avoid per-message IPC
 *   overhead on hot paths.
 * - Failures to send are silently ignored to prevent infinite error loops.
 */

import { invoke } from '@tauri-apps/api/core'

let isTauri = false
try {
  // In a non-Tauri browser preview the @tauri-apps/api imports may throw.
  isTauri =
    typeof window !== 'undefined' &&
    (('__TAURI__' in window && (window as any).__TAURI__ != null) ||
      ('__TAURI_INTERNALS__' in window && (window as any).__TAURI_INTERNALS__ != null))
} catch {
  isTauri = false
}

type LogLevel = 'log' | 'info' | 'warn' | 'error' | 'debug'

interface QueuedMessage {
  level: LogLevel
  message: string
  file?: string
  line?: number
}

const FLUSH_INTERVAL_MS = 250
const MAX_QUEUE_SIZE = 100
const MAX_MESSAGE_LENGTH = 4000

const queue: QueuedMessage[] = []
let flushTimer: ReturnType<typeof setTimeout> | null = null
let fallbackToNative = !isTauri

function safeSerialize(args: unknown[]): string {
  try {
    return args
      .map((arg) => {
        if (typeof arg === 'string') return arg
        if (arg instanceof Error) return `${arg.name}: ${arg.message}\n${arg.stack || ''}`
        try {
          return JSON.stringify(arg)
        } catch {
          return String(arg)
        }
      })
      .join(' ')
  } catch {
    return String(args)
  }
}

function captureLocation(): { file?: string; line?: number } {
  try {
    const stack = new Error().stack
    if (!stack) return {}
    // Skip this function and the console override frame.
    const lines = stack.split('\n').slice(3)
    for (const line of lines) {
      const match = line.match(/\s+at\s+(?:.*\s+\()?(.+?):(\d+):(\d+)\)?$/)
      if (match) {
        const file = match[1]
        if (!file.includes('logger.ts')) {
          return { file, line: parseInt(match[2], 10) }
        }
      }
    }
  } catch {
    // ignore
  }
  return {}
}

function enqueue(level: LogLevel, args: unknown[]) {
  if (fallbackToNative) return

  const text = safeSerialize(args)
  const { file, line } = captureLocation()
  const message: QueuedMessage = {
    level,
    message: text.length > MAX_MESSAGE_LENGTH ? text.slice(0, MAX_MESSAGE_LENGTH) + '...' : text,
    file,
    line,
  }

  queue.push(message)
  if (queue.length > MAX_QUEUE_SIZE) {
    queue.shift()
  }

  if (!flushTimer) {
    flushTimer = setTimeout(flushQueue, FLUSH_INTERVAL_MS)
  }
}

async function flushQueue() {
  flushTimer = null
  if (queue.length === 0 || fallbackToNative) return

  const batch = queue.splice(0, queue.length)
  try {
    for (const item of batch) {
      await invoke('frontend_log', {
        level: item.level,
        message: item.message,
        file: item.file ?? null,
        line: item.line ?? null,
      })
    }
  } catch {
    // If the bridge fails, stop trying to send to avoid flooding IPC.
    fallbackToNative = true
  }
}

const nativeConsole = {
  log: console.log.bind(console),
  info: console.info.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
  debug: console.debug.bind(console),
}

function overrideConsole() {
  if (!isTauri) return

  console.log = (...args: unknown[]) => {
    nativeConsole.log(...args)
    enqueue('log', args)
  }
  console.info = (...args: unknown[]) => {
    nativeConsole.info(...args)
    enqueue('info', args)
  }
  console.warn = (...args: unknown[]) => {
    nativeConsole.warn(...args)
    enqueue('warn', args)
  }
  console.error = (...args: unknown[]) => {
    nativeConsole.error(...args)
    enqueue('error', args)
  }
  console.debug = (...args: unknown[]) => {
    nativeConsole.debug(...args)
    enqueue('debug', args)
  }
}

let initialized = false

export function initFrontendLogger() {
  if (initialized) return
  initialized = true
  overrideConsole()
}

// Auto-initialize in Tauri environments. Call explicitly if imported before
// the Tauri bridge is ready.
if (isTauri) {
  initFrontendLogger()
}

export function flushFrontendLogs(): Promise<void> {
  return flushQueue()
}
