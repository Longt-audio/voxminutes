'use client'

import type { ReactNode } from 'react'

/** 设置页分组卡片：统一标题 + 可选描述 + 内容布局 */
export function SettingsSection({
  title,
  description,
  children,
}: {
  title: string
  description?: string
  children: ReactNode
}) {
  return (
    <section className="rounded-lg border bg-card text-card-foreground shadow-sm p-5">
      <h2 className="text-base font-semibold mb-4">{title}</h2>
      {description && <p className="-mt-2 mb-4 text-xs text-muted-foreground">{description}</p>}
      {children}
    </section>
  )
}
