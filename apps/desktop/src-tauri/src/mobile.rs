use chrono::Utc;
use mobile_runtime::{
    MobileBounds, MobileCapture, MobileDeviceSession, MobileDeviceType, MobileFrame,
    MobileObservation, MobilePlatform, MobileSessionStatus, MobileUiSnapshot, PrivacyClass,
    RawMobileElement, ScreenFrameBuffer,
};
use quick_xml::{Reader, events::Event};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MobileHostError {
    #[error("ADB was not found")]
    AdbMissing,
    #[error("no online Android Emulator was found")]
    NoOnlineEmulator,
    #[error("the foreground application could not be identified")]
    FocusUnavailable,
    #[error("the device screen size could not be parsed")]
    InvalidScreenSize,
    #[error("the mobile application allowlist is empty")]
    EmptyAllowlist,
    #[error("foreground package is not allowed: {0}")]
    PackageNotAllowed(String),
    #[error("ADB command failed: {0}")]
    CommandFailed(String),
    #[error("UI tree could not be parsed: {0}")]
    InvalidUiTree(String),
}

#[derive(Clone, Debug)]
struct AdbObservationOutput {
    device_id: String,
    os_version: String,
    size_output: String,
    focus_output: String,
    ui_xml: String,
    png: Vec<u8>,
}

pub struct MobileHost {
    adb_path: Option<PathBuf>,
    session_id: String,
    frames: Mutex<ScreenFrameBuffer>,
}

impl MobileHost {
    pub fn discover() -> Self {
        Self::new(resolve_adb_path())
    }

    pub fn new(adb_path: Option<PathBuf>) -> Self {
        Self {
            adb_path,
            session_id: format!("mobile_session_{}", Uuid::now_v7()),
            frames: Mutex::new(ScreenFrameBuffer::new(5)),
        }
    }

    pub fn adb_available_with_sdk(&self, android_sdk: Option<&str>) -> bool {
        self.adb_path.is_some()
            || android_sdk
                .map(adb_from_sdk_path)
                .is_some_and(|path| path.is_file())
    }

    pub fn observe_with_sdk(
        &self,
        android_sdk: Option<&str>,
        allowed_apps: &[String],
    ) -> Result<MobileCapture, MobileHostError> {
        let configured_adb = android_sdk
            .map(adb_from_sdk_path)
            .filter(|path| path.is_file());
        let adb = self
            .adb_path
            .as_deref()
            .or(configured_adb.as_deref())
            .ok_or(MobileHostError::AdbMissing)?;
        let devices = run_text(adb, &["devices"])?;
        let device_id = online_emulator_from_output(&devices)?;
        let os_version = run_text(
            adb,
            &[
                "-s",
                &device_id,
                "shell",
                "getprop",
                "ro.build.version.release",
            ],
        )?;
        let size_output = run_text(adb, &["-s", &device_id, "shell", "wm", "size"])?;
        let focus_output = run_text(
            adb,
            &["-s", &device_id, "shell", "dumpsys", "window", "windows"],
        )?;
        run_text(
            adb,
            &[
                "-s",
                &device_id,
                "shell",
                "uiautomator",
                "dump",
                "/sdcard/ordinconn-ui.xml",
            ],
        )?;
        let ui_xml = run_text(
            adb,
            &["-s", &device_id, "shell", "cat", "/sdcard/ordinconn-ui.xml"],
        )?;
        let png = run_bytes(adb, &["-s", &device_id, "exec-out", "screencap", "-p"])?;
        let capture = bind_capture_to_session(
            capture_from_outputs(
                &AdbObservationOutput {
                    device_id,
                    os_version: os_version.trim().into(),
                    size_output,
                    focus_output,
                    ui_xml,
                    png,
                },
                allowed_apps,
            )?,
            &self.session_id,
        );
        self.frames
            .lock()
            .map_err(|_| MobileHostError::CommandFailed("frame buffer lock poisoned".into()))?
            .push(capture.frame.clone());
        Ok(capture)
    }
}

fn adb_from_sdk_path(value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.file_name().is_some_and(|name| name == "adb") {
        path
    } else {
        path.join("platform-tools/adb")
    }
}

fn bind_capture_to_session(mut capture: MobileCapture, session_id: &str) -> MobileCapture {
    capture.session.session_id = session_id.into();
    capture.snapshot.session_id = session_id.into();
    capture.frame.session_id = session_id.into();
    capture.observation.device_session_id = session_id.into();
    capture
}

fn resolve_adb_path() -> Option<PathBuf> {
    for key in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
        if let Ok(root) = env::var(key) {
            let candidate = PathBuf::from(root).join("platform-tools/adb");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    if let Ok(user_home) = env::var("HOME") {
        let candidate = PathBuf::from(user_home).join("Library/Android/sdk/platform-tools/adb");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|path| path.join("adb"))
            .find(|candidate| candidate.is_file())
    })
}

