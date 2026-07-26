//! Windows job-object helper.
//!
//! Assigns the spawned Python backend to a job with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so that all child processes are
//! terminated automatically when the Tauri parent process exits (including
//! crashes or force kills that bypass the normal `RunEvent::Exit` cleanup).

#![cfg(target_os = "windows")]

use std::os::windows::io::AsRawHandle;
use std::process::Child;

// Raw FFI types
pub type HANDLE = *mut std::ffi::c_void;
pub type DWORD = u32;
pub type BOOL = i32;

#[repr(C)]
struct LARGE_INTEGER {
    low_part: DWORD,
    high_part: i32,
}

#[repr(C)]
struct IO_COUNTERS {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[repr(C)]
struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
    per_process_user_time_limit: LARGE_INTEGER,
    per_job_user_time_limit: LARGE_INTEGER,
    limit_flags: DWORD,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: DWORD,
    affinity: usize,
    priority_class: DWORD,
    scheduling_class: DWORD,
}

#[repr(C)]
struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
    basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
    io_info: IO_COUNTERS,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x2000;
const TRUE: BOOL = 1;

extern "system" {
    fn CreateJobObjectW(lpJobAttributes: HANDLE, lpName: *const u16) -> HANDLE;
    fn SetInformationJobObject(
        hJob: HANDLE,
        JobObjectInfoClass: i32,
        lpJobObjectInfo: *const std::ffi::c_void,
        cbJobObjectInfoLength: DWORD,
    ) -> BOOL;
    fn AssignProcessToJobObject(hJob: HANDLE, hProcess: HANDLE) -> BOOL;
    fn CloseHandle(hObject: HANDLE) -> BOOL;
}

/// Put `child` into a kill-on-close job object.
pub fn assign_child_to_job(child: &Child) {
    unsafe {
        let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if job.is_null() {
            log::warn!("CreateJobObjectW failed; backend will not be auto-killed on parent exit");
            return;
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let ok = SetInformationJobObject(
            job,
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
            &info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD,
        );
        if ok != TRUE {
            log::warn!("SetInformationJobObject failed; backend will not be auto-killed on parent exit");
            CloseHandle(job);
            return;
        }

        let process_handle = child.as_raw_handle() as HANDLE;
        let ok = AssignProcessToJobObject(job, process_handle);
        if ok != TRUE {
            // The parent may already be in a job (debugger, shell, etc.).
            log::warn!("AssignProcessToJobObject failed; backend will not be auto-killed on parent exit");
            CloseHandle(job);
            return;
        }

        // Intentionally leak the job handle. As long as it stays open in this
        // process, the job stays active. When the Tauri process exits, the
        // handle is closed and Windows kills every process still in the job.
        let _ = Box::leak(Box::new(job));
        log::info!("Python backend assigned to kill-on-close job object");
    }
}
