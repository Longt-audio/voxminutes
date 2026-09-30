'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import {
  apiGetSettings,
  apiSaveSetting,
  sherpaOnnxGetModelsDirectory,
  setModelsDirectoryCustom,
  getRemoteConfig,
  getRemoteEnabled,
  setRemoteConfig,
  setRemoteEnabled,
  fetchNoticeDocuments,
  fetchWelcome,
  openExternalUrl,
  type NoticeDocument,
  type WelcomeMessage,
} from '@/services/ipc'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import { dispatchRemoteConfigChanged, onRemoteConfigChanged } from '@/lib/remoteConfigSync'
import { ClaimCreditsButton } from '@/components/remote/ClaimCreditsButton'
import { RemoteServerStatus } from '@/components/remote/RemoteServerStatus'
import { Input } from '@/components/ui/input'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import { useModelDownload } from '@/hooks/useModelDownload'
import { LanguageTabs } from '@/components/LanguageTabs'
import { SourceLinksPanel } from '@/components/models/SourceLinks'
import { formatSize, stageText, modelGroup, modelDesc, modelDisplayName } from '@/lib/modelDisplay'
import { privacyPolicyUrl, termsOfServiceUrl, LEGAL_CONSENT_SETTING_KEY } from '@/lib/site'

/** 设置页「再次打开欢迎弹窗」通过该窗口事件通知 AppShell 里的弹窗打开（忽略禁用开关） */
export const OPEN_WELCOME_EVENT = 'vox:open-welcome'

/** 设置键：勾选「以后不再打开」= 设置这个键 */
const DISABLED_KEY = 'welcome.disabled'

/** 欢迎弹窗共三页：0 欢迎词+语言选择 / 1 公告+远程设置+不再打开 / 2 本地模型 */
const TOTAL_STEPS = 3

/** 各步骤的模型选项（多 id 表示一张卡对应多个模型，如 OPUS-MT 中英双向） */
const ASR_OPTIONS: string[][] = [['x-asr-480ms'], ['sense-voice']]
const TRANSLATE_OPTIONS: string[][] = [['opus-mt-zh-en', 'opus-mt-en-zh'], ['hy-mt2-1.8b-q4_k_m']]
const SUMMARY_OPTIONS: string[][] = [
  ['qwen2.5-3b-instruct-q4_k_m'],
  ['qwen3-4b-instruct-2507-q4_k_m'],
  ['gemma-3-4b-it-q4_k_m'],
]

/** 设置页勾选框用：读取/设置 welcome.disabled。 */
export async function getWelcomeDisabled(): Promise<boolean> {
  const settings = await apiGetSettings().catch(() => ({} as Record<string, string>))
  return !!settings[DISABLED_KEY]
}
export async function setWelcomeDisabled(disabled: boolean): Promise<void> {
  await apiSaveSetting(DISABLED_KEY, disabled ? 'true' : null).catch(() => {})
}

/** 欢迎弹窗（合并原「新手指引」与「启动弹窗」）：
 *  - 每次启动弹出 P1，除非用户勾选「以后不再打开」（welcome.disabled）。
 *  - P1：欢迎词（网关拉取，失败回退内置 i18n 默认词）+ 界面语言选择。
 *  - P2：公告区（fetchNoticeDocuments，本地缓存离线可读）
 *        + 远程设置（开关立即生效；只读服务器状态行+自动测试一次；免费领取积分；授权码折叠入口防抖自动保存）+「以后不再打开」。
 *  - P3：本地模型下载（沿用原新手指引的模型卡逻辑）。
 *  - 关闭/完成仍写 onboarding.completed='true'（兼容旧逻辑）。 */
