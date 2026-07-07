//! # Chronos Clipboard Perception Adapter
//!
//! Observes clipboard transitions natively on Windows, classifies content types,
//! and publishes information flow events onto the Cognitive Bus.

use chrono::{DateTime, Utc};
use chronos_bus::EventBus;
use chronos_core::ChronosEvent;
use chronos_logging::ChronosLogger;
use chronos_registry::{ServiceDescriptor, ServiceRegistry, ServiceType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Errors that can occur within the Clipboard Adapter.
#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Bus error: {0}")]
    Bus(String),
    #[error("IO error: {0}")]
    Io(String),
}

/// Normalized details of the active clipboard state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardDetails {
    pub content_type: String,
    pub content_hash: String,
    pub source_application: String,
    pub content_size: u64,
    pub file_count: u32,
    pub text_snippet: Option<String>,
}

/// Normalizes raw clipboard states into standard ChronosEvents.
pub struct ClipboardEventNormalizer;

impl ClipboardEventNormalizer {
    pub fn normalize(
        details: &ClipboardDetails,
        event_type: &str,
        timestamp: DateTime<Utc>,
    ) -> ChronosEvent {
        let payload = json!({
            "timestamp": timestamp.to_rfc3339(),
            "content_type": details.content_type,
            "content_hash": details.content_hash,
            "source_application": details.source_application,
            "content_size": details.content_size,
            "file_count": details.file_count,
        });

        ChronosEvent::new(event_type, "ClipboardAdapter", payload)
    }
}

pub struct ClipboardObserver {
    registry: Arc<ServiceRegistry>,
    bus: Arc<dyn EventBus>,
    logger: ChronosLogger,
    last_hash: Arc<Mutex<Option<String>>>,
}

impl ClipboardObserver {
    pub fn new(
        registry: Arc<ServiceRegistry>,
        bus: Arc<dyn EventBus>,
        logger: ChronosLogger,
    ) -> Self {
        Self {
            registry,
            bus,
            logger,
            last_hash: Arc::new(Mutex::new(None)),
        }
    }

    /// Registers clipboard observation capabilities and starts active polling.
    pub async fn start(&self) -> Result<(), ClipboardError> {
        let desc = ServiceDescriptor::new(
            "chronos-adapter-clipboard",
            "Clipboard Adapter",
            ServiceType::Adapter,
            "1.0.0",
            vec!["ObserveClipboard".to_string()],
            vec![],
            vec![
                "ClipboardChanged".to_string(),
                "ClipboardTextCopied".to_string(),
                "ClipboardFileCopied".to_string(),
                "ClipboardUriCopied".to_string(),
                "ClipboardImageCopied".to_string(),
            ],
        );

        self.registry.register(desc)
            .await
            .map_err(|e| ClipboardError::Registry(e.to_string()))?;

        self.logger.info("Clipboard Adapter started and registered.", None);

        let bus = self.bus.clone();
        let logger = self.logger.clone();
        let last_hash = self.last_hash.clone();

        // Spawn polling observer thread
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                if let Some(details) = get_active_clipboard_details() {
                    let mut lock = last_hash.lock().unwrap();

                    let is_new = match &*lock {
                        Some(h) => h != &details.content_hash,
                        None => true,
                    };

                    if is_new {
                        *lock = Some(details.content_hash.clone());
                        let now = Utc::now();

                        // Map to sub-events based on classification
                        let sub_type = match details.content_type.as_str() {
                            "text" => "ClipboardTextCopied",
                            "uri" => "ClipboardUriCopied",
                            "files" => "ClipboardFileCopied",
                            "image" => "ClipboardImageCopied",
                            _ => "ClipboardChanged",
                        };

                        let change_evt = ClipboardEventNormalizer::normalize(&details, "ClipboardChanged", now);
                        let sub_evt = ClipboardEventNormalizer::normalize(&details, sub_type, now);

                        let _ = bus.publish(change_evt);
                        let _ = bus.publish(sub_evt);

                        logger.info(&format!("Clipboard transition detected: {}", sub_type), None);
                    }
                }
            }
        });

        Ok(())
    }
}

