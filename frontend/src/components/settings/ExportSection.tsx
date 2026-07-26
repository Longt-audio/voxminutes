'use client'

import { useEffect, useState } from 'react'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import {
  getRecordingPreferences,
  setRecordingPreferences,
  selectRecordingFolder,
  openRecordingsFolder,
  apiGetSettings,
  apiSaveSetting,
} from '@/services/ipc'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingsSection } from './SettingsSection'
import { useMessages } from '@/i18n/useMessages'

/** 设置页 Section 3：录音保存目录 + 默认导出目录 */
export function ExportSection() {
  const t = useMessages()
  const [recordingsFolder, setRecordingsFolder] = useState('')
  const [exportDir, setExportDir] = useState('')

  useEffect(() => {
    getRecordingPreferences()
      .then((p) => setRecordingsFolder(p.recordingsFolder))
      .catch(() => {})
    apiGetSettings()
      .then((s) => setExportDir(s['export.default_dir'] || ''))
      .catch(() => {})
  }, [])

  const handleSelectRecordingsFolder = async () => {
    try {
      const folder = await selectRecordingFolder()
      if (!folder) return
      await setRecordingPreferences({ recordingsFolder: folder, autoSave: true })
      setRecordingsFolder(folder)
      toast.success(t.setFolderUpdated)
    } catch (e) {
      toast.error(t.setFolderFailed.replace('{error}', String(e)))
    }
  }

  const handleOpenRecordingsFolder = () => {
    openRecordingsFolder().catch((e) => toast.error(t.setOpenFolderFailed.replace('{error}', String(e))))
  }

  const handleSelectExportDir = async () => {
    try {
      const dir = await openDialog({ directory: true })
      if (typeof dir !== 'string' || !dir) return
      await apiSaveSetting('export.default_dir', dir)
      setExportDir(dir)
      toast.success(t.setExportDirUpdated)
    } catch (e) {
      toast.error(t.setExportDirFailed.replace('{error}', String(e)))
    }
  }

  return (
    <SettingsSection title={t.setRecordingExport}>
      {/* 录音保存目录 */}
      <div className="flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.setRecordingsFolder}</span>
        <div className="flex items-center gap-2">
          <Input className="flex-1 min-w-0" readOnly value={recordingsFolder} placeholder={t.comLoading} />
          <Button className="shrink-0" onClick={handleSelectRecordingsFolder}>
            {t.setChange}
          </Button>
          <Button variant="outline" className="shrink-0" onClick={handleOpenRecordingsFolder}>
            {t.setOpenFolder}
          </Button>
        </div>
      </div>

      {/* 默认导出目录 */}
      <div className="mt-4 flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">{t.setExportDir}</span>
        <div className="flex items-center gap-2">
          <Input className="flex-1 min-w-0" readOnly value={exportDir} placeholder={t.setExportDirUnset} />
          <Button className="shrink-0" onClick={handleSelectExportDir}>
            {t.setChange}
          </Button>
        </div>
      </div>
    </SettingsSection>
  )
}
