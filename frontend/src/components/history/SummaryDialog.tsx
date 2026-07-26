'use client'

import { useEffect, useMemo, useRef, useState } from 'react'
import Link from 'next/link'
import { toast } from 'sonner'
import { open as openUrl } from '@tauri-apps/plugin-shell'
import { Trash2 } from 'lucide-react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { cn } from '@/lib/utils'
import { modelDisplayName } from '@/lib/modelDisplay'
import { useMessages } from '@/i18n/useMessages'
import { useLanguageStore } from '@/stores/languageStore'
import {
  composeFullPrompt,
  getBuiltinPrompts,
  loadPrompts,
  loadUserPrompts,
  pickUserPrompts,
  resetBuiltinOverride,
  saveCustomPrompts,
  type SummaryPromptPreset,
} from '@/lib/summaryPrompts'
import { loadAiWebSites, resetAiWebSites, saveAiWebSites, type AiWebSite } from '@/lib/aiWebSites'
import { summaryGetConfig, summaryLocalModels } from '@/services/ipc'
import type { SummaryGenerateParams } from '@/hooks/useSummaryGeneration'
import type { SummaryApiConfig, SummaryLocalModelInfo } from '@/types'

type MethodTab = 'web' | 'api' | 'local'

// 方式 tab 顺序持久化 key；读取时校验恰好包含 3 个合法 id，否则回退默认
const METHOD_ORDER_KEY = 'voxminutes-summary-method-order'
const DEFAULT_METHOD_ORDER: MethodTab[] = ['local', 'api', 'web']

function loadMethodOrder(): MethodTab[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(METHOD_ORDER_KEY) ?? 'null')
    if (
      Array.isArray(parsed) &&
      parsed.length === DEFAULT_METHOD_ORDER.length &&
      DEFAULT_METHOD_ORDER.every((m) => parsed.includes(m))
    ) {
      return parsed as MethodTab[]
    }
  } catch {}
  return DEFAULT_METHOD_ORDER
}

// 转写过长时只保留开头 16000 + 结尾 6000 字符
const MAX_TRANSCRIPT_CHARS = 24000
const HEAD_CHARS = 16000
const TAIL_CHARS = 6000

interface SummaryDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  transcript: string
  /** 点击「生成」：关闭本配置对话框，由父组件打开结果面板 */
  onGenerate: (params: SummaryGenerateParams) => void
  /** 受控的模板选择（与 RecordingDetail 行内选择器共享）；不传则内部维护 */
  promptId?: string
  onPromptChange?: (id: string) => void
}

