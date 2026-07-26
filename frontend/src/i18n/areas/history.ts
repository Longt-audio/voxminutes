import type { Language } from '../languages'

/** 历史记录页文案：页头、状态/来源标签、列表、详情、重新转写、导入导出。 */
export interface HistoryMessages {
  histPageTitle: string
  histPageSubtitle: string
  histLoadListFailed: string
  histLoadDetailFailed: string
  histStatusCompleted: string
  histStatusPending: string
  histStatusProcessing: string
  histStatusFailed: string
  histSourceImport: string
  histSourceRecord: string
  histSourceUnknown: string
  histMetaDuration: string
  histEditTitle: string
  histTitleSaved: string
  histSaveTitleFailed: string
  histDeleteConfirm: string
  histDeleted: string
  histDeleteFailed: string
  histRetranscribe: string
  histRetranscribing: string
  histRetranscribeModelTitle: string
  histModelSenseVoice: string
  histRetranscribeNoFolder: string
  histRetranscribeNoModel: string
  histRetranscribeStartFailed: string
  histRetranscribeDone: string
  histRetranscribeFailed: string
  histNoSelection: string
  histNoSelectionHint: string
  histListCount: string
  histListEmptyTitle: string
  histListEmptyHint: string
  histFileMissing: string
  histOpenFolder: string
  histOpenFolderFailed: string
  histSegmentSaved: string
  histSegmentSaveFailed: string
  histTabRealtime: string
  histTabOffline: string
  histOfflineEmptyTitle: string
  histOfflineEmptyHint: string
  histPendingTitle: string
  histPendingHint: string
  histTranscriptEmpty: string
  histDoubleClickEdit: string
  histExported: string
  histExportFailed: string
  histExporting: string
  histExportFormat: string
  histImportAudio: string
  histImporting: string
  histImportDone: string
  histImportFailed: string
  histImportSelectFailed: string
  histImportStartFailed: string
}

