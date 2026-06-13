use tauri::{Emitter, Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, FindWindowExW, GetWindowRect, SetWindowPos,
    GetForegroundWindow, GetWindowLongW, SetWindowLongW,
    GWL_EXSTYLE, WS_EX_TRANSPARENT, WS_EX_LAYERED, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW
};
use windows::Win32::System::Performance::{
    PdhOpenQueryW, PdhAddEnglishCounterW, PdhCollectQueryData,
    PdhGetFormattedCounterValue, PdhGetFormattedCounterArrayW,
    PDH_FMT_DOUBLE, PDH_HQUERY, PDH_HCOUNTER
};
use windows::core::PCWSTR;
use sysinfo::System;

#[derive(serde::Serialize, Clone, Debug)]
struct SystemStats {
    cpu: f32,
    ram_pct: f32,
    ram_used: f64,
    ram_total: f64,
    disk_read: f64,
    disk_write: f64,
    net_recv: f64,
    net_sent: f64,
    gpu: f32,
}

fn wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn get_clock_rect() -> Option<RECT> {
    unsafe {
        let tray_class = wstr("Shell_TrayWnd");
        let tray_hwnd = FindWindowW(PCWSTR(tray_class.as_ptr()), None).ok()?;

        let clock_class = wstr("TrayNotifyWnd");
        let clock_hwnd = FindWindowExW(
            Some(tray_hwnd),
            None,
            PCWSTR(clock_class.as_ptr()),
            None
        ).ok()?;

        let mut rect = RECT::default();
        if GetWindowRect(clock_hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            None
        }
    }
}

fn get_window_class(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut buf)
    };
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}

fn is_foreground_window_fullscreen(screen_width: u32, screen_height: u32) -> bool {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0.is_null() {
            return false;
        }

        let mut rect = RECT::default();
        if GetWindowRect(fg_hwnd, &mut rect).is_ok() {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;

            if w >= screen_width as i32 && h >= screen_height as i32 {
                let class_name = get_window_class(fg_hwnd);
                if class_name != "Progman" && class_name != "WorkerW" && class_name != "Shell_TrayWnd" {
                    return true;
                }
            }
        }
        false
    }
}

// PDH handles are raw pointers that are safe to send across threads
struct PdhSystemQuery {
    h_query: PDH_HQUERY,
    h_disk_read: PDH_HCOUNTER,
    h_disk_write: PDH_HCOUNTER,
    h_gpu_util: PDH_HCOUNTER,
}

unsafe impl Send for PdhSystemQuery {}

impl PdhSystemQuery {
    fn new() -> Option<Self> {
        unsafe {
            let mut h_query = PDH_HQUERY::default();
            if PdhOpenQueryW(None, 0, &mut h_query) != 0 {
                return None;
            }

            let mut h_disk_read = PDH_HCOUNTER::default();
            let read_path = wstr("\\PhysicalDisk(_Total)\\Disk Read Bytes/sec");
            let _ = PdhAddEnglishCounterW(h_query, PCWSTR(read_path.as_ptr()), 0, &mut h_disk_read);

            let mut h_disk_write = PDH_HCOUNTER::default();
            let write_path = wstr("\\PhysicalDisk(_Total)\\Disk Write Bytes/sec");
            let _ = PdhAddEnglishCounterW(h_query, PCWSTR(write_path.as_ptr()), 0, &mut h_disk_write);

            let mut h_gpu_util = PDH_HCOUNTER::default();
            let gpu_path = wstr("\\GPU Engine(*_engtype_3d)\\Utilization Percentage");
            if PdhAddEnglishCounterW(h_query, PCWSTR(gpu_path.as_ptr()), 0, &mut h_gpu_util) != 0 {
                let fallback_path = wstr("\\GPU Engine(*)\\Utilization Percentage");
                let _ = PdhAddEnglishCounterW(h_query, PCWSTR(fallback_path.as_ptr()), 0, &mut h_gpu_util);
            }

            let _ = PdhCollectQueryData(h_query);

            Some(PdhSystemQuery {
                h_query,
                h_disk_read,
                h_disk_write,
                h_gpu_util,
            })
        }
    }

