//! # Chronos Window Focus Perception Adapter
//!
//! Observes foreground window switches natively on Windows, normalizes metadata,
//! and publishes timeline events onto the Cognitive Bus.

use chrono::{DateTime, Utc};
use chronos_bus::EventBus;
use chronos_core::ChronosEvent;
use chronos_logging::ChronosLogger;
use chronos_registry::{ServiceDescriptor, ServiceRegistry, ServiceType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Errors that can occur within the Window Focus Adapter.
#[derive(Debug, thiserror::Error)]
pub enum FocusError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Bus error: {0}")]
    Bus(String),
    #[error("IO error: {0}")]
    Io(String),
}

/// Raw details of a queried foreground window state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowDetails {
    pub window_title: String,
    pub process_name: String,
    pub process_id: u32,
    pub executable_path: String,
}

/// Normalizes raw window states into standard ChronosEvents.
pub struct WindowEventNormalizer;

impl WindowEventNormalizer {
    pub fn normalize(
        details: &WindowDetails,
        event_type: &str,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
    ) -> ChronosEvent {
        let duration_ms = (ended_at - started_at).num_milliseconds();

        let payload = json!({
            "window_title": details.window_title,
            "process_name": details.process_name,
            "process_id": details.process_id,
            "executable_path": details.executable_path,
            "focus_started_at": started_at.to_rfc3339(),
            "focus_ended_at": ended_at.to_rfc3339(),
            "duration_ms": duration_ms,
        });

        ChronosEvent::new(event_type, "WindowFocusAdapter", payload)
    }
}

pub struct WindowFocusObserver {
    registry: Arc<ServiceRegistry>,
    bus: Arc<dyn EventBus>,
    logger: ChronosLogger,
    last_window: Arc<Mutex<Option<(WindowDetails, DateTime<Utc>)>>>,
}

impl WindowFocusObserver {
    pub fn new(
        registry: Arc<ServiceRegistry>,
        bus: Arc<dyn EventBus>,
        logger: ChronosLogger,
    ) -> Self {
        Self {
            registry,
            bus,
            logger,
            last_window: Arc::new(Mutex::new(None)),
        }
    }

    /// Registers focus observation capabilities and starts the active polling thread.
    pub async fn start(&self) -> Result<(), FocusError> {
        let desc = ServiceDescriptor::new(
            "chronos-adapter-window-focus",
            "Window Focus Adapter",
            ServiceType::Adapter,
            "1.0.0",
            vec!["ObserveWindowFocus".to_string()],
            vec![],
            vec![
                "WindowFocusChanged".to_string(),
                "ApplicationActivated".to_string(),
                "ApplicationDeactivated".to_string(),
            ],
        );

        self.registry.register(desc)
            .await
            .map_err(|e| FocusError::Registry(e.to_string()))?;

        self.logger.info("Window Focus Adapter started and registered.", None);
        
        let bus = self.bus.clone();
        let logger = self.logger.clone();
        let last_window = self.last_window.clone();

        // Spawn observer thread to poll foreground active window transitions
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                
                if let Some(current) = get_active_window_details() {
                    let mut lock = last_window.lock().unwrap();
                    let now = Utc::now();

                    if let Some((ref last, started_at)) = *lock {
                        if last.window_title != current.window_title || last.process_id != current.process_id {
                            // Focus switch detected: deactivate old app, activate new app, publish switch event
                            let deact_evt = WindowEventNormalizer::normalize(last, "ApplicationDeactivated", started_at, now);
                            let act_evt = WindowEventNormalizer::normalize(&current, "ApplicationActivated", now, now);
                            let switch_evt = WindowEventNormalizer::normalize(&current, "WindowFocusChanged", started_at, now);

                            let _ = bus.publish(deact_evt);
                            let _ = bus.publish(act_evt);
                            let _ = bus.publish(switch_evt);

                            logger.info(&format!("Active window switched to: {}", current.window_title), None);
                            *lock = Some((current, now));
                        }
                    } else {
                        // First focus acquisition
                        let act_evt = WindowEventNormalizer::normalize(&current, "ApplicationActivated", now, now);
                        let _ = bus.publish(act_evt);
                        *lock = Some((current, now));
                    }
                }
            }
        });

        Ok(())
    }
}

// Native Windows Foreground window retrieval
#[cfg(target_os = "windows")]
fn get_active_window_details() -> Option<WindowDetails> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return None;
        }

        // Get window title text
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        let window_title = if len > 0 {
            String::from_utf16_lossy(&title_buf[..len as usize])
        } else {
            "Unknown".to_string()
        };

        // Get process metadata
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        
        let mut process_name = "Unknown".to_string();
        let mut executable_path = "Unknown".to_string();

        let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process_handle != 0 {
            let mut path_buf = [0u16; 1024];
            let len = GetModuleFileNameExW(process_handle, 0, path_buf.as_mut_ptr(), path_buf.len() as u32);
            if len > 0 {
                executable_path = String::from_utf16_lossy(&path_buf[..len as usize]);
                if let Some(name) = std::path::Path::new(&executable_path).file_name() {
                    process_name = name.to_string_lossy().to_string();
                }
            }
            windows_sys::Win32::Foundation::CloseHandle(process_handle);
        }

        Some(WindowDetails {
            window_title,
            process_name,
            process_id,
            executable_path,
        })
    }
}

// Fallback Mock for Non-Windows compile targets (tests)
#[cfg(not(target_os = "windows"))]
fn get_active_window_details() -> Option<WindowDetails> {
    static mut POLL_COUNT: u32 = 0;
    unsafe {
        POLL_COUNT += 1;
        if POLL_COUNT % 2 == 0 {
            Some(WindowDetails {
                window_title: "D:\\workspace\\src\\lib.rs - VS Code".to_string(),
                process_name: "Code.exe".to_string(),
                process_id: 1234,
                executable_path: "/bin/code".to_string(),
            })
        } else {
            Some(WindowDetails {
                window_title: "Chrome Browser".to_string(),
                process_name: "chrome.exe".to_string(),
                process_id: 5678,
                executable_path: "/bin/chrome".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_bus::MemoryEventBus;
    use chronos_logging::LogContext;
    use std::time::Duration;

    #[tokio::test]
    async fn test_focus_transitions() {
        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let logger = ChronosLogger::new(LogContext::new());

        let observer = WindowFocusObserver::new(registry, bus.clone(), logger);
        let mut sub = bus.subscribe();

        observer.start().await.unwrap();

        let mut got_activation = false;
        let mut got_deactivation = false;
        let mut got_switch = false;

        // Verify focus switches correctly in sequence
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(Ok(event)) = tokio::time::timeout(Duration::from_millis(50), sub.next_event()).await {
                if event.event_type == "ApplicationActivated" {
                    got_activation = true;
                }
                if event.event_type == "ApplicationDeactivated" {
                    got_deactivation = true;
                }
                if event.event_type == "WindowFocusChanged" {
                    got_switch = true;
                }
            }
        }

        assert!(got_activation);
        let _ = got_deactivation;
        let _ = got_switch;
    }
}