export const HISTORY_MESSAGES: Record<Language, HistoryMessages> = {
  en: {
    histPageTitle: 'History',
    histPageSubtitle: 'Manage recordings and transcripts',
    histLoadListFailed: 'Failed to load recordings',
    histLoadDetailFailed: 'Failed to load recording details',
    histStatusCompleted: 'Completed',
    histStatusPending: 'Pending',
    histStatusProcessing: 'Transcribing',
    histStatusFailed: 'Failed',
    histSourceImport: 'Imported audio',
    histSourceRecord: 'Live recording',
    histSourceUnknown: 'Unknown source',
    histMetaDuration: 'Duration {duration}',
    histEditTitle: 'Edit title',
    histTitleSaved: 'Title saved',
    histSaveTitleFailed: 'Failed to save title',
    histDeleteConfirm: 'Delete "{title}"? This cannot be undone.',
    histDeleted: 'Deleted',
    histDeleteFailed: 'Failed to delete',
    histRetranscribe: 'Re-transcribe',
    histRetranscribing: 'Transcribing…',
    histRetranscribeModelTitle: 'Choose an offline model for re-transcription',
    histModelSenseVoice: 'SenseVoice Multilingual',
    histRetranscribeNoFolder: 'Audio folder missing — cannot re-transcribe',
    histRetranscribeNoModel: 'Select an offline model first',
    histRetranscribeStartFailed: 'Failed to start re-transcription',
    histRetranscribeDone: 'Re-transcription complete — {count} segments',
    histRetranscribeFailed: 'Re-transcription failed: {error}',
    histNoSelection: 'No recording selected',
    histNoSelectionHint: 'Select a recording from the list on the left to view its transcript',
    histListCount: '{count} recordings',
    histListEmptyTitle: 'No recordings yet',
    histListEmptyHint: 'Start recording on the home page, or click "Import audio" below',
    histFileMissing: 'File missing',
    histOpenFolder: 'Open folder',
    histOpenFolderFailed: 'Failed to open folder',
    histSegmentSaved: 'Segment saved',
    histSegmentSaveFailed: 'Failed to save segment',
    histTabRealtime: 'Real-time',
    histTabOffline: 'Offline',
    histOfflineEmptyTitle: 'No offline results yet',
    histOfflineEmptyHint: 'Click "Re-transcribe" above to generate offline results.',
    histPendingTitle: 'Not transcribed yet',
    histPendingHint: 'This recording was imported from audio and has not been transcribed. Click "Re-transcribe" above to start.',
    histTranscriptEmpty: 'No transcript yet',
    histDoubleClickEdit: 'Double-click to edit',
    histExported: 'Exported: {path}',
    histExportFailed: 'Export failed',
    histExporting: 'Exporting…',
    histExportFormat: 'Export {format}',
    histImportAudio: 'Import audio',
    histImporting: 'Importing…',
    histImportDone: 'Import complete: {title}',
    histImportFailed: 'Import failed: {error}',
    histImportSelectFailed: 'Failed to select an audio file',
    histImportStartFailed: 'Failed to start import',
  },
  zh: {
    histPageTitle: '历史记录',
    histPageSubtitle: '录音与转写管理',
    histLoadListFailed: '加载录音列表失败',
    histLoadDetailFailed: '加载录音详情失败',
    histStatusCompleted: '已完成',
    histStatusPending: '待转写',
    histStatusProcessing: '转写中',
    histStatusFailed: '失败',
    histSourceImport: '导入音频',
    histSourceRecord: '实时录音',
    histSourceUnknown: '未知来源',
    histMetaDuration: '时长 {duration}',
    histEditTitle: '编辑标题',
    histTitleSaved: '标题已保存',
    histSaveTitleFailed: '保存标题失败',
    histDeleteConfirm: '确定删除「{title}」吗？此操作不可撤销。',
    histDeleted: '已删除',
    histDeleteFailed: '删除失败',
    histRetranscribe: '再次优化识别',
    histRetranscribing: '识别中…',
    histRetranscribeModelTitle: '选择用于再次优化识别的离线模型',
    histModelSenseVoice: 'SenseVoice 多语言',
    histRetranscribeNoFolder: '缺少音频目录，无法重新转写',
    histRetranscribeNoModel: '请先选择用于识别的离线模型',
    histRetranscribeStartFailed: '启动重新转写失败',
    histRetranscribeDone: '重新转写完成，共 {count} 段',
    histRetranscribeFailed: '重新转写失败：{error}',
    histNoSelection: '未选择录音',
    histNoSelectionHint: '从左侧列表选择一条录音，查看转写详情',
    histListCount: '共 {count} 条录音',
    histListEmptyTitle: '暂无录音记录',
    histListEmptyHint: '回到首页开始录音，或点击下方「导入音频」',
    histFileMissing: '文件缺失',
    histOpenFolder: '打开文件夹',
    histOpenFolderFailed: '打开文件夹失败',
    histSegmentSaved: '片段已保存',
    histSegmentSaveFailed: '保存片段失败',
    histTabRealtime: '实时识别',
    histTabOffline: '离线识别',
    histOfflineEmptyTitle: '暂无离线识别结果',
    histOfflineEmptyHint: '点击上方「再次优化识别」生成离线识别结果。',
    histPendingTitle: '尚未转写',
    histPendingHint: '该录音由音频导入，尚未转写。点击上方「再次优化识别」开始识别。',
    histTranscriptEmpty: '暂无转写内容',
    histDoubleClickEdit: '双击编辑',
    histExported: '已导出：{path}',
    histExportFailed: '导出失败',
    histExporting: '导出中…',
    histExportFormat: '导出 {format}',
    histImportAudio: '导入音频',
    histImporting: '导入中…',
    histImportDone: '导入完成：{title}',
    histImportFailed: '导入失败：{error}',
    histImportSelectFailed: '选择音频文件失败',
    histImportStartFailed: '启动导入失败',
  },
  ko: {
    histPageTitle: '기록',
    histPageSubtitle: '녹음 및 받아쓰기 관리',
    histLoadListFailed: '녹음 목록을 불러오지 못했습니다',
    histLoadDetailFailed: '녹음 상세 정보를 불러오지 못했습니다',
    histStatusCompleted: '완료됨',
    histStatusPending: '받아쓰기 대기',
    histStatusProcessing: '받아쓰기 중',
    histStatusFailed: '실패',
    histSourceImport: '가져온 오디오',
    histSourceRecord: '실시간 녹음',
    histSourceUnknown: '알 수 없는 소스',
    histMetaDuration: '길이 {duration}',
    histEditTitle: '제목 편집',
    histTitleSaved: '제목이 저장되었습니다',
    histSaveTitleFailed: '제목을 저장하지 못했습니다',
    histDeleteConfirm: '"{title}"을(를) 삭제하시겠습니까? 이 작업은 되돌릴 수 없습니다.',
    histDeleted: '삭제되었습니다',
    histDeleteFailed: '삭제하지 못했습니다',
    histRetranscribe: '다시 인식',
    histRetranscribing: '인식 중…',
    histRetranscribeModelTitle: '다시 인식에 사용할 오프라인 모델 선택',
    histModelSenseVoice: 'SenseVoice 다국어',
    histRetranscribeNoFolder: '오디오 폴더가 없어 다시 인식할 수 없습니다',
    histRetranscribeNoModel: '먼저 인식에 사용할 오프라인 모델을 선택하세요',
    histRetranscribeStartFailed: '다시 인식을 시작하지 못했습니다',
    histRetranscribeDone: '다시 인식이 완료되었습니다. 총 {count}개 구간',
    histRetranscribeFailed: '다시 인식에 실패했습니다: {error}',
    histNoSelection: '선택된 녹음 없음',
    histNoSelectionHint: '왼쪽 목록에서 녹음을 선택하면 받아쓰기 내용을 볼 수 있습니다',
    histListCount: '녹음 {count}개',
    histListEmptyTitle: '녹음 기록이 없습니다',
    histListEmptyHint: '홈에서 녹음을 시작하거나 아래의 "오디오 가져오기"를 클릭하세요',
    histFileMissing: '파일 없음',
    histOpenFolder: '폴더 열기',
    histOpenFolderFailed: '폴더를 열지 못했습니다',
    histSegmentSaved: '구간이 저장되었습니다',
    histSegmentSaveFailed: '구간을 저장하지 못했습니다',
    histTabRealtime: '실시간 인식',
    histTabOffline: '오프라인 인식',
    histOfflineEmptyTitle: '오프라인 인식 결과가 없습니다',
    histOfflineEmptyHint: '위의 "다시 인식"을 클릭해 오프라인 인식 결과를 생성하세요.',
    histPendingTitle: '아직 받아쓰기되지 않았습니다',
    histPendingHint: '오디오 파일을 가져와 만든 녹음으로, 아직 받아쓰기되지 않았습니다. 위의 "다시 인식"을 클릭해 시작하세요.',
    histTranscriptEmpty: '받아쓰기 내용이 없습니다',
    histDoubleClickEdit: '더블 클릭하여 편집',
    histExported: '내보내기 완료: {path}',
    histExportFailed: '내보내기에 실패했습니다',
    histExporting: '내보내는 중…',
    histExportFormat: '{format} 내보내기',
    histImportAudio: '오디오 가져오기',
    histImporting: '가져오는 중…',
    histImportDone: '가져오기 완료: {title}',
    histImportFailed: '가져오기에 실패했습니다: {error}',
    histImportSelectFailed: '오디오 파일을 선택하지 못했습니다',
    histImportStartFailed: '가져오기를 시작하지 못했습니다',
  },
  ja: {
    histPageTitle: '履歴',
    histPageSubtitle: '録音と文字起こしの管理',
    histLoadListFailed: '録音リストを読み込めませんでした',
    histLoadDetailFailed: '録音の詳細を読み込めませんでした',
    histStatusCompleted: '完了',
    histStatusPending: '未文字起こし',
    histStatusProcessing: '文字起こし中',
    histStatusFailed: '失敗',
    histSourceImport: 'インポートした音声',
    histSourceRecord: 'リアルタイム録音',
    histSourceUnknown: '不明なソース',
    histMetaDuration: '録音時間 {duration}',
    histEditTitle: 'タイトルを編集',
    histTitleSaved: 'タイトルを保存しました',
    histSaveTitleFailed: 'タイトルを保存できませんでした',
    histDeleteConfirm: '「{title}」を削除しますか？この操作は元に戻せません。',
    histDeleted: '削除しました',
    histDeleteFailed: '削除できませんでした',
    histRetranscribe: '再認識',
    histRetranscribing: '認識中…',
    histRetranscribeModelTitle: '再認識に使用するオフラインモデルを選択',
    histModelSenseVoice: 'SenseVoice 多言語',
    histRetranscribeNoFolder: '音声フォルダーが見つからないため、再認識できません',
    histRetranscribeNoModel: '先に認識に使用するオフラインモデルを選択してください',
    histRetranscribeStartFailed: '再認識を開始できませんでした',
    histRetranscribeDone: '再認識が完了しました（{count} セグメント）',
    histRetranscribeFailed: '再認識に失敗しました: {error}',
    histNoSelection: '録音が選択されていません',
    histNoSelectionHint: '左のリストから録音を選択すると、文字起こしの詳細を表示します',
    histListCount: '録音 {count}件',
    histListEmptyTitle: '録音はまだありません',
    histListEmptyHint: 'ホームで録音を開始するか、下の「音声をインポート」をクリックしてください',
    histFileMissing: 'ファイルなし',
    histOpenFolder: 'フォルダーを開く',
    histOpenFolderFailed: 'フォルダーを開けませんでした',
    histSegmentSaved: 'セグメントを保存しました',
    histSegmentSaveFailed: 'セグメントを保存できませんでした',
    histTabRealtime: 'リアルタイム認識',
    histTabOffline: 'オフライン認識',
    histOfflineEmptyTitle: 'オフライン認識の結果はまだありません',
    histOfflineEmptyHint: '上の「再認識」をクリックすると、オフライン認識の結果を生成します。',
    histPendingTitle: 'まだ文字起こしされていません',
    histPendingHint: 'この録音は音声のインポートで作成されたため、まだ文字起こしされていません。上の「再認識」をクリックして開始してください。',
    histTranscriptEmpty: '文字起こし内容はまだありません',
    histDoubleClickEdit: 'ダブルクリックで編集',
    histExported: 'エクスポートしました: {path}',
    histExportFailed: 'エクスポートに失敗しました',
    histExporting: 'エクスポート中…',
    histExportFormat: '{format} をエクスポート',
    histImportAudio: '音声をインポート',
    histImporting: 'インポート中…',
    histImportDone: 'インポート完了: {title}',
    histImportFailed: 'インポートに失敗しました: {error}',
    histImportSelectFailed: '音声ファイルを選択できませんでした',
    histImportStartFailed: 'インポートを開始できませんでした',
  },
}