    fn poll(&self) -> (f64, f64, f32) {
        unsafe {
            if PdhCollectQueryData(self.h_query) != 0 {
                return (0.0, 0.0, 0.0);
            }

            let mut read_val = windows::Win32::System::Performance::PDH_FMT_COUNTERVALUE::default();
            let r_status = PdhGetFormattedCounterValue(self.h_disk_read, PDH_FMT_DOUBLE, None, &mut read_val);
            let disk_read = if r_status == 0 { read_val.Anonymous.doubleValue } else { 0.0 };

            let mut write_val = windows::Win32::System::Performance::PDH_FMT_COUNTERVALUE::default();
            let w_status = PdhGetFormattedCounterValue(self.h_disk_write, PDH_FMT_DOUBLE, None, &mut write_val);
            let disk_write = if w_status == 0 { write_val.Anonymous.doubleValue } else { 0.0 };

            let mut gpu_util: f64 = 0.0;
            let mut buffer_size: u32 = 0;
            let mut item_count: u32 = 0;

            let status = PdhGetFormattedCounterArrayW(
                self.h_gpu_util,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                None
            );

            if (status == 0 || status == 0x800007D2) && buffer_size > 0 && item_count > 0 {
                let mut buffer = vec![0u8; buffer_size as usize];
                let status = PdhGetFormattedCounterArrayW(
                    self.h_gpu_util,
                    PDH_FMT_DOUBLE,
                    &mut buffer_size,
                    &mut item_count,
                    Some(buffer.as_mut_ptr() as *mut _)
                );

                if status == 0 && item_count > 0 {
                    let items = std::slice::from_raw_parts(
                        buffer.as_ptr() as *const windows::Win32::System::Performance::PDH_FMT_COUNTERVALUE_ITEM_W,
                        item_count as usize
                    );
                    for item in items {
                        if !item.szName.is_null() {
                            let mut len = 0;
                            while *item.szName.0.add(len) != 0 {
                                len += 1;
                            }
                            let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.szName.0, len));
                            if name.contains("engtype_3d") || name.contains("3D") {
                                gpu_util += item.FmtValue.Anonymous.doubleValue;
                            }
                        }
                    }
                }
            }

            (disk_read, disk_write, gpu_util as f32)
        }
    }
}

fn make_window_click_through(hwnd: HWND) {
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            ex_style | (WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0) as i32
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window: WebviewWindow = app.get_webview_window("main").unwrap();

            // Set window as click-through on Windows
            if let Ok(hwnd) = window.hwnd() {
                make_window_click_through(hwnd);
            }

            let window_clone = window.clone();
            std::thread::spawn(move || {
                // CRITICAL: Wait for WebView2 to fully initialize before touching
                // any Win32 system APIs. System::new() and PDH both call heavy
                // Win32 enumeration that races with WebView2 compositor init.
                std::thread::sleep(std::time::Duration::from_secs(3));

                // Use System::new() (lightweight) instead of System::new_all()
                // which does massive process/component enumeration
                let mut sys = System::new();
                // Prime CPU measurement (first reading is always 0)
                sys.refresh_cpu();
                std::thread::sleep(std::time::Duration::from_millis(200));

                let mut networks = sysinfo::Networks::new_with_refreshed_list();
                let pdh_query = PdhSystemQuery::new();

                let mut last_clock_rect = RECT::default();

                let mut screen_width: u32 = 1920;
                let mut screen_height: u32 = 1080;
                if let Ok(Some(monitor)) = window_clone.current_monitor() {
                    let size = monitor.size();
                    screen_width = size.width;
                    screen_height = size.height;
                }

                loop {
                    // Check if foreground window is fullscreen
                    let is_fullscreen = is_foreground_window_fullscreen(screen_width, screen_height);
                    if is_fullscreen {
                        let _ = window_clone.hide();
                    } else {
                        let _ = window_clone.show();

                        // Recalculate positioning based on clock rectangle changes
                        if let Some(clock_rect) = get_clock_rect() {
                            if clock_rect.left != last_clock_rect.left
                                || clock_rect.top != last_clock_rect.top
                                || clock_rect.right != last_clock_rect.right
                                || clock_rect.bottom != last_clock_rect.bottom
                            {
                                last_clock_rect = clock_rect;
                                if let Ok(hwnd) = window_clone.hwnd() {
                                    let height = clock_rect.bottom - clock_rect.top;
                                    let y = clock_rect.top + (height - 36) / 2;
                                    let mut x = clock_rect.right + 6;
                                    let x_max = screen_width as i32 - 340;
                                    if x > x_max {
                                        x = x_max;
                                    }

                                    unsafe {
                                        let _ = SetWindowPos(
                                            hwnd,
                                            None,
                                            x,
                                            y,
                                            340,
                                            36,
                                            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSIZE | SWP_SHOWWINDOW
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // Poll and refresh stats
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    networks.refresh();

                    let cpu = sys.global_cpu_info().cpu_usage();

                    let total_mem = sys.total_memory() as f64;
                    let used_mem = (sys.total_memory() - sys.free_memory()) as f64;
                    let ram_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 } as f32;

                    let mut net_recv = 0.0;
                    let mut net_sent = 0.0;
                    for (_name, net) in &networks {
                        net_recv += net.received() as f64;
                        net_sent += net.transmitted() as f64;
                    }

                    let (disk_read, disk_write, gpu) = if let Some(ref q) = pdh_query {
                        q.poll()
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                    let stats = SystemStats {
                        cpu,
                        ram_pct,
                        ram_used: used_mem / (1024.0 * 1024.0 * 1024.0),
                        ram_total: total_mem / (1024.0 * 1024.0 * 1024.0),
                        disk_read,
                        disk_write,
                        net_recv,
                        net_sent,
                        gpu,
                    };

                    let _ = window_clone.emit("stats-update", stats);

                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
