use anyhow::Result;
use log::{error, info, warn};
use realfft::{RealFftPlanner, RealToComplex};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Number of time-domain samples sent to the UI per update. This is independent
/// of the capture sample rate; the raw ring buffer is decimated down to this
/// count so the frontend always draws a fixed-width waveform.
const UI_SAMPLE_COUNT: usize = 300;

/// Default time window (ms) of audio to display in the waveform visualizer.
const WAVEFORM_WINDOW_MS: f32 = 300.0;

/// FFT window for the frequency-domain spectrum sent to the UI.
const FFT_SIZE: usize = 2048;
/// Number of log-spaced frequency bands drawn by the frontend spectrum.
const SPECTRUM_BANDS: usize = 48;
const SPECTRUM_MIN_HZ: f32 = 60.0;
const SPECTRUM_MAX_HZ: f32 = 16000.0;
/// Magnitudes at or below this dB level render as silence.
const SPECTRUM_FLOOR_DB: f32 = -60.0;

lazy_static::lazy_static! {
    static ref FFT_PLAN: Arc<dyn RealToComplex<f32>> = {
        let mut planner = RealFftPlanner::<f32>::new();
        planner.plan_fft_forward(FFT_SIZE)
    };
}

/// Compute a normalized (0..1) log-frequency-band magnitude spectrum from the
/// latest `FFT_SIZE` samples, suitable for a PotPlayer-style spectrum display.
fn compute_spectrum(all_samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if all_samples.is_empty() || sample_rate == 0 {
        return vec![0.0; SPECTRUM_BANDS];
    }

    // Latest FFT_SIZE samples, zero-padded at the front when short.
    let mut input = FFT_PLAN.make_input_vec();
    let take = FFT_SIZE.min(all_samples.len());
    input[FFT_SIZE - take..].copy_from_slice(&all_samples[all_samples.len() - take..]);

    // Hann window.
    let n = FFT_SIZE as f32;
    for (i, v) in input.iter_mut().enumerate() {
        *v *= 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1.0)).cos());
    }

    let mut output = FFT_PLAN.make_output_vec();
    if FFT_PLAN.process(&mut input, &mut output).is_err() {
        return vec![0.0; SPECTRUM_BANDS];
    }

    let scale = 2.0 / FFT_SIZE as f32;
    let magnitudes: Vec<f32> = output.iter().map(|c| c.norm() * scale).collect();
    let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
    let max_hz = SPECTRUM_MAX_HZ.min(sample_rate as f32 / 2.0);
    let log_min = SPECTRUM_MIN_HZ.ln();
    let log_span = max_hz.ln() - log_min;

    (0..SPECTRUM_BANDS)
        .map(|b| {
            let f_lo = (log_min + log_span * b as f32 / SPECTRUM_BANDS as f32).exp();
            let f_hi = (log_min + log_span * (b + 1) as f32 / SPECTRUM_BANDS as f32).exp();
            let lo = ((f_lo / bin_hz) as usize).min(magnitudes.len());
            let hi = (((f_hi / bin_hz).ceil() as usize).max(lo + 1)).min(magnitudes.len());
            let count = hi.saturating_sub(lo).max(1);
            let avg = magnitudes[lo..hi].iter().copied().sum::<f32>() / count as f32;
            let db = 20.0 * (avg + 1e-9).log10();
            ((db - SPECTRUM_FLOOR_DB) / -SPECTRUM_FLOOR_DB).clamp(0.0, 1.0)
        })
        .collect()
}

#[derive(Debug, Serialize, Clone)]
pub struct AudioLevelData {
    pub device_name: String,
    pub device_type: String, // "input" or "output"
    pub rms_level: f32,      // RMS level (0.0 to 1.0)
    pub peak_level: f32,     // Peak level (0.0 to 1.0)
    pub is_active: bool,     // Whether audio is being detected
    pub spectrum: Vec<f32>,  // Log-frequency-band magnitudes (0..1) for the spectrum visualizer.
    pub samples: Vec<f32>,   // Time-domain samples for the waveform visualizer.
}

#[derive(Debug, Serialize, Clone)]
pub struct AudioLevelUpdate {
    pub timestamp: u64,
    pub levels: Vec<AudioLevelData>,
}

// Simple global monitoring state
static IS_MONITORING: AtomicBool = AtomicBool::new(false);
/// 每次 start_monitoring 自增；旧任务发现代数变化就退出。
/// 之前只靠 IS_MONITORING + sleep(100ms)：stop/start 间隔太短时旧任务看不到 false，
/// 会出现两个监控任务并存、事件重复推送。
static MONITOR_GENERATION: AtomicU64 = AtomicU64::new(0);

