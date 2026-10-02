use anyhow::Result;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;

/// 增量写盘合并窗口（2026-09-29）。
///
/// 背景：`transcripts.json` 原来**每一个** `transcript-update`（含 partial）就全量重写
/// 一次，而且发生在 Tauri 事件监听器（同步）与 `RECORDING_MANAGER` 锁内。实测一场
/// 16 分钟录音写盘 **870 次 / 约 22MB**，1 小时录音单次快照就有 ~210KB。
///
/// 现在改为「后台线程合并写」：调用方只做一次 `try_send`（O(1)、不持锁、不碰磁盘），
/// 真正的序列化 + 写临时文件 + rename 全部在独立线程里，并把窗口期内的多次变更合并成
/// 一次写。崩溃时最多丢这一个窗口的内容（停止录音的最终写盘仍走同步路径，不受影响）。
const TRANSCRIPT_WRITE_COALESCE_MS: u64 = 1500;

/// 串行化 transcripts.json 的写盘：后台合并写与停止时的最终写共用同一把锁，
/// 否则两者会同时写 `.transcripts.json.tmp` 互相踩。
static TRANSCRIPT_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// 测试用：真实落盘次数（验证「合并写」确实把 N 次变更压成了少数几次写）。
#[cfg(test)]
pub(crate) static TRANSCRIPT_WRITE_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// 后台合并写线程的句柄。
///
/// 生命周期：`RecordingSaver::initialize_meeting_folder` 时启动（此时会议目录已知），
/// 停止录音时 `shutdown()`（先停后台写，再做带译文的最终写，避免旧快照覆盖新文件），
/// 若整个 saver 被丢弃则 sender 随之析构，线程自然退出。
struct TranscriptIncrementalWriter {
    tx: Option<std::sync::mpsc::Sender<()>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl TranscriptIncrementalWriter {
    fn start(folder: PathBuf, segments: Arc<Mutex<Vec<TranscriptSegment>>>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let handle = std::thread::Builder::new()
            .name("transcript-json-writer".to_string())
            .spawn(move || {
                // recv() 收到一次「脏」信号后先等一个合并窗口，再把窗口内的信号全部吸收，
                // 然后写一次最新快照。sender 全部析构后 recv() 返回 Err，线程退出。
                while rx.recv().is_ok() {
                    std::thread::sleep(std::time::Duration::from_millis(
                        TRANSCRIPT_WRITE_COALESCE_MS,
                    ));
                    while rx.try_recv().is_ok() {}
                    let snapshot = match segments.lock() {
                        Ok(guard) => guard.clone(),
                        Err(e) => {
                            warn!("增量写 transcripts.json：段落锁中毒，跳过本次写盘: {}", e);
                            continue;
                        }
                    };
                    if let Err(e) = write_transcripts_json_snapshot(&folder, &snapshot) {
                        warn!("增量写 transcripts.json 失败: {}", e);
                    }
                }
            })
            .ok();
        Self {
            tx: Some(tx),
            handle,
        }
    }

    /// 标记「有变更待写」。非阻塞、不碰磁盘；后台线程会合并写。
    fn mark_dirty(&self) {
        if let Some(tx) = &self.tx {
            // send 到无界通道且消费端只做 try_recv 排空，不会阻塞、
            // 也不会因为消费端忙而堆积（每轮都排空）。
            let _ = tx.send(());
        }
    }

    /// 停止后台写线程，并等待可能在飞的写盘结束（保证随后可以安全地做最终写）。
    fn shutdown(&mut self) {
        self.tx = None; // 析构 sender → recv() 返回 Err → 线程退出
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// 把一份段落快照原子写入 `transcripts.json`（temp + rename）。
///
/// 所有写盘（后台合并写 / 停止时的最终写 / auto-save 关闭时的补写）都必须经过这里，
/// 由 `TRANSCRIPT_WRITE_LOCK` 串行化：两者共用同一个 `.transcripts.json.tmp`，
/// 并发写会互相踩。成功只记 debug（后台写频率高，避免刷日志）。
fn write_transcripts_json_snapshot(folder: &Path, segments: &[TranscriptSegment]) -> Result<()> {
    let _serialize_guard = TRANSCRIPT_WRITE_LOCK.lock();
    #[cfg(test)]
    TRANSCRIPT_WRITE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let transcript_path = folder.join("transcripts.json");
    let temp_path = folder.join(".transcripts.json.tmp");

    let json = serde_json::json!({
        "version": "1.0",
        "segments": segments,
        "last_updated": chrono::Utc::now().to_rfc3339(),
        "total_segments": segments.len()
    });

    let json_string = serde_json::to_string_pretty(&json).map_err(|e| {
        error!("Failed to serialize transcripts to JSON: {}", e);
        anyhow::anyhow!("JSON serialization failed: {}", e)
    })?;

    std::fs::write(&temp_path, &json_string).map_err(|e| {
        error!(
            "Failed to write transcript temp file to {}: {}",
            temp_path.display(),
            e
        );
        anyhow::anyhow!("Failed to write temp file: {}", e)
    })?;

    if !temp_path.exists() {
        error!(
            "Temp transcript file does not exist after write: {}",
            temp_path.display()
        );
        return Err(anyhow::anyhow!("Temp file verification failed"));
    }

    std::fs::rename(&temp_path, &transcript_path).map_err(|e| {
        error!(
            "Failed to rename transcript file from {} to {}: {}",
            temp_path.display(),
            transcript_path.display(),
            e
        );
        anyhow::anyhow!("Failed to rename transcript file: {}", e)
    })?;

    log::debug!(
        "增量写 transcripts.json：{} 段（后台合并写）",
        segments.len()
    );
    Ok(())
}

use super::audio_processing::create_meeting_folder;
use super::incremental_saver::IncrementalAudioSaver;
use super::recording_state::AudioChunk;

/// Structured transcript segment for JSON export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub text: String,
    pub audio_start_time: f64, // Seconds from recording start
    pub audio_end_time: f64,   // Seconds from recording start
    pub duration: f64,         // Segment duration in seconds
    pub display_time: String,  // Formatted time for display like "[02:15]"
    pub confidence: f32,
    pub sequence_id: u64,
    /// 段落最终译文（录音停止、最终写盘时由翻译模块回填；增量写期间为空串）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub translation: String,
}

/// Meeting metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingMetadata {
    pub version: String,
    pub meeting_id: Option<String>,
    pub meeting_name: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub devices: DeviceInfo,
    pub audio_file: String,
    pub transcript_file: String,
    pub sample_rate: u32,
    pub status: String, // "recording", "completed", "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub microphone: Option<String>,
    pub system_audio: Option<String>,
}

/// New recording saver using incremental saving strategy
pub struct RecordingSaver {
    incremental_saver: Option<Arc<AsyncMutex<IncrementalAudioSaver>>>,
    meeting_folder: Option<PathBuf>,
    meeting_name: Option<String>,
    metadata: Option<MeetingMetadata>,
    transcript_segments: Arc<Mutex<Vec<TranscriptSegment>>>,
    chunk_receiver: Option<mpsc::UnboundedReceiver<AudioChunk>>,
    is_saving: Arc<Mutex<bool>>,
    /// 增量写盘的后台合并写线程（见 `TRANSCRIPT_WRITE_COALESCE_MS`）。
    /// 会议目录确定后启动；停止录音时先 shutdown 再做最终写。
    transcript_writer: Option<TranscriptIncrementalWriter>,
}

impl RecordingSaver {
    pub fn new() -> Self {
        Self {
            incremental_saver: None,
            meeting_folder: None,
            meeting_name: None,
            metadata: None,
            transcript_segments: Arc::new(Mutex::new(Vec::new())),
            chunk_receiver: None,
            is_saving: Arc::new(Mutex::new(false)),
            transcript_writer: None,
        }
    }

