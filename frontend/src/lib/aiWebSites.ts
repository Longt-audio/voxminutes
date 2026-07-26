export interface AiWebSite {
  id: string
  name: string
  url: string
  builtin?: boolean
}

const STORAGE_KEY = 'voxminutes-ai-web-sites'

// 内置 AI 聊天网站列表；中文站点名为品牌名，各语言下保持中文不变
const BUILTIN_AI_WEB_SITES: AiWebSite[] = [
  { id: 'deepseek', name: 'DeepSeek', url: 'https://chat.deepseek.com', builtin: true },
  { id: 'qwen', name: '通义千问', url: 'https://www.tongyi.com', builtin: true },
  { id: 'kimi', name: 'Kimi', url: 'https://www.kimi.com', builtin: true },
  { id: 'doubao', name: '豆包', url: 'https://www.doubao.com', builtin: true },
  { id: 'chatgpt', name: 'ChatGPT', url: 'https://chat.openai.com', builtin: true },
  { id: 'gemini', name: 'Gemini', url: 'https://gemini.google.com', builtin: true },
  { id: 'claude', name: 'Claude', url: 'https://claude.ai', builtin: true },
  { id: 'copilot', name: 'Copilot', url: 'https://copilot.microsoft.com', builtin: true },
]

function getBuiltinSites(): AiWebSite[] {
  return BUILTIN_AI_WEB_SITES.map((site) => ({ ...site }))
}

/** 用户自定义列表（localStorage），无有效数据时回退到内置列表。 */
export function loadAiWebSites(): AiWebSite[] {
  if (typeof window === 'undefined') return getBuiltinSites()
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as AiWebSite[]
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed
      }
    }
  } catch { /* ignore */ }
  return getBuiltinSites()
}

export function saveAiWebSites(sites: AiWebSite[]): void {
  if (typeof window === 'undefined') return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(sites))
}

/** 恢复内置列表并持久化。 */
export function resetAiWebSites(): AiWebSite[] {
  const sites = getBuiltinSites()
  saveAiWebSites(sites)
  return sites
}
