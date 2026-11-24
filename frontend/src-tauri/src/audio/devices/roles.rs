//! 「声音实际在哪台设备上播放」检测。
//!
//! 本应用的「系统音频」是对**某一台**播放设备做回环采集（默认跟随 Windows 默认设备）。
//! 如果声音其实在另一台设备上播放（换了耳机、会议软件/浏览器单独指定了输出设备……），
//! 回环录到的就全是静音，识别不会有任何结果。
//!
//! 这里读取每台播放设备的峰值表（与 Windows 声音设置里的绿色音量条同源），
//! 找出「正在出声」的设备，供 UI 给出**有证据的**提示。
//! （早期版本按「默认通信设备 ≠ 录制设备」提示，但很多机器上通信设备常年是一个
//! 用不到的 USB 声卡，提示会一直误报——2026-11-24 用户反馈后改为实测。）

/// 峰值不低于该值视为「正在出声」（≈ -46 dBFS）。
pub const PLAYING_PEAK_THRESHOLD: f32 = 0.005;

/// 当前正在出声的播放设备名（与 cpal 设备名同源：PKEY_Device_FriendlyName），按响度降序。
/// 会在约 0.3 秒内多次采样峰值表（阻塞），请勿在 async 上下文直接调用。
/// 非 Windows 平台返回空列表。
pub fn playing_output_devices(threshold: f32) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        // 在独立线程里初始化 COM（MTA）并查询，避免影响调用方线程已有的 COM 套间。
        let mut devices = std::thread::spawn(move || windows_impl::playing_render_devices(threshold))
            .join()
            .unwrap_or_default();
        devices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        devices.into_iter().map(|(name, _)| name).collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = threshold;
        Vec::new()
    }
}

/// 除 `captured_system_device` 以外、正在出声的最响设备（阻塞，见 [`playing_output_devices`]）。
pub fn playing_device_other_than(captured_system_device: Option<&str>) -> Option<String> {
    pick_other_device(
        playing_output_devices(PLAYING_PEAK_THRESHOLD),
        captured_system_device,
    )
}

fn pick_other_device(playing: Vec<String>, captured: Option<&str>) -> Option<String> {
    playing
        .into_iter()
        .find(|name| captured.map_or(true, |c| !same_device_name(name, c)))
}

/// 去掉部分平台给设备名追加的 " (input)" / " (output)" 后缀，便于比较。
pub fn normalize_device_name(name: &str) -> &str {
    let trimmed = name.trim();
    trimmed
        .strip_suffix("(input)")
        .or_else(|| trimmed.strip_suffix("(output)"))
        .map(str::trim_end)
        .unwrap_or(trimmed)
}