    /// Set the meeting name for this recording session
    pub fn set_meeting_name(&mut self, name: Option<String>) {
        self.meeting_name = name;
    }

    /// Set device information in metadata
    pub fn set_device_info(&mut self, mic_name: Option<String>, sys_name: Option<String>) {
        if let Some(ref mut metadata) = self.metadata {
            metadata.devices.microphone = mic_name;
            metadata.devices.system_audio = sys_name;

            // Write updated metadata to disk if folder exists
            if let Some(folder) = &self.meeting_folder {
                let metadata_clone = metadata.clone();
                if let Err(e) = self.write_metadata(folder, &metadata_clone) {
                    warn!("Failed to update metadata with device info: {}", e);
                }
            }
        }
    }

    /// Add or update a structured transcript segment (upserts based on sequence_id)
    /// Also saves incrementally to disk
    pub fn add_transcript_segment(&self, segment: TranscriptSegment) {
        if let Ok(mut segments) = self.transcript_segments.lock() {
            // Check if segment with same sequence_id exists (update it)
            if let Some(existing) = segments
                .iter_mut()
                .find(|s| s.sequence_id == segment.sequence_id)
            {
                *existing = segment.clone();
                info!(
                    "Updated transcript segment {} (seq: {}) - total segments: {}",
                    segment.id,
                    segment.sequence_id,
                    segments.len()
                );
            } else {
                // New segment, add it
                segments.push(segment.clone());
                info!(
                    "Added new transcript segment {} (seq: {}) - total segments: {}",
                    segment.id,
                    segment.sequence_id,
                    segments.len()
                );
            }
        } else {
            error!(
                "Failed to lock transcript segments for adding segment {}",
                segment.id
            );
        }

        // 增量落盘：只标记「脏」，真正的全量快照由后台线程合并写（见
        // TRANSCRIPT_WRITE_COALESCE_MS 的说明）。这里不再做任何磁盘 I/O——
        // 原来在事件监听器里同步全量重写，会把转写管线与 RECORDING_MANAGER 锁一起拖住。
        if let Some(writer) = &self.transcript_writer {
            writer.mark_dirty();
        } else if let Some(folder) = &self.meeting_folder {
            // 兜底：会议目录已就绪但写线程没起来（理论上不会发生），保持旧行为。
            if let Err(e) = self.write_transcripts_json(folder) {
                warn!("Failed to write incremental transcript update: {}", e);
            }
        }
    }