/// 峰值低于该值视为「没有声音」（≈ -50 dBFS）。
/// 实测（2026-11-24 用户日志）：USB 耳机麦克风无人说话时峰值 ≤ 0.0015，回环无播放时为 0；
/// 正常说话 / 会议声音峰值通常 > 0.05。
pub const SILENCE_PEAK_THRESHOLD: f32 = 0.003;
/// 连续静音多久后提醒用户。
pub const SILENCE_WARN_AFTER: Duration = Duration::from_secs(30);

/// 「持续没有声音」提醒的载荷（事件 `audio-silence-warning`）。
#[derive(Debug, Serialize, Clone)]
pub struct SilenceWarning {
    pub silent_secs: u64,
    pub mic_device: Option<String>,
    pub system_device: Option<String>,
    pub mic_muted: bool,
    /// 实测「正在出声」、但**不是**当前录制设备的播放设备（声音在别处播放 → 回环录不到）。
    pub playing_device: Option<String>,
}

#[derive(Debug, PartialEq)]
enum SilenceEvent {
    Warn(Duration),
    Cleared,
}

/// 连续静音检测（纯逻辑，便于单测）。每次静音段只提醒一次；声音恢复后重新计时。
struct SilenceTracker {
    last_sound: Instant,
    warned: bool,
}

impl SilenceTracker {
    fn new(now: Instant) -> Self {
        Self {
            last_sound: now,
            warned: false,
        }
    }

    fn update(&mut self, now: Instant, peak: f32, paused: bool) -> Option<SilenceEvent> {
        if paused {
            // 暂停期间不计时（暂停本来就可能没声音）
            self.last_sound = now;
            return None;
        }
        if peak >= SILENCE_PEAK_THRESHOLD {
            self.last_sound = now;
            if self.warned {
                self.warned = false;
                return Some(SilenceEvent::Cleared);
            }
            return None;
        }
        let silent_for = now.saturating_duration_since(self.last_sound);
        if !self.warned && silent_for >= SILENCE_WARN_AFTER {
            self.warned = true;
            return Some(SilenceEvent::Warn(silent_for));
        }
        None
    }
}

/// 系统通知文案（按界面语言）。主窗口经常被隐藏（只开悬浮球/字幕），只靠页面内横幅用户看不到。
/// 依据：声音在别的设备上播放 → 提示改录那台；麦克风静音 → 提示打开；
/// 麦克风开着却没信号 → 提示它可能不是正在用的那个麦克风（2026-11-24 真机：当时的默认输入
/// 「耳机 (USB Audio)」只有 -72 dB 底噪，9 分钟录音里一段语音都没检测到）。
fn silence_notification_text(lang: &str, w: &SilenceWarning) -> (String, String) {
    let secs = w.silent_secs;
    let sys = w.system_device.clone().unwrap_or_default();
    let mic = w.mic_device.clone().unwrap_or_default();
    let mut body = match lang {
        "en" => format!("No sound has been captured for {}s, so nothing can be transcribed.", secs),
        "ja" => format!("{}秒間、音声が取り込まれていないため文字起こしできません。", secs),
        "ko" => format!("{}초 동안 소리가 수집되지 않아 받아쓰기를 할 수 없습니다.", secs),
        _ => format!("已连续 {} 秒没有采集到任何声音，识别不会有结果。", secs),
    };
    if let Some(playing) = &w.playing_device {
        body.push_str(&match lang {
            "en" => format!(" Sound is playing on \"{}\", but \"{}\" is being recorded.", playing, sys),
            "ja" => format!("音声は「{}」で再生されていますが、録音中なのは「{}」です。", playing, sys),
            "ko" => format!(" 소리는 \"{}\"에서 재생 중이지만 \"{}\"을(를) 녹음하고 있습니다.", playing, sys),
            _ => format!("声音正在「{}」上播放，但录的是「{}」。", playing, sys),
        });
    }
    if w.mic_muted {
        body.push_str(match lang {
            "en" => " The microphone is muted.",
            "ja" => "マイクはミュート中です。",
            "ko" => " 마이크가 음소거되어 있습니다.",
            _ => "麦克风处于静音状态。",
        });
    } else if w.mic_device.is_some() {
        body.push_str(&match lang {
            "en" => format!(" The microphone \"{}\" is getting no signal — it may not be the mic you are using.", mic),
            "ja" => format!("マイク「{}」に信号がありません。使用中のマイクではない可能性があります。", mic),
            "ko" => format!(" 마이크 \"{}\"에 신호가 없습니다. 사용 중인 마이크가 아닐 수 있습니다.", mic),
            _ => format!("麦克风「{}」没有信号，可能不是你正在用的那个麦克风。", mic),
        });
    }
    let title = match lang {
        "en" => "VoxMinutes: no sound captured",
        "ja" => "VoxMinutes：音声が取り込まれていません",
        "ko" => "VoxMinutes: 소리가 수집되지 않음",
        _ => "VoxMinutes：没有录到声音",
    };
    (title.to_string(), body)
}

