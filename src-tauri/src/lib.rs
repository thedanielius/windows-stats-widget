use tauri::{Emitter, Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, FindWindowExW, GetWindowRect, SetWindowPos,
    GetWindowLongW, SetWindowLongW,
    GWL_EXSTYLE, WS_EX_TRANSPARENT, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    SWP_NOACTIVATE, SWP_NOCOPYBITS, HWND_TOPMOST,
    SWP_NOMOVE, SWP_NOSIZE, SWP_FRAMECHANGED,
};
use windows::Win32::System::Performance::{
    PdhOpenQueryW, PdhAddEnglishCounterW, PdhCollectQueryData,
    PdhGetFormattedCounterValue, PdhGetFormattedCounterArrayW,
    PDH_FMT_DOUBLE, PDH_HQUERY, PDH_HCOUNTER,
    PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W,
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

            let mut read_val = PDH_FMT_COUNTERVALUE::default();
            let r_status = PdhGetFormattedCounterValue(self.h_disk_read, PDH_FMT_DOUBLE, None, &mut read_val);
            let disk_read = if r_status == 0 { read_val.Anonymous.doubleValue } else { 0.0 };

            let mut write_val = PDH_FMT_COUNTERVALUE::default();
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
                        buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
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
        let new_style = ex_style | (WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as i32;
        SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);

        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let window: WebviewWindow = app.get_webview_window("main").unwrap();

            if let Ok(hwnd) = window.hwnd() {
                make_window_click_through(hwnd);
            }

            let window_clone = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));

                let mut sys = System::new();
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

                let widget_width: i32 = 370;
                let widget_height: i32 = 36;
                let tray_padding: i32 = 8;

                let mut was_hidden = true;
                let mut tick: u32 = 0;

                loop {
                    if !window_clone.is_visible().unwrap_or(false) {
                        let _ = window_clone.show();
                        was_hidden = true;
                    }

                    let clock_rect = get_clock_rect();
                    let (x, y) = if let Some(rect) = clock_rect {
                        let tray_height = rect.bottom - rect.top;
                        let cy = rect.top + (tray_height - widget_height) / 2 + 2;
                        let mut cx = rect.left - widget_width - tray_padding;
                        if cx < 0 { cx = 0; }
                        (cx, cy)
                    } else {
                        let default_x = (screen_width as i32 - widget_width - 60).max(0);
                        (default_x, (screen_height as i32 - widget_height).max(0))
                    };

                    let clock_changed = clock_rect.is_none_or(|r| {
                        r.left != last_clock_rect.left
                            || r.top != last_clock_rect.top
                            || r.right != last_clock_rect.right
                            || r.bottom != last_clock_rect.bottom
                    });

                    if let Ok(hwnd) = window_clone.hwnd() {
                        unsafe {
                            let _ = SetWindowPos(
                                hwnd,
                                Some(HWND_TOPMOST),
                                x, y,
                                widget_width, widget_height,
                                SWP_NOACTIVATE | SWP_NOCOPYBITS
                            );
                        }
                    }

                    if clock_changed || was_hidden {
                        if let Some(r) = clock_rect {
                            last_clock_rect = r;
                        }
                        was_hidden = false;
                    }

                    if tick.is_multiple_of(2) {
                        sys.refresh_cpu();
                        sys.refresh_memory();
                        networks.refresh();

                        let cpu = sys.global_cpu_info().cpu_usage();

                        let total_mem = sys.total_memory() as f64;
                        let used_mem = (sys.total_memory() - sys.free_memory()) as f64;
                        let ram_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 } as f32;

                        let mut net_recv = 0.0f64;
                        let mut net_sent = 0.0f64;
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
                    }

                    tick += 1;
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
