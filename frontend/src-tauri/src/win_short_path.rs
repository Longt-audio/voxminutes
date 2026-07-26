//! Windows short-path helpers for non-ASCII directory support.
//!
//! Several bundled C/C++ libraries (Sherpa-ONNX, ONNX Runtime, FFmpeg, llama.cpp)
//! open files using the current ANSI code page. When the application is placed in
//! a path containing Chinese, Korean, or other non-ASCII characters, those libraries
//! may fail to locate their models or audio files. Converting the path to the
//! legacy 8.3 short form avoids this issue entirely.

use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
extern "system" {
    fn GetShortPathNameW(lpszLongPath: *const u16, lpszShortPath: *mut u16, cchBuffer: u32) -> u32;
}

/// Convert a path to its Windows short (8.3) form.
/// On non-Windows platforms or if conversion fails, the original path is returned.
pub fn to_short_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let len = GetShortPathNameW(wide.as_ptr(), std::ptr::null_mut(), 0);
            if len == 0 {
                return path.to_path_buf();
            }
            let mut buffer: Vec<u16> = vec![0; len as usize];
            let result_len = GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), len);
            if result_len == 0 || result_len >= len {
                return path.to_path_buf();
            }
            buffer.truncate(result_len as usize);
            return PathBuf::from(std::ffi::OsString::from_wide(&buffer));
        }
    }

    #[cfg(not(target_os = "windows"))]
    path.to_path_buf()
}

/// Convert a path to a string suitable for passing to ANSI-sensitive C libraries.
/// On Windows this is the short path; elsewhere it is the normal UTF-8 path.
pub fn to_short_path_string(path: &Path) -> String {
    to_short_path(path).to_string_lossy().to_string()
}
