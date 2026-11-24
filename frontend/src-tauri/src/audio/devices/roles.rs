//! 默认设备「角色」查询。
//!
//! Windows 有两套默认播放设备：
//! - 默认设备（eConsole）：浏览器、播放器等普通程序的出声设备；cpal 的
//!   `default_output_device()` 返回的就是它，本应用「系统默认」录的也是它。
//! - 默认通信设备（eCommunications）：腾讯会议 / Teams / Zoom / LINE 通话等
//!   会议软件通常把对方声音送到这里。
//!
//! 两者不一致时（典型：默认设备是笔记本扬声器、通信设备是 USB 耳机），
//! 会议声音进了耳机，我们却在扬声器上做回环采集 → 录到的全是静音。
//! 这里提供查询，供 UI 提示用户改录通信设备。

/// 当前 Windows 默认**通信**播放设备的名称（与 cpal 设备名同源：PKEY_Device_FriendlyName）。
/// 非 Windows 平台或查询失败时返回 `None`。
pub fn default_communication_output_device_name() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // 在独立线程里初始化 COM（MTA）并查询，避免影响调用方线程已有的 COM 套间。
        std::thread::spawn(windows_impl::default_communication_render_name)
            .join()
            .ok()
            .flatten()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
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

/// 若默认通信播放设备与 `captured_system_device` 不是同一个，返回通信设备名。
/// `captured_system_device` 为 `None`（未采集系统声音）时也返回通信设备名。
pub fn communication_output_if_different(captured_system_device: Option<&str>) -> Option<String> {
    let comm = default_communication_output_device_name()?;
    match captured_system_device {
        Some(captured) if same_device_name(captured, &comm) => None,
        _ => Some(comm),
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::Devices::Properties::DEVPKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{
        eCommunications, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        STGM_READ,
    };
    use windows::Win32::System::Variant::VT_LPWSTR;

    pub fn default_communication_render_name() -> Option<String> {
        unsafe {
            let init = CoInitializeEx(None, COINIT_MULTITHREADED);
            let name = query();
            if init.is_ok() {
                CoUninitialize();
            }
            name
        }
    }

    unsafe fn query() -> Option<String> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eCommunications)
            .ok()?;
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

    /// 真机探针：打印当前默认通信设备（仅 Windows 有值）。
    /// 运行：cargo test --lib default_communication_device_probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn default_communication_device_probe() {
        println!(
            "default communication output: {:?}",
            default_communication_output_device_name()
        );
    }
}
