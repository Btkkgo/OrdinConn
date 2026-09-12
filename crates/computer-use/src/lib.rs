use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputerUseCapabilities {
    pub observe_screen: bool,
    pub observe_window: bool,
    pub capture_screen: bool,
    pub open_url: bool,
    pub click: bool,
    pub double_click: bool,
    pub type_text: bool,
    pub hotkey: bool,
    pub scroll: bool,
    pub open_application: bool,
    pub capture_region: bool,
}

impl ComputerUseCapabilities {
    pub const fn v0_1() -> Self {
        Self {
            observe_screen: true,
            observe_window: true,
            capture_screen: true,
            open_url: true,
            click: false,
            double_click: false,
            type_text: false,
            hotkey: false,
            scroll: false,
            open_application: false,
            capture_region: false,
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ComputerUseError {
    #[error("computer-use capability is unavailable in V0.1")]
    Unavailable,
    #[error("computer-use operation failed")]
    OperationFailed,
    #[error("invalid URL")]
    InvalidUrl,
}

#[async_trait]
pub trait ComputerUse: Send + Sync {
    fn capabilities(&self) -> ComputerUseCapabilities;
    async fn observe_window(&self) -> Result<String, ComputerUseError>;
    async fn capture_screen(&self, destination: &Path) -> Result<(), ComputerUseError>;
    async fn open_url(&self, url: &str) -> Result<(), ComputerUseError>;
}

#[derive(Default)]
pub struct LocalComputerUse;

#[cfg(target_os = "macos")]
#[async_trait]
impl ComputerUse for LocalComputerUse {
    fn capabilities(&self) -> ComputerUseCapabilities {
        ComputerUseCapabilities::v0_1()
    }

    async fn observe_window(&self) -> Result<String, ComputerUseError> {
        let output = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to get name of first process whose frontmost is true",
            ])
            .output()
            .map_err(|_| ComputerUseError::OperationFailed)?;
        if !output.status.success() {
            return Err(ComputerUseError::OperationFailed);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    async fn capture_screen(&self, destination: &Path) -> Result<(), ComputerUseError> {
        let status = std::process::Command::new("screencapture")
            .arg("-x")
            .arg(destination)
            .status()
            .map_err(|_| ComputerUseError::OperationFailed)?;
        status
            .success()
            .then_some(())
            .ok_or(ComputerUseError::OperationFailed)
    }

    async fn open_url(&self, url: &str) -> Result<(), ComputerUseError> {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(ComputerUseError::InvalidUrl);
        }
        let status = std::process::Command::new("open")
            .arg(url)
            .status()
            .map_err(|_| ComputerUseError::OperationFailed)?;
        status
            .success()
            .then_some(())
            .ok_or(ComputerUseError::OperationFailed)
    }
}

#[cfg(not(target_os = "macos"))]
#[async_trait]
impl ComputerUse for LocalComputerUse {
    fn capabilities(&self) -> ComputerUseCapabilities {
        let mut capabilities = ComputerUseCapabilities::v0_1();
        capabilities.observe_window = false;
        capabilities.capture_screen = false;
        capabilities.open_url = false;
        capabilities
    }

    async fn observe_window(&self) -> Result<String, ComputerUseError> {
        Err(ComputerUseError::Unavailable)
    }

    async fn capture_screen(&self, _destination: &Path) -> Result<(), ComputerUseError> {
        Err(ComputerUseError::Unavailable)
    }

    async fn open_url(&self, _url: &str) -> Result<(), ComputerUseError> {
        Err(ComputerUseError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v0_1_declares_bounded_capabilities() {
        let capabilities = ComputerUseCapabilities::v0_1();
        assert!(capabilities.observe_window);
        assert!(capabilities.capture_screen);
        assert!(capabilities.open_url);
        assert!(!capabilities.click);
        assert!(!capabilities.type_text);
    }
}