    /// Legacy method for backward compatibility - converts text to basic segment
    pub fn add_transcript_chunk(&self, text: String) {
        let segment = TranscriptSegment {
            id: format!("seg_{}", chrono::Utc::now().timestamp_millis()),
            text,
            audio_start_time: 0.0,
            audio_end_time: 0.0,
            duration: 0.0,
            display_time: "[00:00]".to_string(),
            confidence: 1.0,
            sequence_id: 0,
            translation: String::new(),
        };
        self.add_transcript_segment(segment);
    }

    /// Start accumulation with optional incremental saving
    ///
    /// # Arguments
    /// * `auto_save` - If true, creates checkpoints and enables saving. If false, audio chunks are discarded.
    /// * `base_folder` - Base directory under which the meeting folder is created
    ///   (the user's configured recordings folder).
    ///
    /// Meeting-folder initialization failure is returned as an error instead of
    /// being logged and silently dropped: with the old behavior recording would
    /// start "successfully" but nothing was ever written to disk.
    pub fn start_accumulation(
        &mut self,
        auto_save: bool,
        base_folder: PathBuf,
    ) -> Result<mpsc::UnboundedSender<AudioChunk>> {
        if auto_save {
            info!("Initializing incremental audio saver for recording (auto-save ENABLED)");
        } else {
            info!(
                "Starting recording without audio saving (auto-save DISABLED - transcripts only)"
            );
        }

        // Create channel for receiving audio chunks
        let (sender, receiver) = mpsc::unbounded_channel::<AudioChunk>();
        self.chunk_receiver = Some(receiver);

        // Initialize meeting folder and incremental saver ONLY if auto_save is enabled
        if auto_save {
            if let Some(name) = self.meeting_name.clone() {
                self.initialize_meeting_folder(&base_folder, &name, true)?;
                info!("Successfully initialized meeting folder with checkpoints");
            }
        } else {
            // When auto_save is false, still create meeting folder for transcripts/metadata
            // but skip .checkpoints directory
            if let Some(name) = self.meeting_name.clone() {
                self.initialize_meeting_folder(&base_folder, &name, false)?;
                info!("Successfully initialized meeting folder (transcripts only)");
            }
        }

        // Start accumulation task
        let is_saving_clone = self.is_saving.clone();
        let incremental_saver_arc = self.incremental_saver.clone();
        let save_audio = auto_save;

        if let Some(mut receiver) = self.chunk_receiver.take() {
            tokio::spawn(async move {
                info!(
                    "Recording saver accumulation task started (save_audio: {})",
                    save_audio
                );

                // 2026-10-02 加的诊断：真机出现「No audio checkpoints to merge」时，
                // 完全看不出到底有没有音频块进来、进来多少 —— 这里把计数打出来。
                // 判读：task ended 时 chunks_received=0 → 上游根本没喂（管线/设备问题）；
                //       有计数但 checkpoints=0 → 块太小、始终没到 30 秒阈值。
                let mut chunks_received: u64 = 0;
                let mut samples_received: u64 = 0;

                while let Some(chunk) = receiver.recv().await {
                    // Check if we should continue
                    let should_continue = if let Ok(is_saving) = is_saving_clone.lock() {
                        *is_saving
                    } else {
                        false
                    };

                    if !should_continue {
                        break;
                    }

                    chunks_received += 1;
                    samples_received += chunk.data.len() as u64;

                    // Only process audio chunks if auto_save is enabled
                    if save_audio {
                        // Add chunk to incremental saver
                        if let Some(saver_arc) = &incremental_saver_arc {
                            let mut saver_guard = saver_arc.lock().await;
                            if let Err(e) = saver_guard.add_chunk(chunk) {
                                error!("Failed to add chunk to incremental saver: {}", e);
                            }
                        } else {
                            error!("Incremental saver not available while accumulating");
                        }
                    } else {
                        // auto_save is false: discard audio chunk (no-op)
                        // Transcription already happened in the pipeline before this point
                    }
                }

                info!(
                    "Recording saver accumulation task ended — 收到 {} 个音频块 / {} 个采样点（{:.1}s），save_audio={}",
                    chunks_received,
                    samples_received,
                    samples_received as f64 / 48000.0,
                    save_audio
                );
            });
        }

        // Set saving flag
        if let Ok(mut is_saving) = self.is_saving.lock() {
            *is_saving = true;
        }

        Ok(sender)
    }