export function WelcomeDialog() {
  const t = useMessages()
  const lang = useLanguageStore((s) => s.language)
  const [open, setOpen] = useState(false)
  const [step, setStep] = useState(0)
  // P1 的「我已阅读并同意条款」勾选（未勾选时不能继续，也不能关闭弹窗）
  const [consentChecked, setConsentChecked] = useState(false)
  // 公告文档（分页，最新在前）
  const [docs, setDocs] = useState<NoticeDocument[]>([])
  const [docIndex, setDocIndex] = useState(0)
  // 欢迎词（null = 使用内置 i18n 默认词）
  const [welcome, setWelcome] = useState<WelcomeMessage | null>(null)
  const [dontShow, setDontShow] = useState(false)
  // 每个模型卡的"链接"面板展开状态
  const [linksOpen, setLinksOpen] = useState<Record<string, boolean>>({})
  const [modelsDir, setModelsDir] = useState('')
  // 远程服务设置（P2）：地址不再可编辑（内置默认，自定义走设置页「自定义 LLM」tab 高级卡片），
  // remoteUrl 是只读展示的生效地址；授权码手动粘贴入口折叠，防抖自动保存
  const [remoteUrl, setRemoteUrl] = useState('')
  const [remoteIsDefault, setRemoteIsDefault] = useState(true)
  const [remoteKey, setRemoteKey] = useState('')
  const [remoteEnable, setRemoteEnable] = useState(false)
  // 远程配置是否已加载（未加载时不渲染领取按钮，防闪现）
  const [remoteCfgLoaded, setRemoteCfgLoaded] = useState(false)
  // 「我已有授权码」折叠入口
  const [keyEntryOpen, setKeyEntryOpen] = useState(false)
  // 本次会话刚领取成功：隐藏下方重复的授权码输入框（授权码已在绿色卡片里展示），
  // 下次打开弹窗时复位（loadRemoteConfig），恢复「有码即显示输入框」的既有行为
  const [justClaimed, setJustClaimed] = useState(false)
  // P2 首次展示时自动测一次连接（只自动一次）
  const [autoTestPending, setAutoTestPending] = useState(true)
  const {
    models,
    progressMap,
    startDownload,
    cancelDownload,
    importModel,
    importModelFolder,
    isModelBusy,
  } = useModelDownload()

  // 自动保存用：记录已加载的初始授权码，避免启动时误触发保存
  const loadedRef = useRef(false)
  const initialKeyRef = useRef('')
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // 加载远程配置（自动保存基线）。挂载时 + 每次打开弹窗时 + 收到他处保存事件时都要刷新，
  // 否则会出现「在用户中心填了授权码，欢迎弹窗里还是空的」。
  const loadRemoteConfig = useCallback(() => {
    Promise.all([
      getRemoteConfig().catch(() => null),
      getRemoteEnabled().catch(() => false),
    ]).then(([cfg, en]) => {
      if (cfg) {
        setRemoteUrl(cfg.serverUrl || '')
        setRemoteIsDefault(cfg.isDefault !== false)
        setRemoteKey(cfg.license || '')
        initialKeyRef.current = cfg.license || ''
      }
      setRemoteEnable(!!en)
      loadedRef.current = true
      setRemoteCfgLoaded(true)
      setJustClaimed(false)
    })
  }, [])

  // 启动：未禁用则打开 P1；同时加载远程配置 / 欢迎词 / 公告 / 模型目录。
  // 另外监听设置页「再次打开欢迎弹窗」派发的窗口事件（忽略禁用开关）。
  useEffect(() => {
    let disposed = false

    const init = async () => {
      try {
        const settings = await apiGetSettings()
        const disabled = !!settings[DISABLED_KEY]
        if (disposed) return
        setDontShow(disabled)
        if (!disabled) setOpen(true)
      } catch {
        // 后端不可用时静默，不打扰用户
      }
      loadRemoteConfig()
      // 模型存放目录
      sherpaOnnxGetModelsDirectory()
        .then((dir) => !disposed && setModelsDir(dir))
        .catch(() => {})
    }

    void init()

    const onReopen = () => {
      setStep(0)
      loadRemoteConfig()
      setOpen(true)
    }
    window.addEventListener(OPEN_WELCOME_EVENT, onReopen)

    // 他处（用户中心/设置页高级卡片）保存了远程配置 → 同步刷新本弹窗表单
    const offSync = onRemoteConfigChanged((d) => {
      if (disposed) return
      if (typeof d.serverUrl === 'string') setRemoteUrl(d.serverUrl)
      if (typeof d.isDefault === 'boolean') setRemoteIsDefault(d.isDefault)
      if (typeof d.license === 'string') {
        setRemoteKey(d.license)
        initialKeyRef.current = d.license
      }
      if (typeof d.enabled === 'boolean') setRemoteEnable(d.enabled)
    })

    return () => {
      disposed = true
      window.removeEventListener(OPEN_WELCOME_EVENT, onReopen)
      offSync()
    }
  }, [loadRemoteConfig])

  // 欢迎词 / 公告文档按界面语言拉取（网关 ?lang=；语言切换后重新拉取）。
  // 欢迎词失败回退内置 i18n 默认词（welcome 保持 null）；公告本地缓存优先，离线可读。
  // 注意：网关内容是运营文案，只用于正文 body；弹窗标题始终用本地 i18n（welDefaultTitle）。
  useEffect(() => {
    let disposed = false
    fetchWelcome(lang)
      .then((msg) => {
        if (!disposed && msg && (msg.title || msg.body)) setWelcome(msg)
      })
      .catch(() => {})
    fetchNoticeDocuments(lang)
      .then((items) => {
        if (!disposed && items.length) {
          setDocs(items)
          setDocIndex(0)
        }
      })
      .catch(() => {})
    return () => {
      disposed = true
    }
  }, [lang])

  // 弹窗每次打开都重新拉一次远程配置（组件常驻挂载，open 翻 true 时刷新）
  useEffect(() => {
    if (open) loadRemoteConfig()
  }, [open, loadRemoteConfig])

  // 授权码防抖自动保存（800ms）：地址不再由此处编辑（传空串 = Rust 端不覆盖已存地址，
  // 避免把 effective 默认地址误存成自定义值）
  useEffect(() => {
    if (!loadedRef.current) return
    const key = remoteKey.trim()
    if (key === initialKeyRef.current) return
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    saveTimerRef.current = setTimeout(async () => {
      if (!key) return // 空授权码不保存（避免清掉已存值）
      try {
        await setRemoteConfig('', key)
        initialKeyRef.current = key
        dispatchRemoteConfigChanged({ license: key })
      } catch (e) {
        toast.error(t.onbRemoteSaveFailed.replace('{error}', String(e)))
      }
    }, 800)
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [remoteKey])

  // 任何完成/关闭（含点 X）都写入 onboarding.completed，兼容旧逻辑
  const finish = useCallback(() => {
    apiSaveSetting('onboarding.completed', 'true').catch(() => {})
    setOpen(false)
    setStep(0)
  }, [])

  // 「以后不再打开」勾选即存 welcome.disabled
  const handleToggleDontShow = (next: boolean) => {
    setDontShow(next)
    apiSaveSetting(DISABLED_KEY, next ? 'true' : null).catch(() => {})
  }

  /** 勾选「我已阅读并同意」的瞬间记录同意时间（只写一次）。
   *  ⚠️ 不放在「下一步」里：弹窗每次启动都会弹，若无脑覆盖就把「首次同意时间」这个
   *  举证价值弄丢了。写入失败不阻塞 —— 同意由勾选动作构成，时间戳只是留痕。 */
  const recordConsentOnce = useCallback(() => {
    void (async () => {
      try {
        const settings = await apiGetSettings()
        if (settings[LEGAL_CONSENT_SETTING_KEY]) return
        await apiSaveSetting(LEGAL_CONSENT_SETTING_KEY, new Date().toISOString())
      } catch {
        // 静默：留痕失败不影响使用
      }
    })()
  }, [])

  const handleConsentChange = (next: boolean) => {
    setConsentChecked(next)
    if (next) recordConsentOnce()
  }

  /** P1「下一步」：勾选框已保证用户同意，这里只翻页。 */
  const handleAcceptAndContinue = useCallback(() => {
    setStep(1)
  }, [])

  /** 弹窗是否允许关闭：P1 未勾选同意前一律拦住（Esc / 点遮罩 / 右上角 X 都走这里）。 */
  const canClose = step > 0 || consentChecked
  const handleOpenChange = useCallback(
    (next: boolean) => {
      if (next) {
        setOpen(true)
        return
      }
      if (canClose) finish()
      // 不允许关闭时什么都不做：保持 open=true
    },
    [canClose, finish],
  )

  // 启用开关变化立即生效
  const handleToggleRemoteEnable = async (next: boolean) => {
    setRemoteEnable(next)
    try {
      await setRemoteEnabled(next)
      dispatchRemoteConfigChanged({ enabled: next })
    } catch (e) {
      setRemoteEnable(!next)
      toast.error(t.onbRemoteSaveFailed.replace('{error}', String(e)))
    }
  }

  const handleChangeModelsDir = async () => {
    try {
      const selected = await openDialog({ directory: true, title: t.setModelDir })
      if (!selected) return
      const dir = typeof selected === 'string' ? selected : selected?.[0]
      if (!dir) return
      const effective = await setModelsDirectoryCustom(dir)
      setModelsDir(effective)
    } catch {
      // 静默
    }
  }

  // 多模型卡（OPUS-MT 双向）同时开始下载（后端按模型互斥，不同模型可并行）
  const downloadAll = (ids: string[]) => {
    ids.forEach((id) => startDownload(id))
  }

  const asrInstalled = models.some((m) => modelGroup(m.id) === 'asr' && m.installed)

  // 模型选项卡：标题 + 描述 + 体积 + 下载/导入/取消 + 进度条
  const renderOption = (ids: string[], customTitle?: string) => {
    const infos = ids.map((id) => models.find((m) => m.id === id))
    if (infos.some((m) => !m)) return null // 后端未注册该模型时不展示
    const list = infos as NonNullable<(typeof infos)[number]>[]
    const installed = list.every((m) => m.installed)
    const busyInfo = list.find((m) => isModelBusy(m))
    const progress = busyInfo ? progressMap[busyInfo.id] : undefined
    const totalSize = list.reduce((sum, m) => sum + (m.size_bytes || 0), 0)
    const title = customTitle ?? modelDisplayName(ids[0], t, list[0].display_name)
    const desc = modelDesc(ids[0], t)

    return (
      <div key={ids.join('+')} className="rounded-md border border-border/60 px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="flex-1 min-w-0">
            <div className="truncate text-sm font-medium">{title}</div>
            {formatSize(totalSize) && (
              <div className="mt-0.5 text-xs tabular-nums text-muted-foreground">{formatSize(totalSize)}</div>
            )}
            {desc && <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div>}
          </div>
          {installed ? (
            <Badge variant="success">{t.setInstalled}</Badge>
          ) : busyInfo ? (
            <Button variant="outline" size="sm" onClick={() => cancelDownload(busyInfo.id)}>
              {t.comCancel}
            </Button>
          ) : (
            <>
              {/* 单模型卡支持从本地导入：压缩包/GGUF 文件 或 已解压的模型文件夹；
                  不同模型可并行下载/导入 */}
              {ids.length === 1 && (
                <>
                  <Button variant="outline" size="sm" onClick={() => importModel(ids[0])}>
                    {t.setImport}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => importModelFolder(ids[0])}>
                    {t.setImportFolder}
                  </Button>
                </>
              )}
              <Button size="sm" onClick={() => downloadAll(ids)}>
                {t.comDownload}
              </Button>
              {/* 展开/收起下载源直链（可复制到外部下载器） */}
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground"
                onClick={() => setLinksOpen((prev) => ({ ...prev, [ids[0]]: !prev[ids[0]] }))}
              >
                {t.setLinks}
              </Button>
            </>
          )}
        </div>
        {linksOpen[ids[0]] && !installed && (
          <SourceLinksPanel
            model={list[0]}
            disabled={!!busyInfo}
            onUseSource={(i) => startDownload(ids[0], i)}
          />
        )}
        {busyInfo && (
          <div className="mt-2">
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full bg-primary transition-all"
                style={{ width: `${Math.min(100, Math.max(0, progress?.percent ?? 0))}%` }}
              />
            </div>
            <div className="mt-1 text-xs tabular-nums text-muted-foreground">
              {progress ? stageText(progress, t) : t.setDownloadingPending}
            </div>
          </div>
        )}
      </div>
    )
  }

  const stepIndicator = t.onbStepIndicator
    .replace('{n}', String(step + 1))
    .replace('{total}', String(TOTAL_STEPS))

  // 标题始终走本地 i18n（网关下发的 title 是单语言运营文案，不随界面语言切换）；网关内容只用于正文 body
  const welcomeTitle = t.welDefaultTitle
  // 网关下发的欢迎词段落之间含空行（\n\n），渲染时折叠成单换行，段间紧凑不加空行（存储内容不动）
  const welcomeBody = (welcome?.body || t.welDefaultBody).replace(/\n{2,}/g, '\n')
  const doc = docs.length ? docs[Math.min(docIndex, docs.length - 1)] : null

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        hideClose={!canClose}
        className="max-w-2xl max-h-[85vh] overflow-y-auto custom-scrollbar"
      >
        {/* P1：欢迎词 + 界面语言选择（简洁页） */}
        {step === 0 && (
          <>
            <DialogHeader>
              <div className="mb-2 flex items-center gap-2.5">
                <div className="w-9 h-9 rounded-full bg-primary flex items-center justify-center shrink-0 overflow-hidden">
                  <svg width="18" height="13" viewBox="0 0 20 14" fill="none">
                    <rect x="0" y="6" width="3" height="8" rx="1" fill="white" />
                    <rect x="4.25" y="0" width="3" height="14" rx="1" fill="white" />
                    <rect x="8.5" y="4" width="3" height="10" rx="1" fill="white" />
                    <rect x="12.75" y="0" width="3" height="14" rx="1" fill="white" />
                    <rect x="17" y="5" width="3" height="9" rx="1" fill="white" />
                  </svg>
                </div>
                <DialogTitle>{welcomeTitle}</DialogTitle>
              </div>
              <DialogDescription className="whitespace-pre-wrap text-sm leading-normal max-h-[28vh] overflow-y-auto custom-scrollbar pr-1">
                {welcomeBody}
              </DialogDescription>
            </DialogHeader>

            {/* 界面语言选择（分段控件，一眼可见全部支持语言） */}
            <div className="flex flex-col items-center gap-1.5 rounded-md border border-border/60 py-3">
              <LanguageTabs />
              <p className="text-xs text-muted-foreground">{t.welChooseLanguage}</p>
            </div>

            {/* 首次启动的「告知—同意」（《个人信息保护法》要求：处理前须告知并取得同意）。
                规则：必须勾选才能点「下一步」；勾选前不允许关闭弹窗（Esc / 点遮罩 / 右上角 X
                都被拦下，X 也隐藏 —— 留着会让人以为点得动却毫无反应）。
                勾选的瞬间就把时间写入设置（legal.consent_accepted_at）留痕，便于应用商店审核
                与监管问询时举证。两个条款走系统浏览器打开官网页面。
                想放宽成「继续即同意」：把 Button 的 disabled 去掉、并让 onOpenChange 不拦即可。 */}
            <label className="flex cursor-pointer items-start justify-center gap-1.5 text-center text-[11px] leading-relaxed text-muted-foreground">
              <input
                type="checkbox"
                className="mt-[3px] h-3 w-3 shrink-0 accent-primary"
                checked={consentChecked}
                onChange={(e) => handleConsentChange(e.target.checked)}
              />
              <span>
                {t.welConsentCheck}{' '}
                <button
                  type="button"
                  className="underline underline-offset-2 hover:text-foreground"
                  onClick={(e) => {
                    e.preventDefault()
                    void openExternalUrl(termsOfServiceUrl(lang)).catch(() => {})
                  }}
                >
                  {t.welTermsLink}
                </button>{' '}
                {t.welConsentJoin}{' '}
                <button
                  type="button"
                  className="underline underline-offset-2 hover:text-foreground"
                  onClick={(e) => {
                    e.preventDefault()
                    void openExternalUrl(privacyPolicyUrl(lang)).catch(() => {})
                  }}
                >
                  {t.welPrivacyLink}
                </button>
              </span>
            </label>

            <div className="flex items-center justify-between">
              <span className="text-xs text-muted-foreground/70">{stepIndicator}</span>
              <Button disabled={!consentChecked} onClick={handleAcceptAndContinue}>
                {t.onbNext}
              </Button>
            </div>
          </>
        )}

        {/* P2：公告 + 远程设置 + 以后不再打开 */}
        {step === 1 && (
          <>
            <DialogHeader>
              <DialogTitle>{welcomeTitle}</DialogTitle>
            </DialogHeader>

            {/* 公告区（fetchNoticeDocuments：本地缓存优先，离线可读；最新在前，分页浏览） */}
            {doc && (
              <div className="rounded-md border border-border/60 px-4 py-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium">{t.welNoticesTitle}</span>
                  <div className="flex items-center gap-1">
                    {docs.length > 1 && (
                      <>
                        <Button
                          variant="outline"
                          size="sm"
                          className="h-7 px-2"
                          disabled={docIndex <= 0}
                          onClick={() => setDocIndex((i) => i - 1)}
                        >
                          <ChevronLeft className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          className="h-7 px-2"
                          disabled={docIndex >= docs.length - 1}
                          onClick={() => setDocIndex((i) => i + 1)}
                        >
                          <ChevronRight className="h-3.5 w-3.5" />
                        </Button>
                      </>
                    )}
                    <span className="ml-1 text-xs tabular-nums text-muted-foreground">
                      {t.onbStepIndicator
                        .replace('{n}', String(docIndex + 1))
                        .replace('{total}', String(docs.length))}
                    </span>
                  </div>
                </div>
                <div className="mt-2">
                  <div className="text-sm font-medium">{doc.title}</div>
                  {doc.updated_at && (
                    <div className="mt-0.5 text-xs text-muted-foreground">
                      {new Date(doc.updated_at).toLocaleString()}
                    </div>
                  )}
                  {doc.images && doc.images.length > 0 && (
                    <div className="mt-2 flex flex-col gap-2">
                      {doc.images.map((img, i) => (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img
                          key={i}
                          src={img}
                          alt={`${doc.title} ${i + 1}`}
                          className="w-full max-h-56 object-contain rounded-md border border-border/60"
                        />
                      ))}
                    </div>
                  )}
                  {doc.body && <p className="mt-2 text-sm whitespace-pre-wrap">{doc.body}</p>}
                </div>
              </div>
            )}

            {/* 远程服务设置：开关立即生效；授权码防抖自动保存；地址内置默认（高级改动在设置页） */}
            <div className="rounded-md border border-border/60 px-4 py-3">
              <div className="text-sm font-medium">{t.onbRemoteTitle}</div>
              <p className="mt-0.5 text-xs text-muted-foreground">{t.onbRemoteDesc}</p>
              <div className="mt-3 flex flex-col gap-2.5">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={remoteEnable}
                    onChange={(e) => void handleToggleRemoteEnable(e.target.checked)}
                    className="h-4 w-4"
                  />
                  <span>{t.onbRemoteEnable}</span>
                </label>
                {/* 只读服务器状态行 + 三态测试按钮（首次展示自动测一次） */}
                <RemoteServerStatus
                  serverUrl={remoteUrl}
                  isDefault={remoteIsDefault}
                  autoTest={autoTestPending}
                  onAutoTested={() => setAutoTestPending(false)}
                />
                {/* 无授权码时的醒目领取入口（已有授权码则不渲染）；领取成功通知本弹窗隐藏重复输入框 */}
                <ClaimCreditsButton
                  hasLicense={remoteCfgLoaded ? !!remoteKey.trim() : null}
                  onClaimed={() => setJustClaimed(true)}
                />
                {/* 授权码手动粘贴入口（换机老用户用）：无码时默认折叠；
                    刚领取成功时也不显示（授权码已在上方绿色卡片展示），仍可点「我已有授权码」展开 */}
                {!justClaimed && (keyEntryOpen || !!remoteKey.trim()) ? (
                  <div className="flex flex-col gap-1.5">
                    <span className="text-xs text-muted-foreground">{t.onbRemoteKey}</span>
                    {/* 明文显示：授权码需要用户自己保存备份，password 掩码不利于核对/抄写 */}
                    <Input
                      value={remoteKey}
                      onChange={(e) => setRemoteKey(e.target.value)}
                      placeholder="sk-…"
                      type="text"
                      className="font-mono text-xs"
                    />
                    <p className="text-xs text-muted-foreground">{t.onbRemoteKeySaveHint}</p>
                  </div>
                ) : (
                  <button
                    type="button"
                    className="self-start text-xs text-muted-foreground underline-offset-2 hover:underline"
                    onClick={() => {
                      setJustClaimed(false)
                      setKeyEntryOpen(true)
                    }}
                  >
                    {t.onbRemoteHasKey}
                  </button>
                )}
              </div>
            </div>

            {/* 未安装本地识别模型时提示去 P3 下载 */}
            {!asrInstalled && (
              <p className="text-xs text-muted-foreground">{t.welNoAsrHint}</p>
            )}

            <div className="flex items-center justify-between pt-2">
              <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={dontShow}
                  onChange={(e) => handleToggleDontShow(e.target.checked)}
                  className="h-3.5 w-3.5"
                />
                {t.welDontShowAgain}
              </label>
              <div className="flex items-center gap-3">
                <Button variant="ghost" onClick={() => setStep(0)}>
                  {t.onbBack}
                </Button>
                <span className="text-xs text-muted-foreground/70">{stepIndicator}</span>
                <Button onClick={() => setStep(2)}>{t.onbNext}</Button>
              </div>
            </div>
          </>
        )}

        {/* P3：本地模型下载 */}
        {step === 2 && (
          <>
            <DialogHeader>
              <DialogTitle>{t.onbStepLocalModelsTitle}</DialogTitle>
              <DialogDescription>{t.onbStepLocalModelsDesc}</DialogDescription>
            </DialogHeader>
            {/* 模型存放目录：首次使用可确认/修改 */}
            <div className="flex items-center gap-2 rounded-md border border-border/60 px-3 py-2">
              <span className="text-xs text-muted-foreground shrink-0">{t.setModelDir}</span>
              <span className="flex-1 min-w-0 truncate text-xs text-muted-foreground">{modelsDir || t.comLoading}</span>
              <Button variant="outline" size="sm" className="shrink-0" onClick={handleChangeModelsDir}>
                {t.setModelDirChange}
              </Button>
            </div>
            <div className="flex max-h-[48vh] flex-col gap-4 overflow-y-auto pr-1">
              <div>
                <h4 className="mb-2 text-sm font-medium">{t.setGroupAsr}</h4>
                <div className="flex flex-col gap-3">{ASR_OPTIONS.map((ids) => renderOption(ids))}</div>
              </div>
              <div>
                <h4 className="mb-2 text-sm font-medium">{t.setGroupTranslate}</h4>
                <div className="flex flex-col gap-3">
                  {renderOption(TRANSLATE_OPTIONS[0], t.onbOpusPairTitle)}
                  {renderOption(TRANSLATE_OPTIONS[1])}
                </div>
              </div>
              <div>
                <h4 className="mb-2 text-sm font-medium">{t.setGroupSummary}</h4>
                <div className="flex flex-col gap-3">{SUMMARY_OPTIONS.map((ids) => renderOption(ids))}</div>
              </div>
            </div>
            <div className="flex items-center justify-between pt-2">
              <Button variant="ghost" onClick={() => setStep(1)}>
                {t.onbBack}
              </Button>
              <div className="flex items-center gap-3">
                <span className="text-xs text-muted-foreground/70">{stepIndicator}</span>
                <Button onClick={finish}>{t.onbFinish}</Button>
              </div>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
