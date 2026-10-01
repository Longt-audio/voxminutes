'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { LATEST_DOWNLOAD_URL, OFFICIAL_WEBSITE_URL } from '@/lib/site'
import { openExternalUrl } from '@/services/ipc'

/**
 * 应用内自动更新（2026-09-30 接入）。
 *
 * 设计要点：
 *   · **一切失败都要有兜底出口**：升级链路上任何一步（检查 / 下载 / 安装）
 *     出问题，用户都能退回到「下载完整安装包」和「去官网」——
 *     这条链路跨了 VPS、网络、签名校验、NSIS 安装器四个环节，
 *     任何一个环节在用户机器上翻车都不该让用户卡死。
 *   · 进度用 `downloadAndInstall` 的回调事件算，Started 给总长度、Progress 给增量。
 *   · 安装完成后 `relaunch()`；Windows 上 NSIS 会先结束进程，relaunch 可能不执行，
 *     所以调用处要容忍「进程直接消失」。
 */
export type UpdatePhase =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'latest' }
  | { kind: 'available'; version: string; notes?: string }
  | { kind: 'downloading'; version: string; received: number; total?: number }
  | { kind: 'installing'; version: string }
  | { kind: 'error'; stage: 'check' | 'update'; message: string }

export interface AppUpdater {
  phase: UpdatePhase
  /** 检查更新；有新版则进入 available */
  checkNow: () => Promise<void>
  /** 下载并安装；成功后重启应用 */
  installNow: () => Promise<void>
  /** 回到 idle（用户收起提示时用） */
  reset: () => void
  /** 兜底：下载完整安装包（VPS 直供） */
  downloadFullInstaller: () => void
  /** 兜底：去官网（含 GitHub 备用源） */
  openWebsite: () => void
}

export function useAppUpdater(options?: { autoCheck?: boolean }): AppUpdater {
  const [phase, setPhase] = useState<UpdatePhase>({ kind: 'idle' })
  // check() 返回的 Update 是 Tauri 资源，必须持有并在替换/安装后释放
  const updateRef = useRef<Update | null>(null)

  const releaseUpdate = useCallback(async () => {
    const u = updateRef.current
    updateRef.current = null
    if (u) {
      try {
        await u.close()
      } catch {
        /* 资源已释放，忽略 */
      }
    }
  }, [])

  const checkNow = useCallback(async () => {
    setPhase({ kind: 'checking' })
    await releaseUpdate()
    try {
      const u = await check()
      if (u?.available) {
        updateRef.current = u
        setPhase({ kind: 'available', version: u.version, notes: u.body ?? undefined })
      } else {
        setPhase({ kind: 'latest' })
      }
    } catch (e) {
      // 检查失败最常见的原因：VPS 暂时不可达 / latest.json 还没发布。
      // 这不影响使用，给出口而不是死胡同。
      setPhase({ kind: 'error', stage: 'check', message: String(e) })
    }
  }, [releaseUpdate])

  const installNow = useCallback(async () => {
    const u = updateRef.current
    if (!u) {
      setPhase({ kind: 'error', stage: 'update', message: 'no pending update' })
      return
    }
    const version = u.version
    let received = 0
    let total: number | undefined
    setPhase({ kind: 'downloading', version, received: 0 })
    try {
      // ⚠️ 这里**不能**传 { restartAfterInstall } —— 该选项在本项目锁定的
      // 2.10.x（与 Rust crate tauri-plugin-updater 2.10.1 对齐的那版）里不存在，
      // 只在更高版本的 DownloadOptions 里才有。显式重启统一交给下面的 relaunch()。
      await u.downloadAndInstall((ev) => {
        if (ev.event === 'Started') {
          total = ev.data.contentLength
          setPhase({ kind: 'downloading', version, received: 0, total })
        } else if (ev.event === 'Progress') {
          received += ev.data.chunkLength
          setPhase({ kind: 'downloading', version, received, total })
        } else if (ev.event === 'Finished') {
          // 下载完成 → 进入安装（NSIS 静默安装，随后应用会重启）
          setPhase({ kind: 'installing', version })
        }
      })
      // Windows 上 NSIS 安装器通常已经把进程结束了；能走到这里说明还活着，
      // 主动重启一次，保证用户看到的是新版本。
      await relaunch()
    } catch (e) {
      setPhase({ kind: 'error', stage: 'update', message: String(e) })
    }
  }, [])

  const reset = useCallback(() => {
    setPhase({ kind: 'idle' })
  }, [])

  const downloadFullInstaller = useCallback(() => {
    void openExternalUrl(LATEST_DOWNLOAD_URL).catch(() => {})
  }, [])

  const openWebsite = useCallback(() => {
    if (OFFICIAL_WEBSITE_URL) void openExternalUrl(OFFICIAL_WEBSITE_URL).catch(() => {})
  }, [])

  useEffect(() => {
    if (options?.autoCheck) void checkNow()
    // 卸载时释放资源
    return () => {
      void releaseUpdate()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return { phase, checkNow, installNow, reset, downloadFullInstaller, openWebsite }
}

/** 把「已下载字节 / 总字节」算成 0~100 的整数；总长度未知时返回 null（走不确定进度条） */
export function progressPercent(received: number, total?: number): number | null {
  if (!total || total <= 0) return null
  return Math.max(0, Math.min(100, Math.round((received / total) * 100)))
}
