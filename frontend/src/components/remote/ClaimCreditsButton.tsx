'use client'

import { useState } from 'react'
import { CheckCircle2, Copy, Check, Loader2, Globe } from 'lucide-react'
import { registerDevice, getRemoteUsage, openExternalUrl } from '@/services/ipc'
import { dispatchRemoteConfigChanged } from '@/lib/remoteConfigSync'
import { OFFICIAL_WEBSITE_URL, OFFICIAL_WEBSITE_LABEL } from '@/lib/site'
import { Button } from '@/components/ui/button'
import { useMessages } from '@/i18n/useMessages'

type ClaimState = 'idle' | 'claiming' | 'ok' | 'restored' | 'error'

/** 「免费领取 200 积分」按钮（欢迎弹窗 P2 与账户页共用）：
 *  调 registerDevice（设备绑定自动注册，网关 /v1/register）。
 *  - 无授权码：醒目主按钮 + 副文案
 *  - 领取中：转圈禁用；失败：按钮变「重试」+ 错误文案
 *  - 成功：按钮消失，替换为成功状态 —— **授权码明文显示 + 复制按钮 + 保存提醒**
 *    （is_new=false 老设备显示「已恢复账号」）
 *  - 已有授权码：整体不渲染（账户页有「我的授权码」卡片，不重复） */
export function ClaimCreditsButton({
  hasLicense,
  onClaimed,
}: {
  /** 是否已有授权码；传 null 表示配置未加载完成（先不渲染，避免按钮闪现） */
  hasLicense: boolean | null
  /** 领取成功回调（账户页刷新余额等） */
  onClaimed?: () => void
}) {
  const t = useMessages()
  const [state, setState] = useState<ClaimState>('idle')
  const [credits, setCredits] = useState(0)
  const [license, setLicense] = useState('')
  const [balance, setBalance] = useState<number | null>(null)
  const [copied, setCopied] = useState(false)
  const [error, setError] = useState('')

  const handleClaim = async () => {
    setState('claiming')
    setError('')
    try {
      const r = await registerDevice()
      setCredits(r.credits)
      setLicense(r.api_key || '')
      setState(r.is_new ? 'ok' : 'restored')
      // 授权码已由 Rust 端写入 keychain + 内存；同步给其他远程配置入口
      dispatchRemoteConfigChanged({ license: r.api_key })
      onClaimed?.()
      // 展示当前积分余额（复用账户页的 /v1/usage 查询；失败静默，不阻断成功态）
      getRemoteUsage()
        .then((u) => setBalance(u.credits))
        .catch(() => {})
    } catch (e) {
      setError(String(e))
      setState('error')
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(license)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      // 剪贴板不可用时静默（文本本身可选中手动复制）
    }
  }

  // 已有授权码且非本次刚领取 → 不渲染；hasLicense 未知（配置未加载完）也先不渲染
  if (state !== 'ok' && state !== 'restored' && hasLicense !== false) return null

  if (state === 'ok' || state === 'restored') {
    return (
      <div className="rounded-md border border-green-500/40 bg-green-500/10 px-3 py-2.5">
        <div className="flex items-center gap-1.5 text-sm font-medium text-green-600">
          <CheckCircle2 className="h-4 w-4 shrink-0" />
          {state === 'ok'
            ? `✓ ${t.accClaimOk.replace('{credits}', String(credits))}`
            : `✓ ${t.accClaimRestored}`}
        </div>
        {/* 授权码明文展示 + 复制（注册即自动填入生效，此处是给用户备份用） */}
        {license && (
          <div className="mt-2">
            <div className="text-xs text-muted-foreground">{t.accClaimLicenseLabel}</div>
            <div className="mt-1 flex items-center gap-1.5">
              <code className="flex-1 min-w-0 truncate rounded border border-border/60 bg-background px-2 py-1 font-mono text-xs select-all">
                {license}
              </code>
              <Button variant="outline" size="sm" className="h-7 shrink-0 gap-1 px-2 text-xs" onClick={() => void handleCopy()}>
                {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                {copied ? t.comCopied : t.comCopy}
              </Button>
            </div>
          </div>
        )}
        <p className="mt-1.5 text-xs text-muted-foreground">{t.accClaimSaveHint}</p>
        {/* 当前积分余额（/v1/usage；查询失败则不显示该块）：大号加粗品牌色，一眼可见 */}
        {balance !== null && (
          <div className="mt-2.5 rounded-md border border-primary/30 bg-primary/5 px-3 py-2">
            <div className="text-xs text-muted-foreground">{t.claimBalanceLabel}</div>
            <div className="mt-0.5 text-2xl font-bold tabular-nums text-primary">{balance}</div>
          </div>
        )}
        {/* 限领一次 + 赠送积分有效期说明 */}
        <p className="mt-1.5 text-xs text-muted-foreground">{t.claimOnceNote}</p>
        {/* 官网 / 发卡站：可购买充值卡充值积分（走 Rust open_external_url 用系统浏览器打开；
            window.open(…, '_blank') 在 Tauri 里会被静默取消） */}
        <p className="mt-1.5 flex items-center gap-1 text-xs text-muted-foreground">
          <Globe className="h-3 w-3 shrink-0" />
          {t.accClaimWebsiteHint}
          <button
            type="button"
            className="text-primary underline underline-offset-2 hover:opacity-80"
            onClick={() => void openExternalUrl(OFFICIAL_WEBSITE_URL).catch(() => {})}
          >
            {OFFICIAL_WEBSITE_LABEL}
          </button>
        </p>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-1.5">
      <Button onClick={() => void handleClaim()} disabled={state === 'claiming'} className="w-full">
        {state === 'claiming' && <Loader2 className="h-4 w-4 animate-spin" />}
        {state === 'claiming' ? t.accClaiming : state === 'error' ? t.accRetry : t.accClaimBtn}
      </Button>
      {state === 'error' ? (
        <p className="text-xs text-destructive">{t.accClaimFailed.replace('{error}', error)}</p>
      ) : (
        <p className="text-xs text-muted-foreground">{t.accClaimDesc}</p>
      )}
    </div>
  )
}