// Native Windows Clipboard implementation
#[cfg(target_os = "windows")]
fn get_active_clipboard_details() -> Option<ClipboardDetails> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, GetClipboardOwner, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows_sys::Win32::UI::Shell::DragQueryFileW;

    unsafe {
        if OpenClipboard(0) == 0 {
            return None;
        }

        let mut details = None;

        // Try extracting Owner Window application metadata
        let owner_hwnd = GetClipboardOwner();
        let mut source_application = "Unknown".to_string();
        if owner_hwnd != 0 {
            let mut process_id = 0u32;
            GetWindowThreadProcessId(owner_hwnd, &mut process_id);
            if process_id != 0 {
                let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
                if process_handle != 0 {
                    let mut path_buf = [0u16; 1024];
                    let len = GetModuleFileNameExW(process_handle, 0, path_buf.as_mut_ptr(), path_buf.len() as u32);
                    if len > 0 {
                        let path = String::from_utf16_lossy(&path_buf[..len as usize]);
                        if let Some(name) = std::path::Path::new(&path).file_name() {
                            source_application = name.to_string_lossy().to_string();
                        }
                    }
                    windows_sys::Win32::Foundation::CloseHandle(process_handle);
                }
            }
        }

        // CF_UNICODETEXT = 13
        if IsClipboardFormatAvailable(13) != 0 {
            let h_data = GetClipboardData(13);
            if h_data != 0 {
                let h_mem = h_data as *mut core::ffi::c_void;
                let size = GlobalSize(h_mem) as u64;
                let ptr = GlobalLock(h_mem) as *const u16;
                if !ptr.is_null() {
                    let wstr_len = (size as usize) / 2;
                    let mut slice = std::slice::from_raw_parts(ptr, wstr_len);
                    // Trim trailing null characters
                    if let Some(null_idx) = slice.iter().position(|&x| x == 0) {
                        slice = &slice[..null_idx];
                    }
                    let text = String::from_utf16_lossy(slice);
                    GlobalUnlock(h_mem);

                    if !text.trim().is_empty() {
                        let is_uri = text.starts_with("http://")
                            || text.starts_with("https://")
                            || text.starts_with("file://");

                        let content_type = if is_uri { "uri" } else { "text" };
                        
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        text.hash(&mut hasher);
                        let content_hash = format!("{:x}", hasher.finish());

                        details = Some(ClipboardDetails {
                            content_type: content_type.to_string(),
                            content_hash,
                            source_application,
                            content_size: size,
                            file_count: 0,
                            text_snippet: Some(if text.len() > 60 { text[..60].to_string() } else { text }),
                        });
                    }
                }
            }
        }
        // CF_HDROP = 15
        else if IsClipboardFormatAvailable(15) != 0 {
            let h_drop = GetClipboardData(15);
            if h_drop != 0 {
                let count = DragQueryFileW(h_drop, 0xFFFFFFFF, std::ptr::null_mut(), 0);
                if count > 0 {
                    let mut paths = Vec::new();
                    for i in 0..count {
                        let len = DragQueryFileW(h_drop, i, std::ptr::null_mut(), 0);
                        if len > 0 {
                            let mut buf = vec![0u16; (len + 1) as usize];
                            DragQueryFileW(h_drop, i, buf.as_mut_ptr(), buf.len() as u32);
                            paths.push(String::from_utf16_lossy(&buf[..len as usize]));
                        }
                    }

                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    paths.hash(&mut hasher);
                    let content_hash = format!("{:x}", hasher.finish());
                    let h_mem = h_drop as *mut core::ffi::c_void;
                    let content_size = GlobalSize(h_mem) as u64;

                    details = Some(ClipboardDetails {
                        content_type: "files".to_string(),
                        content_hash,
                        source_application,
                        content_size,
                        file_count: count,
                        text_snippet: None,
                    });
                }
            }
        }
        // CF_DIB = 8, CF_DIBV5 = 17
        else if IsClipboardFormatAvailable(8) != 0 || IsClipboardFormatAvailable(17) != 0 {
            let format = if IsClipboardFormatAvailable(17) != 0 { 17 } else { 8 };
            let h_data = GetClipboardData(format);
            if h_data != 0 {
                let h_mem = h_data as *mut core::ffi::c_void;
                let content_size = GlobalSize(h_mem) as u64;
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                content_size.hash(&mut hasher);
                let content_hash = format!("img-{:x}", hasher.finish());

                details = Some(ClipboardDetails {
                    content_type: "image".to_string(),
                    content_hash,
                    source_application,
                    content_size,
                    file_count: 0,
                    text_snippet: None,
                });
            }
        }

        CloseClipboard();
        details
    }
}

// Fallback Mock for Non-Windows compile targets (tests)
#[cfg(not(target_os = "windows"))]
fn get_active_clipboard_details() -> Option<ClipboardDetails> {
    static mut POLL_COUNT: u32 = 0;
    unsafe {
        POLL_COUNT += 1;
        match POLL_COUNT % 4 {
            0 => Some(ClipboardDetails {
                content_type: "text".to_string(),
                content_hash: "mock-txt-1".to_string(),
                source_application: "Code.exe".to_string(),
                content_size: 128,
                file_count: 0,
                text_snippet: Some("fn main()".to_string()),
            }),
            1 => Some(ClipboardDetails {
                content_type: "uri".to_string(),
                content_hash: "mock-uri-2".to_string(),
                source_application: "chrome.exe".to_string(),
                content_size: 64,
                file_count: 0,
                text_snippet: Some("https://google.com".to_string()),
            }),
            2 => Some(ClipboardDetails {
                content_type: "files".to_string(),
                content_hash: "mock-files-3".to_string(),
                source_application: "explorer.exe".to_string(),
                content_size: 1024,
                file_count: 2,
                text_snippet: None,
            }),
            _ => Some(ClipboardDetails {
                content_type: "image".to_string(),
                content_hash: "mock-img-4".to_string(),
                source_application: "mspaint.exe".to_string(),
                content_size: 20480,
                file_count: 0,
                text_snippet: None,
            }),
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
    async fn test_clipboard_transitions() {
        let registry = Arc::new(ServiceRegistry::new());
        let bus = Arc::new(MemoryEventBus::new(100));
        let logger = ChronosLogger::new(LogContext::new());

        let observer = ClipboardObserver::new(registry, bus.clone(), logger);
        let mut sub = bus.subscribe();

        observer.start().await.unwrap();

        let mut got_change = false;
        let mut got_text = false;

        // Verify clipboard transitions are successfully published
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(Ok(event)) = tokio::time::timeout(Duration::from_millis(50), sub.next_event()).await {
                if event.event_type == "ClipboardChanged" {
                    got_change = true;
                }
                if event.event_type == "ClipboardTextCopied" || event.event_type == "ClipboardUriCopied" {
                    got_text = true;
                }
            }
        }

        assert!(got_change || got_text);
        let _ = got_change;
        let _ = got_text;
    }
}