async fn emit_silence_warning<R: Runtime>(
    app: &AppHandle<R>,
    silent_for: Duration,
    mic_device: &Option<String>,
    system_device: &Option<String>,
    mic_muted: bool,
) {
    let sys_for_query = system_device.clone();
    // 读峰值表要采样 ~0.3s 且走 COM，放到阻塞线程池，避免卡住 tokio worker
    let playing_device = tokio::task::spawn_blocking(move || {
        super::devices::playing_device_other_than(sys_for_query.as_deref())
    })
    .await
    .unwrap_or(None);

    let warning = SilenceWarning {
        silent_secs: silent_for.as_secs(),
        mic_device: mic_device.clone(),
        system_device: system_device.clone(),
        mic_muted,
        playing_device,
    };
    warn!(
        "🔇 已连续 {}s 没有采集到声音（峰值 < {}）：mic={:?}（静音={}）, system={:?}, 正在别处出声的设备={:?} \
         —— 识别不会有结果",
        warning.silent_secs,
        SILENCE_PEAK_THRESHOLD,
        warning.mic_device,
        warning.mic_muted,
        warning.system_device,
        warning.playing_device
    );
    let _ = app.emit("audio-silence-warning", &warning);

    // 系统通知（尊重通知设置里的「系统错误」开关）
    let (title, body) = silence_notification_text(&crate::translation::home_lang(), &warning);
    if let Some(state) =
        app.try_state::<crate::notifications::commands::NotificationManagerState<R>>()
    {
        let guard = state.read().await;
        if let Some(manager) = guard.as_ref() {
            use crate::notifications::types::{
                Notification, NotificationPriority, NotificationTimeout, NotificationType,
            };
            let notification =
                Notification::new(title, body.clone(), NotificationType::SystemError(body))
                    .with_priority(NotificationPriority::High)
                    .with_timeout(NotificationTimeout::Seconds(10));
            if let Err(e) = manager.show_notification(notification).await {
                warn!("Failed to show silence notification: {}", e);
            }
        }
    }
}

/// Decimate a slice of samples down to `target` points by averaging bins.
/// Preserves the overall waveform shape and is cheap enough to run every 50 ms.
fn downsample_samples(samples: &[f32], target: usize) -> Vec<f32> {
    if samples.is_empty() || target == 0 {
        return Vec::new();
    }
    if samples.len() <= target {
        return samples.to_vec();
    }

    let step = samples.len() as f32 / target as f32;
    (0..target)
        .map(|i| {
            let start = (i as f32 * step) as usize;
            let end = (((i + 1) as f32 * step) as usize).min(samples.len());
            let count = (end - start).max(1);
            let sum: f32 = samples[start..end].iter().copied().sum();
            sum / count as f32
        })
        .collect()
}

/// Return the most recent samples from the captured ring buffer clipped to the
/// desired waveform time window.
fn waveform_samples_for_rate(sample_rate: u32, all_samples: &[f32]) -> Vec<f32> {
    let window_samples = ((sample_rate as f32 * WAVEFORM_WINDOW_MS) / 1000.0) as usize;
    let take = window_samples.min(all_samples.len());
    if take == 0 {
        return Vec::new();
    }
    // The ring buffer stores oldest-to-newest, so the latest samples are at the end.
    all_samples[all_samples.len() - take..].to_vec()
}