fn run_text(adb: &Path, args: &[&str]) -> Result<String, MobileHostError> {
    let output = Command::new(adb)
        .args(args)
        .output()
        .map_err(|error| MobileHostError::CommandFailed(error.to_string()))?;
    if !output.status.success() {
        return Err(MobileHostError::CommandFailed(
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(240)
                .collect(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_bytes(adb: &Path, args: &[&str]) -> Result<Vec<u8>, MobileHostError> {
    let output = Command::new(adb)
        .args(args)
        .output()
        .map_err(|error| MobileHostError::CommandFailed(error.to_string()))?;
    if !output.status.success() {
        return Err(MobileHostError::CommandFailed(
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(240)
                .collect(),
        ));
    }
    Ok(output.stdout)
}

fn parse_online_emulators(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let id = fields.next()?;
            let status = fields.next()?;
            (id.starts_with("emulator-") && status == "device").then(|| id.to_owned())
        })
        .collect()
}

fn online_emulator_from_output(output: &str) -> Result<String, MobileHostError> {
    parse_online_emulators(output)
        .into_iter()
        .next()
        .ok_or(MobileHostError::NoOnlineEmulator)
}

fn parse_screen_size(output: &str) -> Option<(u32, u32)> {
    output.lines().rev().find_map(|line| {
        let value = line.split_once(':')?.1.trim();
        let (width, height) = value.split_once('x')?;
        Some((width.parse().ok()?, height.parse().ok()?))
    })
}

fn parse_focused_app(output: &str) -> Option<(String, String)> {
    let line = output
        .lines()
        .find(|line| line.contains("mCurrentFocus") || line.contains("mFocusedApp"))?;
    let component = line
        .split_whitespace()
        .find(|field| field.contains('/') && !field.starts_with("Window"))?
        .trim_end_matches('}');
    let (package_name, activity) = component.split_once('/')?;
    Some((package_name.into(), activity.into()))
}

fn parse_uiautomator_xml(xml: &str) -> Result<Vec<RawMobileElement>, MobileHostError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut elements = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Empty(node)) | Ok(Event::Start(node)) if node.name().as_ref() == b"node" => {
                let mut values = HashMap::new();
                for attribute in node.attributes().with_checks(false) {
                    let attribute = attribute
                        .map_err(|error| MobileHostError::InvalidUiTree(error.to_string()))?;
                    let key = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
                    let value = attribute
                        .decode_and_unescape_value(reader.decoder())
                        .map_err(|error| MobileHostError::InvalidUiTree(error.to_string()))?
                        .into_owned();
                    values.insert(key, value);
                }
                let bounds =
                    parse_bounds(values.get("bounds").map(String::as_str).unwrap_or_default())
                        .ok_or_else(|| MobileHostError::InvalidUiTree("invalid bounds".into()))?;
                let class_name = values.remove("class").unwrap_or_default();
                elements.push(RawMobileElement {
                    text: optional_value(values.remove("text")),
                    role: class_name
                        .rsplit('.')
                        .next()
                        .unwrap_or("node")
                        .to_ascii_lowercase(),
                    class_name,
                    content_description: optional_value(values.remove("content-desc")),
                    bounds,
                    clickable: bool_value(values.get("clickable")),
                    scrollable: bool_value(values.get("scrollable")),
                    enabled: bool_value(values.get("enabled")),
                    focused: bool_value(values.get("focused")),
                    selected: bool_value(values.get("selected")),
                    resource_id: optional_value(values.remove("resource-id")),
                    password: bool_value(values.get("password")),
                });
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(MobileHostError::InvalidUiTree(error.to_string())),
        }
    }
    Ok(elements)
}

use std::collections::HashMap;

fn parse_bounds(value: &str) -> Option<MobileBounds> {
    let normalized = value.replace("][", ",").replace(['[', ']'], "");
    let numbers = normalized
        .split(',')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if numbers.len() != 4 || numbers[2] < numbers[0] || numbers[3] < numbers[1] {
        return None;
    }
    Some(MobileBounds {
        x: numbers[0],
        y: numbers[1],
        width: numbers[2] - numbers[0],
        height: numbers[3] - numbers[1],
    })
}

fn bool_value(value: Option<&String>) -> bool {
    value.is_some_and(|value| value == "true")
}