/// 两个设备名是否指向同一设备（忽略首尾空白与 input/output 后缀）。
pub fn same_device_name(a: &str, b: &str) -> bool {
    normalize_device_name(a) == normalize_device_name(b)
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::time::Duration;

    use windows::Win32::Devices::Properties::DEVPKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        STGM_READ,
    };
    use windows::Win32::System::Variant::VT_LPWSTR;

    /// 峰值表是「上一个刷新周期」的峰值，单次读数可能恰好落在语音停顿里，多采几次取最大。
    const SAMPLES: usize = 8;
    const SAMPLE_INTERVAL: Duration = Duration::from_millis(40);

    pub fn playing_render_devices(threshold: f32) -> Vec<(String, f32)> {
        unsafe {
            let init = CoInitializeEx(None, COINIT_MULTITHREADED);
            let result = query(threshold).unwrap_or_default();
            if init.is_ok() {
                CoUninitialize();
            }
            result
        }
    }

    unsafe fn query(threshold: f32) -> Option<Vec<(String, f32)>> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let collection = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .ok()?;
        let count = collection.GetCount().ok()?;

        let mut meters: Vec<(IMMDevice, IAudioMeterInformation, f32)> = Vec::new();
        for i in 0..count {
            let Ok(device) = collection.Item(i) else { continue };
            if let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) {
                meters.push((device, meter, 0.0));
            }
        }
        for round in 0..SAMPLES {
            for (_, meter, max_peak) in meters.iter_mut() {
                if let Ok(peak) = meter.GetPeakValue() {
                    *max_peak = max_peak.max(peak);
                }
            }
            if round + 1 < SAMPLES {
                std::thread::sleep(SAMPLE_INTERVAL);
            }
        }

        Some(
            meters
                .into_iter()
                .filter(|(_, _, peak)| *peak >= threshold)
                .filter_map(|(device, _, peak)| friendly_name(&device).map(|n| (n, peak)))
                .collect(),
        )
    }

    unsafe fn friendly_name(device: &IMMDevice) -> Option<String> {
        let store = device.OpenPropertyStore(STGM_READ).ok()?;
        let mut value = store
            .GetValue(&DEVPKEY_Device_FriendlyName as *const _ as *const _)
            .ok()?;
        let raw = &value.as_raw().Anonymous.Anonymous;
        let name = if raw.vt == VT_LPWSTR.0 {
            let ptr = *(&raw.Anonymous as *const _ as *const *const u16);
            if ptr.is_null() {
                None
            } else {
                let mut len = 0usize;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let wide = std::slice::from_raw_parts(ptr, len);
                Some(OsString::from_wide(wide).to_string_lossy().into_owned())
            }
        } else {
            None
        };
        PropVariantClear(&mut value).ok();
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_name_comparison_ignores_suffix_and_whitespace() {
        assert!(same_device_name("耳机 (USB Audio)", " 耳机 (USB Audio) "));
        assert!(same_device_name("Speakers (output)", "Speakers"));
        assert!(!same_device_name("扬声器 (Realtek(R) Audio)", "耳机 (USB Audio)"));
    }

    #[test]
    fn picks_loudest_device_that_is_not_captured() {
        let playing = vec!["扬声器 (USB Audio Device)".to_string(), "扬声器 (Realtek(R) Audio)".to_string()];
        assert_eq!(
            pick_other_device(playing.clone(), Some("扬声器 (USB Audio Device)")),
            Some("扬声器 (Realtek(R) Audio)".to_string())
        );
        assert_eq!(
            pick_other_device(playing.clone(), Some("耳机 (USB Audio)")),
            Some("扬声器 (USB Audio Device)".to_string())
        );
        assert_eq!(
            pick_other_device(vec!["扬声器 (Realtek(R) Audio)".into()], Some("扬声器 (Realtek(R) Audio)")),
            None
        );
        assert_eq!(pick_other_device(Vec::new(), None), None);
    }

    /// 真机探针：在指定输出设备上放 2 秒正弦波，检测应能看到它「正在出声」。
    /// 运行：PLAYING_PROBE_DEVICE="扬声器 (Realtek(R) Audio)" \
    ///   cargo test --lib playing_device_probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn playing_device_probe() {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        println!("before playback: {:?}", playing_output_devices(PLAYING_PEAK_THRESHOLD));

        let wanted = std::env::var("PLAYING_PROBE_DEVICE").expect("set PLAYING_PROBE_DEVICE");
        let host = cpal::default_host();
        let device = host
            .output_devices()
            .unwrap()
            .find(|d| d.name().map(|n| same_device_name(&n, &wanted)).unwrap_or(false))
            .expect("probe device not found");
        let config = device.default_output_config().unwrap();
        let rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        let mut phase = 0f32;
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in data.chunks_mut(channels) {
                        let s = (phase * 2.0 * std::f32::consts::PI).sin() * 0.2;
                        phase = (phase + 440.0 / rate) % 1.0;
                        frame.iter_mut().for_each(|x| *x = s);
                    }
                },
                |e| eprintln!("stream error: {e}"),
                None,
            )
            .unwrap();
        stream.play().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(700));
        let during = playing_output_devices(PLAYING_PEAK_THRESHOLD);
        println!("during playback on {wanted:?}: {during:?}");
        println!(
            "other than default-captured 耳机 (USB Audio): {:?}",
            playing_device_other_than(Some("耳机 (USB Audio)"))
        );
        std::thread::sleep(std::time::Duration::from_millis(1300));
        drop(stream);
        assert!(during.iter().any(|n| same_device_name(n, &wanted)));
    }
}