    /// Initialize meeting folder structure and metadata
    ///
    /// # Arguments
    /// * `base_folder` - Base recordings directory (from the user's preferences)
    /// * `meeting_name` - Name of the meeting
    /// * `create_checkpoints` - Whether to create .checkpoints/ directory and IncrementalAudioSaver
    fn initialize_meeting_folder(
        &mut self,
        base_folder: &PathBuf,
        meeting_name: &str,
        create_checkpoints: bool,
    ) -> Result<()> {
        // Create meeting folder structure (with or without .checkpoints/ subdirectory)
        // 文件夹名固定 Rec_ 前缀（语言无关）；会议标题仍记在 metadata.meeting_name
        let meeting_folder = create_meeting_folder(
            base_folder,
            super::audio_processing::MEETING_FOLDER_PREFIX,
            create_checkpoints,
        )?;

        // Only initialize incremental saver if checkpoints are needed (auto_save is true)
        if create_checkpoints {
            let incremental_saver = IncrementalAudioSaver::new(meeting_folder.clone(), 48000)?;
            self.incremental_saver = Some(Arc::new(AsyncMutex::new(incremental_saver)));
            info!(
                "✅ Incremental audio saver initialized for meeting: {}",
                meeting_name
            );
        } else {
            info!("⚠️  Skipped incremental audio saver (auto-save disabled)");
        }

        // Create initial metadata
        let metadata = MeetingMetadata {
            version: "1.0".to_string(),
            meeting_id: None, // Will be set by backend
            meeting_name: Some(meeting_name.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            duration_seconds: None,
            devices: DeviceInfo {
                microphone: None, // Could be enhanced to store actual device names
                system_audio: None,
            },
            audio_file: if create_checkpoints {
                "audio.mp4".to_string()
            } else {
                "".to_string()
            },
            transcript_file: "transcripts.json".to_string(),
            sample_rate: 48000,
            status: "recording".to_string(),
        };

        // Write initial metadata.json
        self.write_metadata(&meeting_folder, &metadata)?;

        self.meeting_folder = Some(meeting_folder.clone());
        self.metadata = Some(metadata);

        // 目录确定后启动后台合并写线程（每个会议目录一个；停止录音时 shutdown）。
        if let Some(mut old) = self.transcript_writer.take() {
            old.shutdown();
        }
        self.transcript_writer = Some(TranscriptIncrementalWriter::start(
            meeting_folder,
            self.transcript_segments.clone(),
        ));

        Ok(())
    }

    /// Write metadata.json to disk (atomic write with temp file)
    fn write_metadata(&self, folder: &PathBuf, metadata: &MeetingMetadata) -> Result<()> {
        let metadata_path = folder.join("metadata.json");
        let temp_path = folder.join(".metadata.json.tmp");

        let json_string = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&temp_path, json_string)?;
        std::fs::rename(&temp_path, &metadata_path)?; // Atomic

        Ok(())
    }