/// Start audio level monitoring for the given microphone / system-audio devices.
/// Reads real audio levels from the global store updated by the audio pipeline.
///
/// 设备类型由调用方**显式**给出：以前靠设备名里是否包含 "mic" 猜输入/输出，
/// 中文/日文系统的「耳机 (USB Audio)」「麦克风阵列」「マイク」都猜不中（而且 USB 耳机的
/// 输入、输出两端同名），麦克风电平条因此永远为 0。
pub async fn start_monitoring<R: Runtime>(
    app_handle: AppHandle<R>,
    mic_device: Option<String>,
    system_device: Option<String>,
) -> Result<()> {
    info!(
        "Starting real audio level monitoring: mic={:?}, system={:?}",
        mic_device, system_device
    );

    // Stop any existing monitoring
    IS_MONITORING.store(false, Ordering::SeqCst);
    let generation = MONITOR_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    // Wait a bit for any existing tasks to stop
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    if MONITOR_GENERATION.load(Ordering::SeqCst) != generation {
        // 等待期间又有新的 start_monitoring：交给更新的那次
        return Ok(());
    }

    // Start new monitoring
    IS_MONITORING.store(true, Ordering::SeqCst);

    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        let mut last_vad_active: Option<bool> = None;
        let mut silence = SilenceTracker::new(Instant::now());
        let mut paused = false;
        let mut mic_muted = false;
        let mut last_pause_check = Instant::now();
        while IS_MONITORING.load(Ordering::SeqCst)
            && MONITOR_GENERATION.load(Ordering::SeqCst) == generation
        {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // 上报 VAD「语音进行中」状态变化（供前端显示「正在识别」提示）。
            let vad_active = super::recording_state::is_vad_speaking();
            if last_vad_active != Some(vad_active) {
                last_vad_active = Some(vad_active);
                let _ = app_handle_clone.emit(
                    "vad-speech-activity",
                    serde_json::json!({ "active": vad_active }),
                );
            }

            let store = super::recording_state::get_audio_level_store();
            let (sample_rate, spectrum_samples) = super::recording_state::get_spectrum_data();

            // Capture a fixed time window of the most recent samples and decimate
            // them to a UI-friendly count. The frontend draws these as a fixed
            // non-scrolling waveform that jumps up and down in place.
            let window_samples = waveform_samples_for_rate(sample_rate, &spectrum_samples);
            let ui_samples = downsample_samples(&window_samples, UI_SAMPLE_COUNT);

            // If data is stale (> 2 seconds old), treat as silent
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let is_stale = now.saturating_sub(store.last_update_ms) > 2000;

            // Frequency-domain spectrum from the full-rate ring buffer (shared
            // by all reported devices; the frontend picks the first non-empty).
            let spectrum = if is_stale {
                vec![0.0; SPECTRUM_BANDS]
            } else {
                compute_spectrum(&spectrum_samples, sample_rate)
            };

            // 连续静音检测：两路（静音后的麦克风 + 系统回环）峰值都低于阈值即视为没有声音。
            // 数据陈旧（流断了、设备重建中）同样按没有声音处理。
            if last_pause_check.elapsed() >= Duration::from_secs(1) {
                last_pause_check = Instant::now();
                if let Some((p, m)) = super::recording_commands::try_recording_flags() {
                    paused = p;
                    mic_muted = m;
                }
            }
            let loudest = if is_stale {
                0.0
            } else {
                store.mic_peak.max(store.system_peak)
            };
            match silence.update(Instant::now(), loudest, paused) {
                Some(SilenceEvent::Warn(silent_for)) => {
                    emit_silence_warning(
                        &app_handle_clone,
                        silent_for,
                        &mic_device,
                        &system_device,
                        mic_muted,
                    )
                    .await;
                }
                Some(SilenceEvent::Cleared) => {
                    info!("🔊 Audio captured again — clearing silence warning");
                    let _ = app_handle_clone.emit("audio-silence-cleared", ());
                }
                None => {}
            }

            let mut levels: Vec<AudioLevelData> = Vec::new();

            let mut push_level = |name: &String, is_mic: bool| {
                let (rms, peak, active) = if is_stale {
                    (0.0f32, 0.0f32, false)
                } else if is_mic {
                    (store.mic_rms, store.mic_peak, store.mic_active)
                } else {
                    (store.system_rms, store.system_peak, store.system_active)
                };
                levels.push(AudioLevelData {
                    device_name: name.clone(),
                    device_type: if is_mic { "input" } else { "output" }.to_string(),
                    rms_level: rms,
                    peak_level: peak,
                    is_active: active,
                    spectrum: spectrum.clone(),
                    samples: ui_samples.clone(),
                });
            };
            if let Some(name) = &mic_device {
                push_level(name, true);
            }
            if let Some(name) = &system_device {
                push_level(name, false);
            }

            if !levels.is_empty() {
                let update = AudioLevelUpdate {
                    timestamp: now,
                    levels,
                };

                if let Err(e) = app_handle_clone.emit("audio-levels", &update) {
                    error!("Failed to emit audio levels: {}", e);
                    break;
                }
            }
        }

        // 监控结束：清除「语音进行中」状态。
        let _ = app_handle_clone.emit(
            "vad-speech-activity",
            serde_json::json!({ "active": false }),
        );
        // 已发出的静音提醒随本任务一起撤销（设备切换后新任务会重新计时、必要时再提醒），
        // 否则前端横幅会一直挂着。
        if silence.warned {
            let _ = app_handle_clone.emit("audio-silence-cleared", ());
        }

        info!("Audio level monitoring task ended");
    });

    Ok(())
}