/** 会议总结配置对话框：AI 网站 / API / 本地模型三种方式 + 模板与 prompt 编辑（生成在结果面板进行） */
export function SummaryDialog({
  open,
  onOpenChange,
  transcript,
  onGenerate,
  promptId: controlledPromptId,
  onPromptChange,
}: SummaryDialogProps) {
  const t = useMessages()
  const language = useLanguageStore((s) => s.language)

  const [methodOrder, setMethodOrder] = useState<MethodTab[]>(DEFAULT_METHOD_ORDER)
  const [method, setMethod] = useState<MethodTab>(DEFAULT_METHOD_ORDER[0])
  const dragTabRef = useRef<MethodTab | null>(null)
  const [prompts, setPrompts] = useState<SummaryPromptPreset[]>([])
  const [innerPromptId, setInnerPromptId] = useState('default')
  const [promptContent, setPromptContent] = useState('')
  const [editorOpen, setEditorOpen] = useState(false)
  const [addOpen, setAddOpen] = useState(false)
  const [newPresetName, setNewPresetName] = useState('')
  const [newPresetContent, setNewPresetContent] = useState('')

  const [sites, setSites] = useState<AiWebSite[]>([])
  const [manageOpen, setManageOpen] = useState(false)
  const [newSiteName, setNewSiteName] = useState('')
  const [newSiteUrl, setNewSiteUrl] = useState('')

  const [apiConfig, setApiConfig] = useState<SummaryApiConfig | null>(null)
  const [localModels, setLocalModels] = useState<SummaryLocalModelInfo[] | null>(null)
  const [localModelId, setLocalModelId] = useState('')

  const promptId = controlledPromptId ?? innerPromptId

  // 挂载后读取持久化的 tab 顺序（localStorage 仅客户端可用）
  useEffect(() => {
    const order = loadMethodOrder()
    setMethodOrder(order)
    setMethod(order[0])
  }, [])

  // 超长转写截断：开头 + …… + 结尾
  const clip = useMemo(() => {
    if (transcript.length <= MAX_TRANSCRIPT_CHARS) return { text: transcript, truncated: false }
    return {
      text: transcript.slice(0, HEAD_CHARS) + '\n\n……\n\n' + transcript.slice(-TAIL_CHARS),
      truncated: true,
    }
  }, [transcript])

  // 打开时加载：模板列表 / 网站列表 / API 配置 / 本地模型状态
  useEffect(() => {
    if (!open) return
    setPrompts(loadPrompts(language))
    setSites(loadAiWebSites())
    summaryGetConfig()
      .then(setApiConfig)
      .catch(() => setApiConfig(null))
    summaryLocalModels()
      .then((models) => {
        setLocalModels(models)
        // 默认选第一个已安装模型；之前选的若仍已安装则保留
        setLocalModelId((prev) =>
          prev && models.some((m) => m.id === prev && m.installed)
            ? prev
            : (models.find((m) => m.installed)?.id ?? '')
        )
      })
      .catch(() => setLocalModels([]))
  }, [open, language])

  // 模板切换时把该模板内容放进编辑器（用户后续编辑只留在组件 state）
  useEffect(() => {
    if (!open) return
    const preset = prompts.find((p) => p.id === promptId)
    if (preset) setPromptContent(preset.content)
  }, [open, promptId, prompts])

  const handlePromptChange = (id: string) => {
    if (onPromptChange) onPromptChange(id)
    else setInnerPromptId(id)
  }

  // 编辑器里显式保存：内置模板存为 override，自定义模板整体持久化
  const handleSavePrompt = () => {
    const next = loadPrompts(language).map((p) => (p.id === promptId ? { ...p, content: promptContent } : p))
    saveCustomPrompts(pickUserPrompts(next, language))
    setPrompts(next)
    setEditorOpen(false)
    toast.success(t.sumPresetSaved)
  }

  // 新增自定义模板：id 前缀 custom-，保存后选中它
  const handleAddPreset = () => {
    const name = newPresetName.trim()
    const content = newPresetContent.trim()
    if (!name || !content) return
    const preset: SummaryPromptPreset = { id: `custom-${crypto.randomUUID()}`, name, content }
    saveCustomPrompts([...loadUserPrompts(language), preset])
    setPrompts(loadPrompts(language))
    setNewPresetName('')
    setNewPresetContent('')
    setAddOpen(false)
    handlePromptChange(preset.id)
    toast.success(t.sumPresetSaved)
  }

  // 删除当前自定义模板并回退到默认模板
  const handleDeletePreset = () => {
    saveCustomPrompts(loadUserPrompts(language).filter((p) => p.id !== promptId))
    setPrompts(loadPrompts(language))
    setEditorOpen(false)
    handlePromptChange('default')
  }

  // 清除当前内置模板的用户覆盖，恢复默认内容
  const handleResetPreset = () => {
    resetBuiltinOverride(promptId, language)
    setPrompts(loadPrompts(language))
  }

  const currentPreset = prompts.find((p) => p.id === promptId)
  const isCustomPreset = !!currentPreset && !currentPreset.builtin
  const builtinOrig = currentPreset?.builtin
    ? getBuiltinPrompts(language).find((b) => b.id === promptId)
    : undefined
  const hasPresetOverride =
    !!builtinOrig &&
    !!currentPreset &&
    (builtinOrig.content !== currentPreset.content || builtinOrig.name !== currentPreset.name)

  const updateSites = (next: AiWebSite[]) => {
    setSites(next)
    saveAiWebSites(next)
  }

  const handleAddSite = () => {
    const name = newSiteName.trim()
    const url = newSiteUrl.trim()
    if (!name || !url) return
    updateSites([...sites, { id: crypto.randomUUID(), name, url }])
    setNewSiteName('')
    setNewSiteUrl('')
  }

  const handleCopyAndOpen = async (site: AiWebSite) => {
    try {
      await navigator.clipboard.writeText(composeFullPrompt(promptContent, clip.text))
      await openUrl(site.url)
      toast.success(t.sumCopiedFull)
    } catch {
      toast.error(t.comError)
    }
  }

  const installedLocalModels = useMemo(() => (localModels ?? []).filter((m) => m.installed), [localModels])

  const canGenerate =
    !!clip.text.trim() &&
    (method === 'api' ? !!apiConfig : method === 'local' ? installedLocalModels.length > 0 : false)

  // 生成：交给父组件打开结果面板，自己关闭
  const handleGenerate = () => {
    if (!canGenerate || method === 'web') return
    onGenerate({
      method,
      prompt: composeFullPrompt(promptContent, clip.text),
      apiConfig,
      localModelId: localModelId || undefined,
    })
    onOpenChange(false)
  }

  // tab 拖拽换位并持久化
  const handleDropTab = (target: MethodTab) => {
    const dragged = dragTabRef.current
    dragTabRef.current = null
    if (!dragged || dragged === target) return
    const next = methodOrder.filter((m) => m !== dragged)
    next.splice(next.indexOf(target), 0, dragged)
    setMethodOrder(next)
    try {
      localStorage.setItem(METHOD_ORDER_KEY, JSON.stringify(next))
    } catch {}
  }

  const tabLabels: Record<MethodTab, string> = {
    web: t.sumTabWeb,
    api: t.sumTabApi,
    local: t.sumTabLocal,
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="text-base">{t.sumDialogTitle}</DialogTitle>
        </DialogHeader>

        {/* 方式 tab（可拖拽换位，顺序持久化） */}
        <div className="shrink-0 flex gap-4 border-b border-border/60">
          {methodOrder.map((key) => (
            <button
              key={key}
              draggable
              onDragStart={() => {
                dragTabRef.current = key
              }}
              onDragOver={(e) => e.preventDefault()}
              onDrop={() => handleDropTab(key)}
              className={cn(
                'px-1 pb-2 text-xs font-medium border-b-2 -mb-px transition-colors cursor-grab active:cursor-grabbing',
                method === key
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
              onClick={() => setMethod(key)}
            >
              {tabLabels[key]}
            </button>
          ))}
        </div>

        {/* 模板选择 + 可折叠 prompt 编辑器（三种方式共用） */}
        <div className="shrink-0 flex items-center gap-2">
          <span className="shrink-0 text-xs text-muted-foreground">{t.sumPromptPreset}</span>
          <Select value={promptId} onValueChange={handlePromptChange}>
            <SelectTrigger className="h-8 flex-1 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {prompts.map((p) => (
                <SelectItem key={p.id} value={p.id} className="text-xs">
                  {p.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="ghost" size="sm" className="shrink-0" onClick={() => setEditorOpen((v) => !v)}>
            {t.sumPromptEdit}
          </Button>
          <Button variant="ghost" size="sm" className="shrink-0" onClick={() => setAddOpen((v) => !v)}>
            {t.sumAddPreset}
          </Button>
        </div>
        {editorOpen && (
          <div className="shrink-0">
            <textarea
              className="w-full min-h-[140px] rounded-md border border-input bg-background px-3 py-2 text-xs leading-relaxed shadow-sm focus:outline-none focus:ring-1 focus:ring-ring"
              value={promptContent}
              onChange={(e) => setPromptContent(e.target.value)}
            />
            <div className="mt-1.5 flex justify-end gap-2">
              {isCustomPreset && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-destructive hover:text-destructive hover:bg-destructive/10"
                  onClick={handleDeletePreset}
                >
                  {t.comDelete}
                </Button>
              )}
              {hasPresetOverride && (
                <Button variant="ghost" size="sm" onClick={handleResetPreset}>
                  {t.sumResetPreset}
                </Button>
              )}
              <Button variant="outline" size="sm" onClick={handleSavePrompt}>
                {t.comSave}
              </Button>
            </div>
          </div>
        )}
        {addOpen && (
          <div className="shrink-0 flex flex-col gap-1.5">
            <Input
              className="h-8 text-xs"
              value={newPresetName}
              placeholder={t.sumPresetName}
              onChange={(e) => setNewPresetName(e.target.value)}
            />
            <textarea
              className="w-full min-h-[100px] rounded-md border border-input bg-background px-3 py-2 text-xs leading-relaxed shadow-sm focus:outline-none focus:ring-1 focus:ring-ring"
              value={newPresetContent}
              onChange={(e) => setNewPresetContent(e.target.value)}
            />
            <div className="flex justify-end gap-2">
              <Button variant="ghost" size="sm" onClick={() => setAddOpen(false)}>
                {t.comCancel}
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={!newPresetName.trim() || !newPresetContent.trim()}
                onClick={handleAddPreset}
              >
                {t.comSave}
              </Button>
            </div>
          </div>
        )}

        {/* 各方式的内容区 */}
        <div className="shrink-0">
          {method === 'web' && (
            <div className="flex flex-col gap-2">
              <p className="text-xs text-muted-foreground">{t.sumWebHint}</p>
              <div className="max-h-[180px] overflow-y-auto custom-scrollbar flex flex-col gap-1">
                {sites.map((site) => (
                  <div
                    key={site.id}
                    className="flex items-center gap-2 rounded-md border border-border/60 px-2.5 py-1.5"
                  >
                    <div className="flex-1 min-w-0">
                      <div className="text-sm truncate">{site.name}</div>
                      <div className="text-[11px] text-muted-foreground truncate">{site.url}</div>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      className="shrink-0"
                      onClick={() => handleCopyAndOpen(site)}
                    >
                      {t.sumCopyAndOpen}
                    </Button>
                  </div>
                ))}
              </div>
              <button
                type="button"
                className="self-start text-xs text-primary hover:underline"
                onClick={() => setManageOpen((v) => !v)}
              >
                {t.sumSitesManage}
              </button>
              {manageOpen && (
                <div className="flex flex-col gap-1.5 rounded-md border border-border/60 p-2.5">
                  {sites.map((site) => (
                    <div key={site.id} className="flex items-center gap-1.5">
                      <Input
                        className="h-8 flex-1 text-xs"
                        value={site.name}
                        placeholder={t.sumSiteName}
                        onChange={(e) =>
                          updateSites(sites.map((s) => (s.id === site.id ? { ...s, name: e.target.value } : s)))
                        }
                      />
                      <Input
                        className="h-8 flex-[2] text-xs"
                        value={site.url}
                        placeholder={t.sumSiteUrl}
                        onChange={(e) =>
                          updateSites(sites.map((s) => (s.id === site.id ? { ...s, url: e.target.value } : s)))
                        }
                      />
                      {!site.builtin && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 shrink-0"
                          title={t.comDelete}
                          onClick={() => updateSites(sites.filter((s) => s.id !== site.id))}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                  ))}
                  <div className="flex items-center gap-1.5">
                    <Input
                      className="h-8 flex-1 text-xs"
                      value={newSiteName}
                      placeholder={t.sumSiteName}
                      onChange={(e) => setNewSiteName(e.target.value)}
                    />
                    <Input
                      className="h-8 flex-[2] text-xs"
                      value={newSiteUrl}
                      placeholder={t.sumSiteUrl}
                      onChange={(e) => setNewSiteUrl(e.target.value)}
                    />
                    <Button variant="outline" size="sm" className="shrink-0" onClick={handleAddSite}>
                      {t.sumAddSite}
                    </Button>
                  </div>
                  <div className="flex justify-end">
                    <Button variant="ghost" size="sm" onClick={() => setSites(resetAiWebSites())}>
                      {t.sumResetSites}
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}

          {method === 'api' &&
            (apiConfig ? (
              <div className="rounded-md border bg-muted/30 px-3 py-2 text-xs text-muted-foreground truncate">
                {apiConfig.endpoint} · {apiConfig.model}
              </div>
            ) : (
              <div className="flex items-center justify-between gap-2 rounded-md border border-primary/20 bg-primary/5 px-4 py-3">
                <span className="text-sm">{t.sumApiNotConfigured}</span>
                <Button variant="outline" size="sm" className="shrink-0" asChild>
                  <Link href="/settings">{t.sumGoSettings}</Link>
                </Button>
              </div>
            ))}

          {method === 'local' &&
            (localModels === null ? (
              <p className="text-xs text-muted-foreground">{t.comLoading}</p>
            ) : installedLocalModels.length === 0 ? (
              <div className="flex items-center justify-between gap-2 rounded-md border border-primary/20 bg-primary/5 px-4 py-3">
                <span className="text-sm">{t.sumLocalNotInstalled}</span>
                <Button variant="outline" size="sm" className="shrink-0" asChild>
                  <Link href="/settings">{t.sumDownloadInSettings}</Link>
                </Button>
              </div>
            ) : installedLocalModels.length === 1 ? (
              <div className="rounded-md border bg-muted/30 px-3 py-2 text-xs text-muted-foreground truncate">
                {t.sumLocalModel}
                {` · ${modelDisplayName(installedLocalModels[0].id, t, installedLocalModels[0].displayName)}`}
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <span className="shrink-0 text-xs text-muted-foreground">{t.sumLocalModel}</span>
                <Select value={localModelId} onValueChange={setLocalModelId}>
                  <SelectTrigger className="h-8 flex-1 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {installedLocalModels.map((m) => (
                      <SelectItem key={m.id} value={m.id} className="text-xs">
                        {modelDisplayName(m.id, t, m.displayName)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            ))}
        </div>

        {/* 超长截断提示 */}
        {clip.truncated && method !== 'web' && (
          <p className="shrink-0 text-[11px] text-muted-foreground">{t.sumTruncated}</p>
        )}

        {/* 底部操作 */}
        <div className="shrink-0 flex items-center justify-end gap-2">
          {method !== 'web' && (
            <Button size="sm" disabled={!canGenerate} onClick={handleGenerate}>
              {t.sumGenerate}
            </Button>
          )}
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t.comClose}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
