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
  /** 重识别触发按钮上方的小标题（离线重识别只用非流式模型） */
  histOfflineAsrModelLabel: string
  /** 页头里实时引擎的标签（区别于离线识别模型，避免混淆） */
  histRealtimeEngineLabel: string
  /** 离线识别完成后展示的音频时长标签 */
  histAudioDurationLabel: string
  /** 离线识别完成后展示的识别耗时标签 */
  histAsrElapsedLabel: string
  /** 另一条录音正在离线识别时的提示（切页面也能看到） */
  histRetranscribeOtherBusy: string
  /** 「复制文本」按钮：复制当前 tab 的语音识别结果 */
  histCopyTranscript: string
  histCopyTranscriptHint: string
  histCopyTranscriptDone: string
  histCopyTranscriptFailed: string
  /** 复制成功后的按钮瞬态文案 */
  histCopied: string
  /** 识别完成但有上游告警时的标题（{count} 为段数） */
  histRetranscribeDoneWithWarnings: string
  /** 离线 tab 里持久展示的上游告警小标题 */
  histOfflineWarningsTitle: string
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
  /** 说话人图例标签（离线识别说话人分离） */
  histSpeakerLegend: string
  /** 默认说话人名（{n} 为编号） */
  histSpeakerDefault: string
  /** 说话人重命名提示 */
  histSpeakerRename: string
  /** 离线识别进行中的「停止识别」按钮 */
  histRetranscribeStop: string
  /** 点了停止识别、等待后端中断的过渡态 */
  histRetranscribeStopping: string
  /** 已成功停止离线识别 */
  histRetranscribeStopped: string
  /** 远程文件转写（qwen/豆包录音文件识别）耗时提示 */
  histRetranscribeRemoteSlow: string
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
  histMerge: string
  histMergeSelected: string
  histMergeClear: string
  histMergeTitle: string
  histMergeNewTitle: string
  histMergeNewTitleHint: string
  histMergeDeleteSources: string
  histMergeConfirm: string
  histMergeInProgress: string
  histMergeSuccess: string
  histMergeFailed: string
  histMergeNeedTwo: string
  histMergeMarker: string
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
    histRetranscribe: 'Offline Recognition',
    histRetranscribing: 'Transcribing…',
    histRetranscribeModelTitle: 'Choose a model for offline recognition',
    histOfflineAsrModelLabel: 'Offline speech recognition model',
    histRealtimeEngineLabel: 'Live engine',
    histAudioDurationLabel: 'Audio length',
    histAsrElapsedLabel: 'Recognition time',
    histRetranscribeOtherBusy: 'Another recording is being transcribed offline',
    histCopyTranscript: 'Copy text',
    histCopyTranscriptHint: 'Copy the speech recognition result of this tab',
    histCopyTranscriptDone: 'Transcript copied',
    histCopyTranscriptFailed: 'Failed to copy',
    histCopied: 'Copied',
    histRetranscribeDoneWithWarnings: 'Transcription finished ({count} segments), but some audio was not recognized',
    histOfflineWarningsTitle: 'Recognition warnings',
    histModelSenseVoice: 'SenseVoice Multilingual',
    histRetranscribeNoFolder: 'Audio folder missing — cannot re-transcribe',
    histRetranscribeNoModel: 'Select a model first',
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
    histSpeakerLegend: 'Speakers',
    histSpeakerDefault: 'Speaker {n}',
    histSpeakerRename: 'Click a name to rename it',
    histRetranscribeStop: 'Stop',
    histRetranscribeStopping: 'Stopping…',
    histRetranscribeStopped: 'Re-transcription stopped',
    histRetranscribeRemoteSlow: 'Cloud file transcription takes about 1–2 minutes for long recordings. Keep waiting — or press “Stop” to abort.',
    histSegmentSaveFailed: 'Failed to save segment',
    histTabRealtime: 'Real-time',
    histTabOffline: 'Offline',
    histOfflineEmptyTitle: 'No offline results yet',
    histOfflineEmptyHint: 'Click "Offline Recognition" above to generate offline results.',
    histPendingTitle: 'Not transcribed yet',
    histPendingHint: 'This recording was imported from audio and has not been transcribed. Click "Offline Recognition" above to start.',
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
    histMerge: 'Merge',
    histMergeSelected: 'Merge selected ({n})',
    histMergeClear: 'Clear selection',
    histMergeTitle: 'Merge recordings',
    histMergeNewTitle: 'New title',
    histMergeNewTitleHint: 'Leave empty to use the default title',
    histMergeDeleteSources: 'Delete source recordings after merge',
    histMergeConfirm: 'Confirm merge',
    histMergeInProgress: 'Merging…',
    histMergeSuccess: 'Merge complete',
    histMergeFailed: 'Merge failed: {error}',
    histMergeNeedTwo: 'Select at least two recordings',
    histMergeMarker: '—— Part {n} "{title}", original start {time} ——',
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
    histRetranscribe: '离线识别',
    histRetranscribing: '识别中…',
    histRetranscribeModelTitle: '选择用于离线识别的模型',
    histOfflineAsrModelLabel: '离线语音识别模型',
    histRealtimeEngineLabel: '实时引擎',
    histAudioDurationLabel: '音频时长',
    histAsrElapsedLabel: '识别耗时',
    histRetranscribeOtherBusy: '另一条录音正在离线识别中',
    histCopyTranscript: '复制文本',
    histCopyTranscriptHint: '复制本 tab 的语音识别结果',
    histCopyTranscriptDone: '已复制识别结果',
    histCopyTranscriptFailed: '复制失败',
    histCopied: '已复制',
    histRetranscribeDoneWithWarnings: '识别完成（共 {count} 段），但部分音频未能识别',
    histOfflineWarningsTitle: '识别告警',
    histModelSenseVoice: 'SenseVoice 多语言',
    histRetranscribeNoFolder: '缺少音频目录，无法重新转写',
    histRetranscribeNoModel: '请先选择用于识别的模型',
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
    histSpeakerLegend: '说话人',
    histSpeakerDefault: '说话人{n}',
    histSpeakerRename: '点击名字可重命名',
    histRetranscribeStop: '停止识别',
    histRetranscribeStopping: '正在停止…',
    histRetranscribeStopped: '已停止识别',
    histRetranscribeRemoteSlow: '云端文件转写较慢（长录音约 1~2 分钟），请耐心等待；如需中断请点「停止识别」。',
    histSegmentSaveFailed: '保存片段失败',
    histTabRealtime: '实时识别',
    histTabOffline: '离线识别',
    histOfflineEmptyTitle: '暂无离线识别结果',
    histOfflineEmptyHint: '点击上方「离线识别」生成离线识别结果。',
    histPendingTitle: '尚未转写',
    histPendingHint: '该录音由音频导入，尚未转写。点击上方「离线识别」开始识别。',
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
    histMerge: '合并',
    histMergeSelected: '合并选中（{n}）',
    histMergeClear: '清除选择',
    histMergeTitle: '合并工程',
    histMergeNewTitle: '新工程标题',
    histMergeNewTitleHint: '留空则使用默认标题',
    histMergeDeleteSources: '合并后删除原工程',
    histMergeConfirm: '确认合并',
    histMergeInProgress: '合并中…',
    histMergeSuccess: '合并成功',
    histMergeFailed: '合并失败：{error}',
    histMergeNeedTwo: '请至少选择两个工程',
    histMergeMarker: '—— 第 {n} 段会议「{title}」，原开始时间 {time} ——',
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
    histRetranscribe: '오프라인 인식',
    histRetranscribing: '인식 중…',
    histRetranscribeModelTitle: '오프라인 인식에 사용할 모델 선택',
    histOfflineAsrModelLabel: '오프라인 음성 인식 모델',
    histRealtimeEngineLabel: '실시간 엔진',
    histAudioDurationLabel: '오디오 길이',
    histAsrElapsedLabel: '인식 소요 시간',
    histRetranscribeOtherBusy: '다른 녹음의 오프라인 인식이 진행 중입니다',
    histCopyTranscript: '텍스트 복사',
    histCopyTranscriptHint: '이 탭의 음성 인식 결과를 복사합니다',
    histCopyTranscriptDone: '인식 결과를 복사했습니다',
    histCopyTranscriptFailed: '복사 실패',
    histCopied: '복사됨',
    histRetranscribeDoneWithWarnings: '인식 완료({count}개 구간)했지만 일부 오디오는 인식되지 않았습니다',
    histOfflineWarningsTitle: '인식 경고',
    histModelSenseVoice: 'SenseVoice 다국어',
    histRetranscribeNoFolder: '오디오 폴더가 없어 다시 인식할 수 없습니다',
    histRetranscribeNoModel: '먼저 인식에 사용할 모델을 선택하세요',
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
    histSpeakerLegend: '화자',
    histSpeakerDefault: '화자 {n}',
    histSpeakerRename: '이름을 클릭해 변경하세요',
    histRetranscribeStop: '인식 중지',
    histRetranscribeStopping: '중지 중…',
    histRetranscribeStopped: '인식을 중지했습니다',
    histRetranscribeRemoteSlow: '클라우드 파일 인식은 오래 걸립니다(긴 녹음은 약 1~2분). 기다리거나 「인식 중지」를 누르세요.',
    histSegmentSaveFailed: '구간을 저장하지 못했습니다',
    histTabRealtime: '실시간 인식',
    histTabOffline: '오프라인 인식',
    histOfflineEmptyTitle: '오프라인 인식 결과가 없습니다',
    histOfflineEmptyHint: '위의 "오프라인 인식"을 클릭해 오프라인 인식 결과를 생성하세요.',
    histPendingTitle: '아직 받아쓰기되지 않았습니다',
    histPendingHint: '오디오 파일을 가져와 만든 녹음으로, 아직 받아쓰기되지 않았습니다. 위의 "오프라인 인식"을 클릭해 시작하세요.',
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
    histMerge: '병합',
    histMergeSelected: '선택 항목 병합({n})',
    histMergeClear: '선택 지우기',
    histMergeTitle: '녹음 병합',
    histMergeNewTitle: '새 제목',
    histMergeNewTitleHint: '비워 두면 기본 제목을 사용합니다',
    histMergeDeleteSources: '병합 후 원본 녹음 삭제',
    histMergeConfirm: '병합 확인',
    histMergeInProgress: '병합 중…',
    histMergeSuccess: '병합이 완료되었습니다',
    histMergeFailed: '병합에 실패했습니다: {error}',
    histMergeNeedTwo: '두 개 이상의 녹음을 선택하세요',
    histMergeMarker: '—— {n}번째 회의「{title}」, 원래 시작 시간 {time} ——',
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
    histRetranscribe: 'オフライン認識',
    histRetranscribing: '認識中…',
    histRetranscribeModelTitle: 'オフライン認識に使用するモデルを選択',
    histOfflineAsrModelLabel: 'オフライン音声認識モデル',
    histRealtimeEngineLabel: 'リアルタイムエンジン',
    histAudioDurationLabel: '音声の長さ',
    histAsrElapsedLabel: '認識にかかった時間',
    histRetranscribeOtherBusy: '別の録音をオフライン認識中です',
    histCopyTranscript: 'テキストをコピー',
    histCopyTranscriptHint: 'このタブの音声認識結果をコピーします',
    histCopyTranscriptDone: '認識結果をコピーしました',
    histCopyTranscriptFailed: 'コピーに失敗しました',
    histCopied: 'コピー済み',
    histRetranscribeDoneWithWarnings: '認識が完了しました（{count} 区間）。ただし一部の音声は認識できませんでした',
    histOfflineWarningsTitle: '認識の警告',
    histModelSenseVoice: 'SenseVoice 多言語',
    histRetranscribeNoFolder: '音声フォルダーが見つからないため、再認識できません',
    histRetranscribeNoModel: '先に認識に使用するモデルを選択してください',
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
    histSpeakerLegend: '話者',
    histSpeakerDefault: '話者{n}',
    histSpeakerRename: '名前をクリックして変更',
    histRetranscribeStop: '認識を停止',
    histRetranscribeStopping: '停止中…',
    histRetranscribeStopped: '認識を停止しました',
    histRetranscribeRemoteSlow: 'クラウドのファイル認識は時間がかかります（長い録音で約1〜2分）。「認識を停止」で中断できます。',
    histSegmentSaveFailed: 'セグメントを保存できませんでした',
    histTabRealtime: 'リアルタイム認識',
    histTabOffline: 'オフライン認識',
    histOfflineEmptyTitle: 'オフライン認識の結果はまだありません',
    histOfflineEmptyHint: '上の「オフライン認識」をクリックすると、オフライン認識の結果を生成します。',
    histPendingTitle: 'まだ文字起こしされていません',
    histPendingHint: 'この録音は音声のインポートで作成されたため、まだ文字起こしされていません。上の「オフライン認識」をクリックして開始してください。',
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
    histMerge: '結合',
    histMergeSelected: '選択した録音を結合（{n}）',
    histMergeClear: '選択をクリア',
    histMergeTitle: '録音の結合',
    histMergeNewTitle: '新しいタイトル',
    histMergeNewTitleHint: '空欄の場合はデフォルトのタイトルを使用します',
    histMergeDeleteSources: '結合後に元の録音を削除',
    histMergeConfirm: '結合を実行',
    histMergeInProgress: '結合中…',
    histMergeSuccess: '結合が完了しました',
    histMergeFailed: '結合に失敗しました: {error}',
    histMergeNeedTwo: '2つ以上の録音を選択してください',
    histMergeMarker: '—— 第 {n} 部「{title}」、元の開始時刻 {time} ——',
  },
}