/// Stop audio level monitoring
pub async fn stop_monitoring() -> Result<()> {
    info!("Stopping audio level monitoring");
    IS_MONITORING.store(false, Ordering::SeqCst);
    MONITOR_GENERATION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

/// Check if currently monitoring
pub fn is_monitoring() -> bool {
    IS_MONITORING.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOISE_FLOOR: f32 = 0.0015; // USB 耳机麦克风无人说话时的实测峰值
    const SPEECH: f32 = 0.2;

    #[test]
    fn warns_once_after_sustained_silence() {
        let t0 = Instant::now();
        let mut tracker = SilenceTracker::new(t0);
        assert_eq!(tracker.update(t0 + Duration::from_secs(10), NOISE_FLOOR, false), None);
        assert_eq!(tracker.update(t0 + Duration::from_secs(29), 0.0, false), None);
        assert_eq!(
            tracker.update(t0 + Duration::from_secs(30), NOISE_FLOOR, false),
            Some(SilenceEvent::Warn(Duration::from_secs(30)))
        );
        // 同一段静音不重复提醒
        assert_eq!(tracker.update(t0 + Duration::from_secs(90), 0.0, false), None);
    }

    #[test]
    fn sound_resets_timer_and_clears_warning() {
        let t0 = Instant::now();
        let mut tracker = SilenceTracker::new(t0);
        assert_eq!(tracker.update(t0 + Duration::from_secs(20), SPEECH, false), None);
        // 有声音后重新计时：第 45 秒距上次有声音只有 25 秒
        assert_eq!(tracker.update(t0 + Duration::from_secs(45), 0.0, false), None);
        assert!(matches!(
            tracker.update(t0 + Duration::from_secs(50), 0.0, false),
            Some(SilenceEvent::Warn(_))
        ));
        assert_eq!(
            tracker.update(t0 + Duration::from_secs(51), SPEECH, false),
            Some(SilenceEvent::Cleared)
        );
        // 清除后可再次提醒
        assert!(matches!(
            tracker.update(t0 + Duration::from_secs(81), 0.0, false),
            Some(SilenceEvent::Warn(_))
        ));
    }

    #[test]
    fn paused_time_does_not_count_as_silence() {
        let t0 = Instant::now();
        let mut tracker = SilenceTracker::new(t0);
        assert_eq!(tracker.update(t0 + Duration::from_secs(120), 0.0, true), None);
        assert_eq!(tracker.update(t0 + Duration::from_secs(140), 0.0, false), None);
        assert!(matches!(
            tracker.update(t0 + Duration::from_secs(150), 0.0, false),
            Some(SilenceEvent::Warn(_))
        ));
    }

    #[test]
    fn notification_text_follows_evidence() {
        let mut w = SilenceWarning {
            silent_secs: 30,
            mic_device: Some("耳机 (USB Audio)".into()),
            system_device: Some("扬声器 (Realtek(R) Audio)".into()),
            mic_muted: false,
            playing_device: Some("扬声器 (USB Audio Device)".into()),
        };
        let (_, body) = silence_notification_text("zh", &w);
        assert!(body.contains("声音正在「扬声器 (USB Audio Device)」上播放"));
        assert!(body.contains("录的是「扬声器 (Realtek(R) Audio)」"));
        // 麦克风开着却没信号 → 点名这个麦克风
        assert!(body.contains("麦克风「耳机 (USB Audio)」没有信号"));

        // 没检测到别处出声、麦克风静音 → 只提示静音
        w.playing_device = None;
        w.mic_muted = true;
        let (_, body) = silence_notification_text("zh", &w);
        assert!(!body.contains("正在「"));
        assert!(body.contains("麦克风处于静音状态") && !body.contains("没有信号"));

        let (title, body) = silence_notification_text("en", &w);
        assert!(title.contains("no sound") && body.contains("muted"));
    }
}
