import { marked } from 'marked'
import { invoke } from '@tauri-apps/api/core'

/**
 * 会议总结 PDF 导出（打印绕道方案）：
 * Markdown 在主窗口用本地 marked 渲染成 HTML → Rust 打开 /print-summary.html
 * 预览窗口并弹出系统打印对话框，用户选「存为 PDF」。
 *
 * 选型背景（2026-09-25 调研）：Tauri 无静默生成 PDF 能力；jspdf/pdfmake 要嵌中文字体
 * 子集且分页弱；typst 会让安装包 +10~25MB。打印绕道零体积、中文排版满分。
 * 注意别退回 mig 的缺陷做法：marked 走 CDN（离线打印出空白页）/ 甩给外部浏览器。
 */

/** LLM 产出的 Markdown 不可信：去掉可执行/嵌外联内容（打印窗口有 invoke 权限）。 */
function sanitizeHtml(html: string): string {
  return html
    .replace(/<\s*(script|iframe|object|embed|form|link|meta)[^>]*>[\s\S]*?<\s*\/\s*\1\s*>/gi, '')
    .replace(/<\s*(script|iframe|object|embed|form|link|meta)[^>]*\/?>/gi, '')
    .replace(/\son\w+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, '')
    .replace(/(href|src)\s*=\s*(["'])\s*javascript:[^"']*\2/gi, '$1=$2#$2')
}

/** Markdown → 打印页正文 HTML（GFM：表格/删除线/任务列表；单换行转 <br>）。 */
export function summaryMarkdownToHtml(md: string): string {
  const raw = marked.parse(md, { async: false, gfm: true, breaks: true })
  return sanitizeHtml(raw)
}

/** 打开总结打印预览窗口（标题为空时窗口标题由 Rust 兜底）。 */
export async function openSummaryPrintWindow(title: string, markdown: string): Promise<void> {
  const html = summaryMarkdownToHtml(markdown)
  await invoke('open_summary_print_window', { title, html })
}