    /// Write transcripts.json to disk (atomic write with temp file and validation)
    fn write_transcripts_json(&self, folder: &PathBuf) -> Result<()> {
        // Clone segments to avoid holding lock during I/O
        let segments_clone = if let Ok(segments) = self.transcript_segments.lock() {
            segments.clone()
        } else {
            error!("Failed to lock transcript segments for writing");
            return Err(anyhow::anyhow!("Failed to lock transcript segments"));
        };

        write_transcripts_json_snapshot(folder, &segments_clone)?;
        info!(
            "✅ Successfully wrote transcripts.json with {} segments",
            segments_clone.len()
        );
        Ok(())
    }

    // in frontend/src-tauri/src/audio/recording_saver.rs
    pub fn get_stats(&self) -> (usize, u32) {
        if let Some(ref saver) = self.incremental_saver {
            if let Ok(guard) = saver.try_lock() {
                (guard.get_checkpoint_count() as usize, 48000)
            } else {
                (0, 48000)
            }
        } else {
            (0, 48000)
        }
    }

    /// Stop and save using incremental saving approach
    ///
    /// # Arguments
    /// * `app` - Tauri app handle for emitting events
    /// * `recording_duration` - Actual recording duration in seconds (from RecordingState)
    pub async fn stop_and_save<R: Runtime>(
        &mut self,
        app: &AppHandle<R>,
        recording_duration: Option<f64>,
    ) -> Result<Option<String>, String> {
        info!("Stopping recording saver");

        // 先停后台合并写线程：否则一个「旧快照」可能在最终写盘之后落盘，
        // 把带译文的最终文件覆盖回去（两者共用一把写锁，但快照可能取早于译文回填）。
        if let Some(mut writer) = self.transcript_writer.take() {
            writer.shutdown();
        }

        // Stop accumulation
        if let Ok(mut is_saving) = self.is_saving.lock() {
            *is_saving = false;
        }

        // Give time for final chunks
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 回填最终译文：此刻翻译队列已排空（停止流程在摘监听后先 drain 再走到这里），
        // 录音进行中的增量写没有译文，最终写盘前统一补齐。
        if let Ok(mut segments) = self.transcript_segments.lock() {
            for seg in segments.iter_mut() {
                if seg.translation.is_empty() {
                    if let Some(t) = crate::translation::final_translation(seg.sequence_id) {
                        seg.translation = t;
                    }
                }
            }
        }

        // Check if incremental saver exists (indicates auto_save was enabled)
        let should_save_audio = self.incremental_saver.is_some();

        if !should_save_audio {
            // auto-save 关闭时不会再有最终写：把带译文的 transcripts.json 补写一次
            if let Some(folder) = &self.meeting_folder {
                if let Err(e) = self.write_transcripts_json(folder) {
                    warn!("Failed to write final transcripts (translations): {}", e);
                }
            }
            info!("⚠️  No audio saver initialized (auto-save was disabled) - skipping audio finalization");
            info!("✅ Transcripts and metadata already saved incrementally");
            return Ok(None);
        }

        // Finalize incremental saver (merge checkpoints into final audio.mp4)
        let final_audio_path = if let Some(saver_arc) = &self.incremental_saver {
            let mut saver = saver_arc.lock().await;
            match saver.finalize().await {
                Ok(path) => {
                    info!("✅ Successfully finalized audio: {}", path.display());
                    path
                }
                Err(e) => {
                    error!("❌ Failed to finalize incremental saver: {}", e);
                    return Err(format!("Failed to finalize audio: {}", e));
                }
            }
        } else {
            error!("No incremental saver initialized - cannot save recording");
            return Err("No incremental saver initialized".to_string());
        };

        // Save final transcripts.json with validation
        if let Some(folder) = &self.meeting_folder {
            if let Err(e) = self.write_transcripts_json(folder) {
                error!("❌ Failed to write final transcripts: {}", e);
                return Err(format!("Failed to save transcripts: {}", e));
            }

            // Verify transcripts were written correctly
            let transcript_path = folder.join("transcripts.json");
            if !transcript_path.exists() {
                error!(
                    "❌ Transcript file was not created at: {}",
                    transcript_path.display()
                );
                return Err("Transcript file verification failed".to_string());
            }
            info!(
                "✅ Transcripts saved and verified at: {}",
                transcript_path.display()
            );
        }

        // Update metadata to completed status with actual recording duration
        if let (Some(folder), Some(mut metadata)) = (&self.meeting_folder, self.metadata.clone()) {
            metadata.status = "completed".to_string();
            metadata.completed_at = Some(chrono::Utc::now().to_rfc3339());

            // Use actual recording duration from RecordingState (more accurate than transcript segments)
            // Falls back to last transcript segment if duration not provided.
            //
            // 2026-09-22 补充：`recording_duration` 偶尔为 0/None（实测录音文件夹
            // metadata.json 里 `duration_seconds: 0.0`，历史页因此显示不出音频长度）。
            // 增加两级兜底：① 直接用 ffprobe 探测刚写好的音频文件（最准）；
            // ② 退回最后一段转写的时间戳。
            metadata.duration_seconds = recording_duration
                .filter(|d| *d > 0.0)
                .or_else(|| {
                    crate::audio::decoder::probe_audio_duration(&final_audio_path)
                        .ok()
                        .filter(|d| *d > 0.0)
                })
                .or_else(|| {
                    if let Ok(segments) = self.transcript_segments.lock() {
                        segments
                            .last()
                            .map(|seg| seg.audio_end_time)
                            .filter(|d| *d > 0.0)
                    } else {
                        None
                    }
                });

            if let Err(e) = self.write_metadata(folder, &metadata) {
                error!("❌ Failed to update metadata to completed: {}", e);
                return Err(format!("Failed to update metadata: {}", e));
            }

            info!(
                "✅ Metadata updated with duration: {}",
                metadata
                    .duration_seconds
                    .map_or_else(|| "None".to_string(), |d| format!("{:.1}s", d))
            );
        }

        // Emit save event with audio and transcript paths
        let save_event = serde_json::json!({
            "audio_file": final_audio_path.to_string_lossy(),
            "transcript_file": self.meeting_folder.as_ref()
                .map(|f| f.join("transcripts.json").to_string_lossy().to_string()),
            "meeting_name": self.meeting_name,
            "meeting_folder": self.meeting_folder.as_ref()
                .map(|f| f.to_string_lossy().to_string())
        });

        if let Err(e) = app.emit("recording-saved", &save_event) {
            warn!("Failed to emit recording-saved event: {}", e);
        }

        // Clean up transcript segments
        if let Ok(mut segments) = self.transcript_segments.lock() {
            segments.clear();
        }

        Ok(Some(final_audio_path.to_string_lossy().to_string()))
    }