fn optional_value(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn capture_from_outputs(
    output: &AdbObservationOutput,
    allowed_apps: &[String],
) -> Result<MobileCapture, MobileHostError> {
    if allowed_apps.is_empty() {
        return Err(MobileHostError::EmptyAllowlist);
    }
    let (width, height) =
        parse_screen_size(&output.size_output).ok_or(MobileHostError::InvalidScreenSize)?;
    let (package_name, activity) =
        parse_focused_app(&output.focus_output).ok_or(MobileHostError::FocusUnavailable)?;
    if !allowed_apps.iter().any(|allowed| allowed == &package_name) {
        return Err(MobileHostError::PackageNotAllowed(package_name));
    }
    let session_id = format!("mobile_session_{}", Uuid::now_v7());
    let now = Utc::now();
    let snapshot = MobileUiSnapshot::from_elements(
        &session_id,
        &package_name,
        &activity,
        width,
        height,
        parse_uiautomator_xml(&output.ui_xml)?,
    );
    let frame = MobileFrame::from_png(&session_id, width, height, output.png.clone());
    let observation = MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        None,
        "generic-android",
        PrivacyClass::UserAllowed,
    );
    Ok(MobileCapture {
        session: MobileDeviceSession {
            session_id,
            device_id: output.device_id.clone(),
            platform: MobilePlatform::Android,
            device_type: MobileDeviceType::Emulator,
            os_version: output.os_version.clone(),
            screen_width: width,
            screen_height: height,
            connected_at: now,
            current_app: Some(package_name),
            current_activity: Some(activity),
            status: MobileSessionStatus::Connected,
            last_observation_at: Some(now),
        },
        snapshot,
        frame,
        observation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICES: &str = "List of devices attached\nemulator-5554\tdevice\nemulator-5556\toffline\nR58M123\tdevice\n";
    const FOCUS: &str =
        "mCurrentFocus=Window{123 u0 com.example.news/com.example.news.MainActivity}";
    const XML: &str = r#"<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><hierarchy rotation="0"><node index="0" text="BTC ETF inflows" resource-id="com.example.news:id/title" class="android.widget.TextView" package="com.example.news" content-desc="headline" clickable="false" enabled="true" bounds="[10,20][900,100]"/><node index="1" text="secret" password="true" resource-id="com.example.news:id/password" class="android.widget.EditText" package="com.example.news" clickable="true" enabled="true" bounds="[10,120][900,200]"/></hierarchy>"#;

    #[test]
    fn adb_parsers_select_only_online_emulators() {
        assert_eq!(parse_online_emulators(DEVICES), vec!["emulator-5554"]);
        assert_eq!(
            parse_screen_size("Physical size: 1080x2400\nOverride size: 900x2000"),
            Some((900, 2000))
        );
        assert_eq!(
            parse_focused_app(FOCUS),
            Some((
                "com.example.news".into(),
                "com.example.news.MainActivity".into()
            ))
        );
        assert_eq!(parse_screen_size("size unknown"), None);
        assert_eq!(parse_focused_app("no focused window"), None);
    }

    #[test]
    fn uiautomator_parser_redacts_password_nodes() {
        let elements = parse_uiautomator_xml(XML).unwrap();
        assert_eq!(elements.len(), 2);
        assert!(elements[1].password);
        let snapshot = mobile_runtime::MobileUiSnapshot::from_elements(
            "session-1",
            "com.example.news",
            ".MainActivity",
            1080,
            2400,
            elements,
        );
        assert_eq!(snapshot.elements[1].text.as_deref(), Some("[REDACTED]"));
    }

    #[test]
    fn fixture_capture_requires_an_explicit_app_allowlist() {
        let output = AdbObservationOutput {
            device_id: "emulator-5554".into(),
            os_version: "15".into(),
            size_output: "Physical size: 1080x2400".into(),
            focus_output: FOCUS.into(),
            ui_xml: XML.into(),
            png: vec![1, 2, 3],
        };
        assert_eq!(
            capture_from_outputs(&output, &[]).unwrap_err(),
            MobileHostError::EmptyAllowlist
        );
        assert_eq!(
            capture_from_outputs(&output, &["org.telegram.messenger".into()]).unwrap_err(),
            MobileHostError::PackageNotAllowed("com.example.news".into())
        );
        let capture = capture_from_outputs(&output, &["com.example.news".into()]).unwrap();
        assert_eq!(capture.session.device_id, "emulator-5554");
        assert_eq!(capture.observation.package_name, "com.example.news");
        assert_eq!(capture.observation.visible_facts, Vec::<String>::new());
    }

    #[test]
    fn missing_adb_and_no_emulator_are_truthful_errors() {
        assert_eq!(
            MobileHost::new(None)
                .observe_with_sdk(None, &[])
                .unwrap_err(),
            MobileHostError::AdbMissing
        );
        assert_eq!(
            online_emulator_from_output("List of devices attached\n").unwrap_err(),
            MobileHostError::NoOnlineEmulator
        );
    }

    #[test]
    fn real_emulator_smoke_is_explicitly_gated() {
        if std::env::var("ORDINCONN_MOBILE_SMOKE").as_deref() != Ok("1") {
            return;
        }
        let host = MobileHost::discover();
        let allowed = std::env::var("ORDINCONN_MOBILE_ALLOWED_APPS")
            .unwrap_or_default()
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        host.observe_with_sdk(None, &allowed)
            .expect("real Android Emulator observation");
    }

    #[test]
    fn sdk_configuration_and_session_identity_are_stable() {
        assert_eq!(
            adb_from_sdk_path("/opt/android"),
            PathBuf::from("/opt/android/platform-tools/adb")
        );
        assert_eq!(
            adb_from_sdk_path("/opt/android/platform-tools/adb"),
            PathBuf::from("/opt/android/platform-tools/adb")
        );
        let host = MobileHost::new(None);
        assert!(host.session_id.starts_with("mobile_session_"));
    }
}
