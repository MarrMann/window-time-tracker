use winapi::{
    shared::{
        minwindef::{BOOL, LPARAM},
        windef::HWND,
    },
    um::{
        winuser::{EnumWindows, GetWindowTextW, IsWindowVisible},
    },
};
use regex::Regex;

unsafe extern "system" fn enum_windows_callback(window_handle: HWND, lparam: LPARAM) -> BOOL {
    let mut title: [u16; 512] = [0; 512];
    GetWindowTextW(window_handle, title.as_mut_ptr(), title.len() as i32);

    if IsWindowVisible(window_handle) == 0 {
        return 1;
    }

    // let pid = {
    //     let mut pid:u32 = 0;
    //     GetWindowThreadProcessId(hwnd, &mut pid);
    //     pid
    // };

    let windows = &mut *(lparam as *mut Vec<(String, HWND)>);
    // Regex ascii only
    let title = String::from_utf16_lossy(&title).trim().to_string().replace(r"[\x20-\x7E]", "");
    let re:Regex = Regex::new(r"[\x20-\x7E]").unwrap();
    if re.is_match(&title) {
        windows.push((title, window_handle));
    }

    1
}

pub fn get_open_windows(top_windows: u32) -> Vec<(String, HWND)> {
    let mut windows: Vec<(String, HWND)> = Vec::new();
    unsafe {
        EnumWindows(Some(enum_windows_callback), &mut windows as *mut _ as LPARAM);
    }
    
    // Return only the top_windows number of windows
    windows.iter().take(top_windows as usize).map(|w| w.clone()).collect()
}