    /// Get the meeting folder path (for passing to backend)
    pub fn get_meeting_folder(&self) -> Option<&PathBuf> {
        self.meeting_folder.as_ref()
    }

    /// Get accumulated transcript segments (for reload sync)
    pub fn get_transcript_segments(&self) -> Vec<TranscriptSegment> {
        if let Ok(segments) = self.transcript_segments.lock() {
            segments.clone()
        } else {
            Vec::new()
        }
    }

    /// Get meeting name (for reload sync)
    pub fn get_meeting_name(&self) -> Option<String> {
        self.meeting_name.clone()
    }
}

impl Default for RecordingSaver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod transcript_writer_tests {
    use super::*;

    fn seg(seq: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: format!("seg_{}", seq),
            text: text.to_string(),
            audio_start_time: 0.0,
            audio_end_time: 0.0,
            duration: 0.0,
            display_time: "[00:00]".to_string(),
            confidence: 1.0,
            sequence_id: seq,
            translation: String::new(),
        }
    }

    fn read_segments(folder: &Path) -> Vec<TranscriptSegment> {
        let raw = std::fs::read_to_string(folder.join("transcripts.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        serde_json::from_value(v["segments"].clone()).unwrap()
    }

    /// 合并写：短时间内的多次变更只落盘少数几次，且最终文件是最新快照。
    #[test]
    fn incremental_writer_coalesces_and_lands_latest_snapshot() {
        let dir = std::env::temp_dir().join(format!(
            "vox_transcript_writer_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let segments = Arc::new(Mutex::new(Vec::<TranscriptSegment>::new()));
        let mut writer = TranscriptIncrementalWriter::start(dir.clone(), segments.clone());

        // 模拟 partial：1.5 秒窗口内连续 20 次变更（真实场景约 1~3 次/秒）
        for i in 0..20u64 {
            segments.lock().unwrap().push(seg(i, &format!("片段{}", i)));
            writer.mark_dirty();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        // 等后台线程把最后一个窗口写完
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut landed = Vec::new();
        while std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if dir.join("transcripts.json").exists() {
                landed = read_segments(&dir);
                if landed.len() == 20 {
                    break;
                }
            }
        }

        assert_eq!(landed.len(), 20, "最终快照必须是全部 20 段");
        assert_eq!(landed[19].text, "片段19");

        // 关键回归：20 次变更绝不应该写 20 次盘（合并窗口 1.5s / 变更间隔 20ms）
        let writes = TRANSCRIPT_WRITE_COUNT.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            writes >= 1 && writes <= 5,
            "20 次变更应被合并成 1~5 次写盘，实际 {} 次",
            writes
        );

        writer.shutdown();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// shutdown 之后不再有后台写：最终写盘（带译文）不会被旧快照覆盖。
    #[test]
    fn shutdown_stops_background_writes_before_final_write() {
        let dir = std::env::temp_dir().join(format!(
            "vox_transcript_writer_stop_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let segments = Arc::new(Mutex::new(vec![seg(1, "原文")]));
        let mut writer = TranscriptIncrementalWriter::start(dir.clone(), segments.clone());
        writer.mark_dirty();
        std::thread::sleep(std::time::Duration::from_millis(200)); // 让它在飞
        writer.shutdown();

        // 模拟停止流程：回填译文后做最终同步写
        segments.lock().unwrap()[0].translation = "译文".to_string();
        let snapshot = segments.lock().unwrap().clone();
        write_transcripts_json_snapshot(&dir, &snapshot).unwrap();

        // 再等一会儿：shutdown 之后不应再有任何后台写把译文抹掉
        std::thread::sleep(std::time::Duration::from_millis(300));
        let landed = read_segments(&dir);
        assert_eq!(landed.len(), 1);
        assert_eq!(landed[0].translation, "译文", "最终写盘后不得被旧快照覆盖");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
