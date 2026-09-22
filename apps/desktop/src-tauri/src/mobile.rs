use chrono::Utc;
use mobile_runtime::{
    AndroidAvdInfo, AndroidDeviceInfo, AndroidEnvironmentDiagnostics, MobileActionDecision,
    MobileActionDenyReason, MobileActionReceipt, MobileActionRequest, MobileActionStatus,
    MobileActionTarget, MobileBounds, MobileCapture, MobileDeviceSession, MobileDeviceType,
    MobileFrame, MobileObservation, MobilePlatform, MobileSessionStatus, MobileUiSnapshot,
    PrivacyClass, RawMobileElement, ScreenFrameBuffer, SwipeDirection, VerificationResult,
    evaluate_action,
};
use quick_xml::{Reader, events::Event};
#[cfg(test)]
use serde::Serialize;
use std::{
    env, fs,
    io::{ErrorKind, Read},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;
use uuid::Uuid;

#[cfg(test)]
const M1_5_CAPTURE_GATES: [&str; 7] = [
    "FRAME_CAPTURE",
    "UI_TREE_CAPTURE",
    "SNAPSHOT_PARSE",
    "ELEMENT_REFS",
    "MOBILE_OBSERVATION",
    "WORKSPACE_PROJECTION",
    "SESSION_SHUTDOWN",
];

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobileGateStatus {
    Pass,
    Fail,
    Blocked,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileGateCheck {
    pub name: String,
    pub status: MobileGateStatus,
    pub detail: String,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileM15GateReport {
    pub passed: bool,
    pub checks: Vec<MobileGateCheck>,
}

#[cfg(test)]
impl MobileM15GateReport {
    pub fn status(&self, name: &str) -> Option<MobileGateStatus> {
        self.checks
            .iter()
            .find(|check| check.name == name)
            .map(|check| check.status)
    }
}

#[cfg(test)]
fn gate_check(name: &str, passed: bool, detail: impl Into<String>) -> MobileGateCheck {
    MobileGateCheck {
        name: name.into(),
        status: if passed {
            MobileGateStatus::Pass
        } else {
            MobileGateStatus::Fail
        },
        detail: detail.into(),
    }
}

#[cfg(test)]
fn m1_5_environment_gate(diagnostics: &AndroidEnvironmentDiagnostics) -> MobileM15GateReport {
    let mut checks = vec![
        gate_check(
            "ADB_READY",
            diagnostics.adb_status == "ready",
            diagnostics
                .adb_path
                .clone()
                .unwrap_or_else(|| "ADB executable not detected".into()),
        ),
        gate_check(
            "EMULATOR_READY",
            diagnostics.emulator_status == "ready",
            diagnostics
                .emulator_path
                .clone()
                .unwrap_or_else(|| "emulator executable not detected".into()),
        ),
        gate_check(
            "DEVICE_ONLINE",
            !diagnostics.online_devices.is_empty(),
            if diagnostics.online_devices.is_empty() {
                "no online Android Emulator detected".into()
            } else {
                diagnostics
                    .online_devices
                    .iter()
                    .map(|device| device.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ),
    ];
    checks.extend(M1_5_CAPTURE_GATES.map(|name| MobileGateCheck {
        name: name.into(),
        status: MobileGateStatus::Blocked,
        detail: "blocked until Android environment prerequisites pass".into(),
    }));
    MobileM15GateReport {
        passed: false,
        checks,
    }
}

#[cfg(test)]
fn m1_5_capture_gate(
    diagnostics: &AndroidEnvironmentDiagnostics,
    capture: &MobileCapture,
    ipc_serializable: bool,
    session_shutdown: bool,
) -> MobileM15GateReport {
    let mut checks = m1_5_environment_gate(diagnostics).checks;
    checks.truncate(3);
    checks.extend([
        gate_check(
            "FRAME_CAPTURE",
            !capture.frame.png_bytes.is_empty() && !capture.frame.frame_hash.is_empty(),
            format!(
                "{} bytes, {}",
                capture.frame.png_bytes.len(),
                capture.frame.frame_hash
            ),
        ),
        gate_check(
            "UI_TREE_CAPTURE",
            !capture.snapshot.elements.is_empty(),
            format!("{} sanitized UI elements", capture.snapshot.elements.len()),
        ),
        gate_check(
            "SNAPSHOT_PARSE",
            !capture.snapshot.snapshot_id.is_empty()
                && !capture.snapshot.package_name.is_empty()
                && !capture.snapshot.activity.is_empty(),
            capture.snapshot.snapshot_id.clone(),
        ),
        gate_check(
            "ELEMENT_REFS",
            !capture.snapshot.elements.is_empty()
                && capture
                    .snapshot
                    .elements
                    .iter()
                    .all(|element| element.element_ref.starts_with("@e")),
            format!("{} snapshot-bound refs", capture.snapshot.elements.len()),
        ),
        gate_check(
            "MOBILE_OBSERVATION",
            !capture.observation.id.is_empty()
                && capture.observation.frame_hash == capture.frame.frame_hash,
            capture.observation.id.clone(),
        ),
        gate_check(
            "WORKSPACE_PROJECTION",
            ipc_serializable,
            "production capture persisted, audited, projected, and serialized; real Tauri command and frontend acceptance is verified separately",
        ),
        gate_check(
            "SESSION_SHUTDOWN",
            session_shutdown,
            "OrdinConn logical device session ended without terminating the user AVD",
        ),
    ]);
    MobileM15GateReport {
        passed: checks
            .iter()
            .all(|check| check.status == MobileGateStatus::Pass),
        checks,
    }
}

#[cfg(test)]
fn m1_5_capture_failure_gate(
    diagnostics: &AndroidEnvironmentDiagnostics,
    detail: impl Into<String>,
) -> MobileM15GateReport {
    let mut report = m1_5_environment_gate(diagnostics);
    report.checks[3] = MobileGateCheck {
        name: "FRAME_CAPTURE".into(),
        status: MobileGateStatus::Fail,
        detail: detail.into(),
    };
    report
}

#[derive(Clone, Debug)]
pub struct AndroidEnvironmentDetector {
    candidate_roots: Vec<PathBuf>,
    path_dirs: Vec<PathBuf>,
    avd_home: Option<PathBuf>,
}

impl AndroidEnvironmentDetector {
    pub fn production(configured_sdk: Option<&str>) -> Self {
        let mut candidate_roots = Vec::new();
        if let Some(configured) = configured_sdk.filter(|value| !value.trim().is_empty()) {
            candidate_roots.push(PathBuf::from(configured));
        }
        for key in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
            if let Ok(value) = env::var(key)
                && !value.trim().is_empty()
            {
                candidate_roots.push(PathBuf::from(value));
            }
        }
        let user_home = env::var_os("HOME").map(PathBuf::from);
        if let Some(home) = &user_home {
            candidate_roots.push(home.join("Library/Android/sdk"));
        }
        let path_dirs = env::var_os("PATH")
            .map(|paths| env::split_paths(&paths).collect())
            .unwrap_or_default();
        Self::new(
            candidate_roots,
            path_dirs,
            user_home.map(|home| home.join(".android/avd")),
        )
    }

    pub fn new(
        candidate_roots: Vec<PathBuf>,
        path_dirs: Vec<PathBuf>,
        avd_home: Option<PathBuf>,
    ) -> Self {
        Self {
            candidate_roots,
            path_dirs,
            avd_home,
        }
    }

    pub fn detect(&self) -> AndroidEnvironmentDiagnostics {
        self.detect_until(Instant::now() + Duration::from_secs(5))
    }

    fn detect_until(&self, deadline: Instant) -> AndroidEnvironmentDiagnostics {
        let sdk_roots = self
            .candidate_roots
            .iter()
            .filter_map(|candidate| normalize_sdk_root(candidate))
            .collect::<Vec<_>>();
        let adb_path = discover_adb(&self.candidate_roots, &self.path_dirs);
        let emulator_path =
            discover_tool(&sdk_roots, &self.path_dirs, "emulator/emulator", "emulator");
        let sdkmanager_path = discover_sdk_manager(&sdk_roots, &self.path_dirs, "sdkmanager");
        let avdmanager_path = discover_sdk_manager(&sdk_roots, &self.path_dirs, "avdmanager");
        let sdk_root = sdk_roots
            .iter()
            .find(|root| {
                [
                    root.join("platform-tools/adb"),
                    root.join("emulator/emulator"),
                    root.join("cmdline-tools/latest/bin/sdkmanager"),
                    root.join("tools/bin/sdkmanager"),
                ]
                .iter()
                .any(|candidate| candidate.is_file())
            })
            .cloned()
            .or_else(|| adb_path.as_deref().and_then(infer_sdk_root));
        let adb_version = adb_path
            .as_deref()
            .and_then(|adb| run_command_text_until(adb, &["version"], deadline))
            .and_then(|value| value.lines().next().map(str::to_owned));
        let online_devices = adb_path
            .as_deref()
            .and_then(|adb| run_command_text_until(adb, &["devices", "-l"], deadline))
            .map(|output| parse_online_android_emulators(&output, adb_path.as_deref(), deadline))
            .unwrap_or_default();
        let running_names = online_devices
            .iter()
            .filter_map(|device| device.avd_name.as_deref())
            .collect::<Vec<_>>();
        let emulator_avds_output = emulator_path
            .as_deref()
            .and_then(|emulator| run_command_text_until(emulator, &["-list-avds"], deadline));
        let available_avds = emulator_avds_output
            .as_deref()
            .map(|output| {
                output
                    .lines()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(|name| self.avd_info(name, running_names.contains(&name)))
                    .collect()
            })
            .unwrap_or_default();
        AndroidEnvironmentDiagnostics {
            sdk_status: if sdk_root.is_some() {
                "detected"
            } else {
                "missing"
            }
            .into(),
            adb_status: match (&adb_path, &adb_version) {
                (None, _) => "missing",
                (Some(_), Some(_)) => "ready",
                (Some(_), None) => "error",
            }
            .into(),
            emulator_status: match (&emulator_path, &emulator_avds_output) {
                (None, _) => "missing",
                (Some(_), Some(_)) => "ready",
                (Some(_), None) => "error",
            }
            .into(),
            sdk_root: path_string(sdk_root.as_deref()),
            adb_path: path_string(adb_path.as_deref()),
            emulator_path: path_string(emulator_path.as_deref()),
            sdkmanager_path: path_string(sdkmanager_path.as_deref()),
            avdmanager_path: path_string(avdmanager_path.as_deref()),
            adb_version,
            available_avds,
            online_devices,
        }
    }

    fn avd_info(&self, name: &str, running: bool) -> AndroidAvdInfo {
        let values = self
            .avd_home
            .as_ref()
            .map(|home| home.join(format!("{name}.avd/config.ini")))
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|contents| parse_properties(&contents))
            .unwrap_or_default();
        AndroidAvdInfo {
            name: name.into(),
            status: if running { "running" } else { "stopped" }.into(),
            device_profile: values.get("hw.device.name").cloned(),
            architecture: values
                .get("abi.type")
                .or_else(|| values.get("hw.cpu.arch"))
                .cloned(),
            running,
        }
    }
}

fn normalize_sdk_root(candidate: &Path) -> Option<PathBuf> {
    if candidate.file_name().is_some_and(|name| name == "adb") && candidate.is_file() {
        return candidate
            .parent()
            .filter(|parent| {
                parent
                    .file_name()
                    .is_some_and(|name| name == "platform-tools")
            })
            .and_then(Path::parent)
            .map(Path::to_path_buf);
    }
    candidate.is_dir().then(|| candidate.to_path_buf())
}

fn discover_tool(
    sdk_roots: &[PathBuf],
    path_dirs: &[PathBuf],
    relative: &str,
    binary: &str,
) -> Option<PathBuf> {
    sdk_roots
        .iter()
        .map(|root| root.join(relative))
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            path_dirs
                .iter()
                .map(|directory| directory.join(binary))
                .find(|candidate| candidate.is_file())
        })
}

fn discover_adb(candidates: &[PathBuf], path_dirs: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find_map(|candidate| {
            if candidate.file_name().is_some_and(|name| name == "adb") && candidate.is_file() {
                Some(candidate.clone())
            } else if candidate.is_dir() {
                let adb = candidate.join("platform-tools/adb");
                adb.is_file().then_some(adb)
            } else {
                None
            }
        })
        .or_else(|| {
            path_dirs
                .iter()
                .map(|directory| directory.join("adb"))
                .find(|candidate| candidate.is_file())
        })
}

fn discover_sdk_manager(
    sdk_roots: &[PathBuf],
    path_dirs: &[PathBuf],
    binary: &str,
) -> Option<PathBuf> {
    sdk_roots
        .iter()
        .flat_map(|root| {
            [
                root.join(format!("cmdline-tools/latest/bin/{binary}")),
                root.join(format!("tools/bin/{binary}")),
            ]
        })
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            path_dirs
                .iter()
                .map(|directory| directory.join(binary))
                .find(|candidate| candidate.is_file())
        })
}

fn infer_sdk_root(tool: &Path) -> Option<PathBuf> {
    let parent = tool.parent()?;
    match parent.file_name()?.to_str()? {
        "platform-tools" | "emulator" => parent.parent().map(Path::to_path_buf),
        _ => None,
    }
}

fn run_command_text_with_timeout(
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let child = command.spawn().ok()?;
    let output = wait_for_output_with_timeout(child, timeout).ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_command_text_until(program: &Path, args: &[&str], deadline: Instant) -> Option<String> {
    let remaining = deadline.checked_duration_since(Instant::now())?;
    run_command_text_with_timeout(program, args, remaining)
}

fn run_command_bytes_with_timeout(
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<Vec<u8>, MobileHostError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let child = command
        .spawn()
        .map_err(|error| MobileHostError::CommandFailed(error.to_string()))?;
    let output =
        wait_for_output_with_timeout(child, timeout).map_err(MobileHostError::CommandFailed)?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(MobileHostError::CommandFailed(
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(240)
            .collect(),
    ))
}

fn set_nonblocking(pipe: &impl AsRawFd) -> Result<(), String> {
    let fd = pipe.as_raw_fd();
    // SAFETY: `fd` belongs to the live pipe borrowed for this call. `fcntl` does
    // not take ownership, and both operations preserve all existing flags.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: as above; only O_NONBLOCK is added to the descriptor's flags.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn drain_available(pipe: &mut impl Read, output: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => output.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn kill_owned_process_group(child: &mut Child) {
    let process_group = -(child.id() as i32);
    // SAFETY: the child was spawned as leader of a new process group. A
    // negative PID targets only that owned group, never an existing emulator.
    let _ = unsafe { libc::kill(process_group, libc::SIGKILL) };
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_for_output_with_timeout(mut child: Child, timeout: Duration) -> Result<Output, String> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "child stdout was not piped".to_owned())?;
    let mut stderr = child.stderr.take();
    set_nonblocking(&stdout)?;
    if let Some(stderr) = &stderr {
        set_nonblocking(stderr)?;
    }
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let deadline = Instant::now() + timeout;
    loop {
        drain_available(&mut stdout, &mut stdout_bytes)?;
        if let Some(stderr) = &mut stderr {
            drain_available(stderr, &mut stderr_bytes)?;
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            drain_available(&mut stdout, &mut stdout_bytes)?;
            if let Some(stderr) = &mut stderr {
                drain_available(stderr, &mut stderr_bytes)?;
            }
            return Ok(Output {
                status,
                stdout: stdout_bytes,
                stderr: stderr_bytes,
            });
        }
        if Instant::now() >= deadline {
            kill_owned_process_group(&mut child);
            return Err(format!(
                "command timed out after {} ms",
                timeout.as_millis()
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn parse_online_android_emulators(
    output: &str,
    adb: Option<&Path>,
    deadline: Instant,
) -> Vec<AndroidDeviceInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let id = fields.next()?;
            let status = fields.next()?;
            if !id.starts_with("emulator-") || status != "device" {
                return None;
            }
            let model = fields
                .find_map(|field| field.strip_prefix("model:"))
                .map(str::to_owned);
            let avd_name = adb
                .and_then(|path| {
                    run_command_text_until(path, &["-s", id, "emu", "avd", "name"], deadline)
                })
                .and_then(|value| {
                    value
                        .lines()
                        .map(str::trim)
                        .find(|line| !line.is_empty() && *line != "OK")
                        .map(str::to_owned)
                });
            Some(AndroidDeviceInfo {
                id: id.into(),
                status: status.into(),
                model,
                avd_name,
            })
        })
        .collect()
}

fn parse_properties(contents: &str) -> HashMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().into(), value.trim().into()))
        })
        .collect()
}

fn path_string(path: Option<&Path>) -> Option<String> {
    path.map(|value| value.to_string_lossy().into_owned())
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MobileHostError {
    #[error("ADB was not found")]
    AdbMissing,
    #[error("no online Android Emulator was found")]
    NoOnlineEmulator,
    #[error("Android Emulator was not found")]
    EmulatorMissing,
    #[error("Android AVD was not found: {0}")]
    AvdNotFound(String),
    #[error("Android AVD did not finish booting before timeout: {0}")]
    AvdBootTimeout(String),
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
    detector: Option<AndroidEnvironmentDetector>,
    session_id: Mutex<String>,
    session_active: AtomicBool,
    frames: Mutex<ScreenFrameBuffer>,
    operation_lock: Mutex<()>,
    action_state: Mutex<MobileActionState>,
}

#[derive(Default)]
struct MobileActionState {
    latest_capture: Option<MobileCapture>,
    executed_count: u8,
}

pub struct MobileActionExecution {
    pub receipt: MobileActionReceipt,
    pub capture: Option<MobileCapture>,
}

impl MobileHost {
    pub fn discover() -> Self {
        Self {
            detector: None,
            session_id: Mutex::new(format!("mobile_session_{}", Uuid::now_v7())),
            session_active: AtomicBool::new(false),
            frames: Mutex::new(ScreenFrameBuffer::new(5)),
            operation_lock: Mutex::new(()),
            action_state: Mutex::new(MobileActionState::default()),
        }
    }

    #[cfg(test)]
    pub fn new(adb_path: Option<PathBuf>) -> Self {
        Self {
            detector: Some(AndroidEnvironmentDetector::new(
                adb_path.into_iter().collect(),
                Vec::new(),
                None,
            )),
            session_id: Mutex::new(format!("mobile_session_{}", Uuid::now_v7())),
            session_active: AtomicBool::new(false),
            frames: Mutex::new(ScreenFrameBuffer::new(5)),
            operation_lock: Mutex::new(()),
            action_state: Mutex::new(MobileActionState::default()),
        }
    }

    #[cfg(test)]
    fn with_detector(detector: AndroidEnvironmentDetector) -> Self {
        Self {
            detector: Some(detector),
            session_id: Mutex::new(format!("mobile_session_{}", Uuid::now_v7())),
            session_active: AtomicBool::new(false),
            frames: Mutex::new(ScreenFrameBuffer::new(5)),
            operation_lock: Mutex::new(()),
            action_state: Mutex::new(MobileActionState::default()),
        }
    }

    pub fn environment_diagnostics(
        &self,
        configured_sdk: Option<&str>,
    ) -> AndroidEnvironmentDiagnostics {
        if configured_sdk.is_some() {
            AndroidEnvironmentDetector::production(configured_sdk).detect()
        } else {
            self.detector
                .clone()
                .unwrap_or_else(|| AndroidEnvironmentDetector::production(None))
                .detect()
        }
    }

    pub fn start_avd(
        &self,
        configured_sdk: Option<&str>,
        name: &str,
        timeout: Duration,
    ) -> Result<AndroidDeviceInfo, MobileHostError> {
        let detector = self
            .detector
            .clone()
            .unwrap_or_else(|| AndroidEnvironmentDetector::production(configured_sdk));
        self.start_avd_with_detector(&detector, name, timeout)
    }

    #[cfg(test)]
    fn start_avd_with_timeout(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<AndroidDeviceInfo, MobileHostError> {
        let detector = self
            .detector
            .as_ref()
            .expect("test host must contain a detector");
        self.start_avd_with_detector(detector, name, timeout)
    }

    fn start_avd_with_detector(
        &self,
        detector: &AndroidEnvironmentDetector,
        name: &str,
        timeout: Duration,
    ) -> Result<AndroidDeviceInfo, MobileHostError> {
        let deadline = Instant::now() + timeout;
        let initial = detector.detect_until(deadline);
        if Instant::now() >= deadline {
            return Err(MobileHostError::AvdBootTimeout(name.into()));
        }
        let avd = initial
            .available_avds
            .iter()
            .find(|avd| avd.name == name)
            .ok_or_else(|| MobileHostError::AvdNotFound(name.into()))?;
        let adb = initial
            .adb_path
            .as_deref()
            .map(PathBuf::from)
            .ok_or(MobileHostError::AdbMissing)?;
        if !avd.running {
            let emulator = initial
                .emulator_path
                .as_deref()
                .ok_or(MobileHostError::EmulatorMissing)?;
            Command::new(emulator)
                .args(["-avd", name])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| MobileHostError::CommandFailed(error.to_string()))?;
        }
        loop {
            let diagnostics = detector.detect_until(deadline);
            if let Some(device) = diagnostics
                .online_devices
                .into_iter()
                .find(|device| device.avd_name.as_deref() == Some(name))
                && run_command_text_until(
                    &adb,
                    &["-s", &device.id, "shell", "getprop", "sys.boot_completed"],
                    deadline,
                )
                .is_some_and(|value| value.trim() == "1")
            {
                self.session_active.store(true, Ordering::Release);
                return Ok(device);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(MobileHostError::AvdBootTimeout(name.into()));
            }
            thread::sleep(Duration::from_millis(250).min(deadline - now));
        }
    }

    pub fn stop_session(&self) -> bool {
        let _guard = self
            .operation_lock
            .lock()
            .expect("mobile operation lock poisoned");
        if let Ok(mut state) = self.action_state.lock() {
            state.latest_capture = None;
            state.executed_count = 0;
        }
        let was_active = self.session_active.swap(false, Ordering::AcqRel);
        if was_active {
            *self
                .session_id
                .lock()
                .expect("mobile session id lock poisoned") =
                format!("mobile_session_{}", Uuid::now_v7());
        }
        was_active
    }

    pub fn session_id(&self) -> String {
        self.session_id
            .lock()
            .expect("mobile session id lock poisoned")
            .clone()
    }

    pub fn is_session_active(&self) -> bool {
        self.session_active.load(Ordering::SeqCst)
    }

    pub fn current_capture(&self) -> Option<MobileCapture> {
        self.action_state.lock().ok()?.latest_capture.clone()
    }

    pub fn observe_with_sdk(
        &self,
        android_sdk: Option<&str>,
        allowed_apps: &[String],
    ) -> Result<MobileCapture, MobileHostError> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| MobileHostError::CommandFailed("mobile operation lock poisoned".into()))?;
        self.observe_unlocked(android_sdk, allowed_apps)
    }

    fn observe_unlocked(
        &self,
        android_sdk: Option<&str>,
        allowed_apps: &[String],
    ) -> Result<MobileCapture, MobileHostError> {
        let diagnostics = self.environment_diagnostics(android_sdk);
        let adb = diagnostics
            .adb_path
            .as_deref()
            .map(Path::new)
            .filter(|_| diagnostics.adb_status == "ready")
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
        let focus_output = run_text(adb, &["-s", &device_id, "shell", "dumpsys", "window"])?;
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
            &self.session_id(),
        );
        self.frames
            .lock()
            .map_err(|_| MobileHostError::CommandFailed("frame buffer lock poisoned".into()))?
            .push(capture.frame.clone());
        self.action_state
            .lock()
            .map_err(|_| MobileHostError::CommandFailed("action state lock poisoned".into()))?
            .latest_capture = Some(capture.clone());
        self.session_active.store(true, Ordering::Release);
        Ok(capture)
    }

    pub fn execute_action(
        &self,
        request: MobileActionRequest,
        android_sdk: Option<&str>,
        allowed_apps: &[String],
    ) -> MobileActionExecution {
        let guard = self.operation_lock.lock();
        if guard.is_err() {
            return MobileActionExecution {
                receipt: action_receipt(
                    &request,
                    None,
                    MobileActionDecision::Denied(MobileActionDenyReason::InactiveSession),
                    MobileActionStatus::Failed,
                ),
                capture: None,
            };
        }
        let _guard = guard.expect("checked above");
        let state = self.action_state.lock();
        let (before, executed_count) = match state {
            Ok(state) => (state.latest_capture.clone(), state.executed_count),
            Err(_) => (None, 20),
        };
        let Some(before) = before else {
            return MobileActionExecution {
                receipt: action_receipt(
                    &request,
                    None,
                    MobileActionDecision::Denied(MobileActionDenyReason::StaleSnapshot),
                    MobileActionStatus::Blocked,
                ),
                capture: None,
            };
        };
        let mut receipt = action_receipt(
            &request,
            Some(&before),
            MobileActionDecision::Allowed,
            MobileActionStatus::Blocked,
        );
        if !self.is_session_active() {
            receipt.decision =
                MobileActionDecision::Denied(MobileActionDenyReason::InactiveSession);
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        }
        let initial = evaluate_action(
            &request,
            &before.session,
            &before.snapshot,
            allowed_apps,
            executed_count,
            &before.snapshot.package_name,
            &before.snapshot.activity,
            Utc::now(),
        );
        if initial != MobileActionDecision::Allowed {
            receipt.decision = initial;
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        }
        let diagnostics = self.environment_diagnostics(android_sdk);
        let Some(adb) = diagnostics
            .adb_path
            .as_deref()
            .map(Path::new)
            .filter(|_| diagnostics.adb_status == "ready")
        else {
            receipt.status = MobileActionStatus::Failed;
            receipt.verification = Some(VerificationResult::DeviceOffline);
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        };
        let fixture_override = cfg!(test) && self.detector.is_some();
        let supported_current = m2_navigation_package(&before.snapshot.package_name)
            || (matches!(request.target, MobileActionTarget::OpenApp { .. })
                && resolve_home_package(adb, &before.session.device_id)
                    .ok()
                    .as_deref()
                    == Some(before.snapshot.package_name.as_str()));
        let supported_target = match &request.target {
            MobileActionTarget::OpenApp { package_name } => m2_navigation_package(package_name),
            _ => true,
        };
        if !fixture_override
            && (!supported_target
                || (!matches!(
                    request.target,
                    MobileActionTarget::Back | MobileActionTarget::Home
                ) && !supported_current))
        {
            receipt.decision =
                MobileActionDecision::Denied(MobileActionDenyReason::PackageNotAllowed);
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        }
        receipt.target = request.target.clone();
        let live = (|| -> Result<(String, String), MobileHostError> {
            let devices = run_text(adb, &["devices"])?;
            if !parse_online_emulators(&devices).contains(&before.session.device_id) {
                return Err(MobileHostError::NoOnlineEmulator);
            }
            let focus = run_text(
                adb,
                &[
                    "-s",
                    &before.session.device_id,
                    "shell",
                    "dumpsys",
                    "window",
                ],
            )?;
            parse_focused_app(&focus).ok_or(MobileHostError::FocusUnavailable)
        })();
        let (live_package, live_activity) = match live {
            Ok(value) => value,
            Err(_) => {
                receipt.status = MobileActionStatus::Failed;
                receipt.verification = Some(VerificationResult::DeviceOffline);
                return MobileActionExecution {
                    receipt,
                    capture: None,
                };
            }
        };
        let decision = evaluate_action(
            &request,
            &before.session,
            &before.snapshot,
            allowed_apps,
            executed_count,
            &live_package,
            &live_activity,
            Utc::now(),
        );
        if decision != MobileActionDecision::Allowed {
            receipt.decision = decision;
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        }
        if !matches!(
            request.target,
            MobileActionTarget::Back | MobileActionTarget::Home
        ) {
            let live_tree = (|| -> Result<MobileUiSnapshot, MobileHostError> {
                run_text(
                    adb,
                    &[
                        "-s",
                        &before.session.device_id,
                        "shell",
                        "uiautomator",
                        "dump",
                        "/sdcard/ordinconn-ui.xml",
                    ],
                )?;
                let xml = run_text(
                    adb,
                    &[
                        "-s",
                        &before.session.device_id,
                        "shell",
                        "cat",
                        "/sdcard/ordinconn-ui.xml",
                    ],
                )?;
                Ok(MobileUiSnapshot::from_elements(
                    &before.session.session_id,
                    &live_package,
                    &live_activity,
                    before.snapshot.screen_width,
                    before.snapshot.screen_height,
                    parse_uiautomator_xml(&xml)?,
                ))
            })();
            let matches = live_tree
                .as_ref()
                .is_ok_and(|snapshot| pre_action_ui_matches(&before.snapshot, snapshot));
            let age_ms = Utc::now()
                .signed_duration_since(before.snapshot.captured_at)
                .num_milliseconds();
            let still_fresh = (0..=10_000).contains(&age_ms);
            if !matches || !still_fresh {
                receipt.decision =
                    MobileActionDecision::Denied(MobileActionDenyReason::StaleSnapshot);
                return MobileActionExecution {
                    receipt,
                    capture: None,
                };
            }
            let final_focus = run_text(
                adb,
                &[
                    "-s",
                    &before.session.device_id,
                    "shell",
                    "dumpsys",
                    "window",
                ],
            )
            .ok()
            .and_then(|output| parse_focused_app(&output));
            if final_focus.as_ref() != Some(&(live_package.clone(), live_activity.clone())) {
                receipt.decision =
                    MobileActionDecision::Denied(MobileActionDenyReason::ForegroundChanged);
                return MobileActionExecution {
                    receipt,
                    capture: None,
                };
            }
        }
        let home_package = if matches!(request.target, MobileActionTarget::Home) {
            resolve_home_package(adb, &before.session.device_id).ok()
        } else {
            None
        };
        if matches!(request.target, MobileActionTarget::Home) && home_package.is_none() {
            receipt.status = MobileActionStatus::Failed;
            receipt.verification = Some(VerificationResult::UnexpectedState);
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        }
        if let Ok(mut state) = self.action_state.lock() {
            state.executed_count = state.executed_count.saturating_add(1);
        }
        let action = execute_adb_action(adb, &before, &request);
        if action.is_err() {
            receipt.status = MobileActionStatus::Failed;
            receipt.verification = Some(VerificationResult::Interrupted);
            return MobileActionExecution {
                receipt,
                capture: None,
            };
        }
        receipt.command_sent = true;
        receipt.status = MobileActionStatus::Executed;
        thread::sleep(Duration::from_millis(300));
        match self.observe_unlocked(android_sdk, allowed_apps) {
            Ok(mut after) => {
                if after.session.device_id != before.session.device_id {
                    if let Ok(mut state) = self.action_state.lock() {
                        state.latest_capture = None;
                    }
                    receipt.status = MobileActionStatus::Failed;
                    receipt.verification = Some(VerificationResult::UnexpectedState);
                    receipt.completed_at = Utc::now();
                    return MobileActionExecution {
                        receipt,
                        capture: None,
                    };
                }
                let action_verification =
                    if after.observation.verification_status != VerificationResult::Verified {
                        after.observation.verification_status
                    } else {
                        verify_executed_action(
                            &request.target,
                            &before.observation,
                            &after.observation,
                            home_package.as_deref(),
                        )
                    };
                if matches!(request.target, MobileActionTarget::Type { .. })
                    && let Some(value) = request.text.as_ref()
                {
                    redact_typed_post_capture(&mut after, value.as_str());
                    if let Ok(mut state) = self.action_state.lock() {
                        state.latest_capture = Some(after.clone());
                    }
                }
                receipt.post_package = Some(after.observation.package_name.clone());
                receipt.post_snapshot_id = Some(after.snapshot.snapshot_id.clone());
                receipt.post_activity = Some(after.observation.activity.clone());
                receipt.post_frame_hash = Some(after.observation.frame_hash.clone());
                receipt.post_ui_tree_hash = Some(after.observation.ui_tree_hash.clone());
                receipt.verification = Some(action_verification);
                receipt.completed_at = Utc::now();
                MobileActionExecution {
                    receipt,
                    capture: Some(after),
                }
            }
            Err(_) => {
                receipt.status = MobileActionStatus::Failed;
                receipt.verification = Some(VerificationResult::Interrupted);
                receipt.completed_at = Utc::now();
                MobileActionExecution {
                    receipt,
                    capture: None,
                }
            }
        }
    }
}

fn m2_navigation_package(package: &str) -> bool {
    matches!(
        package,
        "com.android.settings" | "com.google.android.settings.intelligence"
    )
}

fn pre_action_ui_matches(before: &MobileUiSnapshot, live: &MobileUiSnapshot) -> bool {
    before.package_name == live.package_name
        && before.activity == live.activity
        && before.screen_width == live.screen_width
        && before.screen_height == live.screen_height
        && live.sensitive_state.is_none()
        && serde_json::to_vec(&before.elements).ok() == serde_json::to_vec(&live.elements).ok()
}

fn verify_executed_action(
    target: &MobileActionTarget,
    before: &MobileObservation,
    after: &MobileObservation,
    home_package: Option<&str>,
) -> VerificationResult {
    if before.device_session_id != after.device_session_id {
        return VerificationResult::UnexpectedState;
    }
    if matches!(target, MobileActionTarget::Home)
        && home_package.is_none_or(|package| after.package_name != package)
    {
        return VerificationResult::UnexpectedState;
    }
    if let MobileActionTarget::OpenApp { package_name } = target
        && after.package_name != *package_name
    {
        return VerificationResult::UnexpectedState;
    }
    let foreground_changed =
        before.package_name != after.package_name || before.activity != after.activity;
    let ui_changed = before.ui_tree_hash != after.ui_tree_hash;
    let verified = match target {
        MobileActionTarget::Tap { .. }
        | MobileActionTarget::Type { .. }
        | MobileActionTarget::Back => foreground_changed || ui_changed,
        MobileActionTarget::Swipe { .. } => ui_changed,
        MobileActionTarget::Home | MobileActionTarget::OpenApp { .. } => {
            foreground_changed || ui_changed
        }
    };
    if verified {
        VerificationResult::Verified
    } else {
        VerificationResult::NoChange
    }
}

fn resolve_home_package(adb: &Path, device_id: &str) -> Result<String, MobileHostError> {
    let output = run_text(
        adb,
        &[
            "-s",
            device_id,
            "shell",
            "cmd",
            "package",
            "resolve-activity",
            "--brief",
            "-a",
            "android.intent.action.MAIN",
            "-c",
            "android.intent.category.HOME",
        ],
    )?;
    output
        .lines()
        .map(str::trim)
        .find_map(|line| {
            let package = line.split_once('/')?.0;
            (!package.is_empty()
                && package.contains('.')
                && package
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_')))
            .then(|| package.to_owned())
        })
        .ok_or(MobileHostError::FocusUnavailable)
}

fn action_receipt(
    request: &MobileActionRequest,
    before: Option<&MobileCapture>,
    decision: MobileActionDecision,
    status: MobileActionStatus,
) -> MobileActionReceipt {
    MobileActionReceipt {
        action_id: request.action_id.clone(),
        session_id: before.map_or(String::new(), |capture| capture.session.session_id.clone()),
        snapshot_id: before.map_or(String::new(), |capture| {
            capture.snapshot.snapshot_id.clone()
        }),
        target: match &request.target {
            MobileActionTarget::Tap { element_ref }
                if before.is_some_and(|capture| {
                    capture
                        .snapshot
                        .elements
                        .iter()
                        .any(|element| element.element_ref == *element_ref)
                }) =>
            {
                MobileActionTarget::Tap {
                    element_ref: element_ref.clone(),
                }
            }
            MobileActionTarget::Type { element_ref }
                if before.is_some_and(|capture| {
                    capture
                        .snapshot
                        .elements
                        .iter()
                        .any(|element| element.element_ref == *element_ref)
                }) =>
            {
                MobileActionTarget::Type {
                    element_ref: element_ref.clone(),
                }
            }
            MobileActionTarget::Tap { .. } => MobileActionTarget::Tap {
                element_ref: "[REDACTED]".into(),
            },
            MobileActionTarget::Type { .. } => MobileActionTarget::Type {
                element_ref: "[REDACTED]".into(),
            },
            MobileActionTarget::OpenApp { .. } => MobileActionTarget::OpenApp {
                package_name: "[REDACTED]".into(),
            },
            other => other.clone(),
        },
        decision,
        status,
        requested_at: request.requested_at,
        completed_at: Utc::now(),
        pre_package: before.map_or(String::new(), |capture| {
            capture.observation.package_name.clone()
        }),
        pre_activity: before.map_or(String::new(), |capture| {
            capture.observation.activity.clone()
        }),
        pre_frame_hash: before.map_or(String::new(), |capture| {
            capture.observation.frame_hash.clone()
        }),
        pre_ui_tree_hash: before.map_or(String::new(), |capture| {
            capture.observation.ui_tree_hash.clone()
        }),
        post_package: None,
        post_snapshot_id: None,
        post_activity: None,
        post_frame_hash: None,
        post_ui_tree_hash: None,
        verification: None,
        text_length: request.text.as_ref().map(|text| text.len()),
        text_sha256: request.text.as_ref().map(|text| text.sha256()),
        command_sent: false,
    }
}

fn redact_typed_post_capture(capture: &mut MobileCapture, typed_value: &str) {
    let needle = typed_value.to_ascii_lowercase();
    for element in &mut capture.snapshot.elements {
        let editable = element.class_name.ends_with("EditText");
        let matches_text = element
            .text
            .as_ref()
            .is_some_and(|text| text.to_ascii_lowercase().contains(&needle));
        let matches_description = element
            .content_description
            .as_ref()
            .is_some_and(|description| description.to_ascii_lowercase().contains(&needle));
        let matches_id = element
            .resource_id
            .as_ref()
            .is_some_and(|id| id.to_ascii_lowercase().contains(&needle));
        if editable || matches_text || matches_description || matches_id {
            element.text = Some("[REDACTED]".into());
            element.content_description = None;
            if matches_id {
                element.resource_id = None;
            }
            capture
                .snapshot
                .redactions
                .push(format!("element:{}:typed", element.element_ref));
        }
    }
    capture.observation = MobileObservation::from_snapshot(
        &capture.snapshot,
        &capture.frame,
        capture.observation.task_id.clone(),
        capture.observation.app_id.clone(),
        capture.observation.privacy_class,
    );
}

fn execute_adb_action(
    adb: &Path,
    before: &MobileCapture,
    request: &MobileActionRequest,
) -> Result<(), MobileHostError> {
    let device_id = &before.session.device_id;
    let coordinates = |element_ref: &str| {
        before
            .snapshot
            .elements
            .iter()
            .find(|element| element.element_ref == element_ref)
            .map(|element| {
                (
                    element.bounds.x + element.bounds.width / 2,
                    element.bounds.y + element.bounds.height / 2,
                )
            })
            .ok_or(MobileHostError::InvalidUiTree(
                "action target missing".into(),
            ))
    };
    match &request.target {
        MobileActionTarget::Tap { element_ref } => {
            let (x, y) = coordinates(element_ref)?;
            run_text(
                adb,
                &[
                    "-s",
                    device_id,
                    "shell",
                    "input",
                    "tap",
                    &x.to_string(),
                    &y.to_string(),
                ],
            )?;
        }
        MobileActionTarget::Swipe { direction } => {
            let width = before.snapshot.screen_width;
            let height = before.snapshot.screen_height;
            let (x1, y1, x2, y2) = match direction {
                SwipeDirection::Up => (width / 2, height * 7 / 10, width / 2, height * 3 / 10),
                SwipeDirection::Down => (width / 2, height * 3 / 10, width / 2, height * 7 / 10),
                SwipeDirection::Left => (width * 7 / 10, height / 2, width * 3 / 10, height / 2),
                SwipeDirection::Right => (width * 3 / 10, height / 2, width * 7 / 10, height / 2),
            };
            run_text(
                adb,
                &[
                    "-s",
                    device_id,
                    "shell",
                    "input",
                    "swipe",
                    &x1.to_string(),
                    &y1.to_string(),
                    &x2.to_string(),
                    &y2.to_string(),
                    "350",
                ],
            )?;
        }
        MobileActionTarget::Type { .. } => {
            let value = request
                .text
                .as_ref()
                .ok_or(MobileHostError::InvalidUiTree("type text missing".into()))?;
            let encoded = value.as_str().replace(' ', "%s");
            run_text(adb, &["-s", device_id, "shell", "input", "text", &encoded])?;
        }
        MobileActionTarget::Back => {
            run_text(adb, &["-s", device_id, "shell", "input", "keyevent", "4"])?;
        }
        MobileActionTarget::Home => {
            run_text(adb, &["-s", device_id, "shell", "input", "keyevent", "3"])?;
        }
        MobileActionTarget::OpenApp { package_name } => {
            let resolution = run_text(
                adb,
                &[
                    "-s",
                    device_id,
                    "shell",
                    "cmd",
                    "package",
                    "resolve-activity",
                    "--brief",
                    "-a",
                    "android.intent.action.MAIN",
                    "-c",
                    "android.intent.category.LAUNCHER",
                    "-p",
                    package_name,
                ],
            )?;
            let component = resolution
                .lines()
                .map(str::trim)
                .find(|line| line.starts_with(&format!("{package_name}/")))
                .ok_or(MobileHostError::InvalidUiTree(
                    "launcher activity not resolved".into(),
                ))?;
            if !component.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'$' | b'/')
            }) {
                return Err(MobileHostError::InvalidUiTree(
                    "invalid launcher component".into(),
                ));
            }
            run_text(
                adb,
                &[
                    "-s",
                    device_id,
                    "shell",
                    "am",
                    "start",
                    "--activity-clear-top",
                    "-n",
                    component,
                ],
            )?;
        }
    }
    Ok(())
}

fn bind_capture_to_session(mut capture: MobileCapture, session_id: &str) -> MobileCapture {
    capture.session.session_id = session_id.into();
    capture.snapshot.session_id = session_id.into();
    capture.frame.session_id = session_id.into();
    capture.observation.device_session_id = session_id.into();
    capture
}

fn run_text(adb: &Path, args: &[&str]) -> Result<String, MobileHostError> {
    run_command_bytes_with_timeout(adb, args, Duration::from_secs(10))
        .map(|output| String::from_utf8_lossy(&output).into_owned())
}

fn run_bytes(adb: &Path, args: &[&str]) -> Result<Vec<u8>, MobileHostError> {
    run_command_bytes_with_timeout(adb, args, Duration::from_secs(10))
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
                let class_name = values.remove("class").unwrap_or_default();
                let mut element = RawMobileElement {
                    text: optional_value(values.remove("text")),
                    role: class_name
                        .rsplit('.')
                        .next()
                        .unwrap_or("node")
                        .to_ascii_lowercase(),
                    class_name,
                    content_description: optional_value(values.remove("content-desc")),
                    bounds: MobileBounds {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    },
                    clickable: bool_value(values.get("clickable")),
                    scrollable: bool_value(values.get("scrollable")),
                    enabled: bool_value(values.get("enabled")),
                    focused: bool_value(values.get("focused")),
                    selected: bool_value(values.get("selected")),
                    resource_id: optional_value(values.remove("resource-id")),
                    password: bool_value(values.get("password")),
                };
                let Some(bounds) =
                    parse_bounds(values.get("bounds").map(String::as_str).unwrap_or_default())
                else {
                    if element.requires_redaction() {
                        return Err(MobileHostError::InvalidUiTree(
                            "sensitive node has invalid bounds".into(),
                        ));
                    }
                    continue;
                };
                element.bounds = bounds;
                elements.push(element);
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
    use std::{fs, os::unix::fs::PermissionsExt};
    use tempfile::TempDir;

    const DEVICES: &str = "List of devices attached\nemulator-5554\tdevice\nemulator-5556\toffline\nR58M123\tdevice\n";
    const FOCUS: &str =
        "mCurrentFocus=Window{123 u0 com.example.news/com.example.news.MainActivity}";
    const XML: &str = r#"<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><hierarchy rotation="0"><node index="0" text="BTC ETF inflows" resource-id="com.example.news:id/title" class="android.widget.TextView" package="com.example.news" content-desc="headline" clickable="false" enabled="true" bounds="[10,20][900,100]"/><node index="1" text="secret" password="true" resource-id="com.example.news:id/password" class="android.widget.EditText" package="com.example.news" clickable="true" enabled="true" bounds="[10,120][900,200]"/></hierarchy>"#;

    fn mobile_action_fixture() -> (TempDir, PathBuf, PathBuf, PathBuf, MobileHost) {
        let fixture = TempDir::new().unwrap();
        let sdk = fixture.path().join("sdk");
        let focus = fixture.path().join("focus.txt");
        let input_log = fixture.path().join("input.log");
        let fail_post = fixture.path().join("fail-post");
        let other_first = fixture.path().join("other-first");
        let typed_visible = fixture.path().join("typed-visible");
        fs::write(&focus, FOCUS).unwrap();
        let safe_xml = r#"<hierarchy><node text="Network" class="android.widget.Button" resource-id="com.example.news:id/network" clickable="true" enabled="true" bounds="[10,20][900,100]"/><node text="Search" class="android.widget.EditText" resource-id="com.example.news:id/search" clickable="true" enabled="true" focused="true" bounds="[10,120][900,200]"/></hierarchy>"#;
        let typed_xml = safe_xml.replace("text=\"Search\"", "text=\"wifi\"");
        executable(
            &sdk.join("platform-tools/adb"),
            &format!(
                r##"#!/bin/sh
if [ "$1" = "version" ]; then echo adb-fixture; exit 0; fi
if [ "$1" = "devices" ]; then
  if [ -f '{}' ]; then printf 'List of devices attached\nemulator-5556 device\nemulator-5554 device\n';
  else printf 'List of devices attached\nemulator-5554 device\n'; fi
  exit 0
fi
case "$*" in
  *ro.build.version.release*) echo 16 ;;
  *"wm size"*) echo 'Physical size: 1080x2400' ;;
  *"dumpsys window"*) /bin/cat '{}' ;;
  *"uiautomator dump"*) if [ -f '{}' ] && [ -f '{}' ]; then exit 1; fi; echo dumped ;;
  *"cat /sdcard/ordinconn-ui.xml"*) if [ -f '{}' ]; then printf '%s' '{}'; else printf '%s' '{}'; fi ;;
  *"screencap -p"*) printf PNG ;;
  *"android.intent.category.HOME"*) echo 'com.example.launcher/.HomeActivity' ;;
  *"cmd package resolve-activity"*) echo 'com.example.news/.MainActivity' ;;
  *"shell am start"*) printf '%s\n' "$*" >> '{}'; exit 0 ;;
  *"shell input text"*) printf 'shell input text [REDACTED]\n' >> '{}'; /usr/bin/touch '{}'; exit 0 ;;
  *"shell input"*) printf '%s\n' "$*" >> '{}'; exit 0 ;;
  *) exit 1 ;;
esac
"##,
                other_first.display(),
                focus.display(),
                fail_post.display(),
                input_log.display(),
                typed_visible.display(),
                typed_xml,
                safe_xml,
                input_log.display(),
                input_log.display(),
                typed_visible.display(),
                input_log.display()
            ),
        );
        let host = MobileHost::new(None);
        (fixture, sdk, focus, input_log, host)
    }

    fn mobile_action_request(capture: &MobileCapture) -> mobile_runtime::MobileActionRequest {
        mobile_runtime::MobileActionRequest {
            action_id: format!("action_{}", Uuid::now_v7()),
            session_id: capture.session.session_id.clone(),
            snapshot_id: capture.snapshot.snapshot_id.clone(),
            expected_package: capture.snapshot.package_name.clone(),
            requested_at: Utc::now(),
            target: mobile_runtime::MobileActionTarget::Tap {
                element_ref: "@e1".into(),
            },
            text: None,
        }
    }

    #[test]
    fn rejected_action_receipt_does_not_persist_untrusted_identifiers() {
        let (_fixture, sdk, _focus, _input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let mut request = mobile_action_request(&capture);
        request.session_id = "private-session-value".into();
        request.snapshot_id = "private-snapshot-value".into();
        request.target = MobileActionTarget::Tap {
            element_ref: "private-element-value".into(),
        };
        let receipt = host.execute_action(request, sdk.to_str(), &allowed).receipt;
        let serialized = serde_json::to_string(&receipt).unwrap();
        assert!(!serialized.contains("private-session-value"));
        assert!(!serialized.contains("private-snapshot-value"));
        assert!(!serialized.contains("private-element-value"));
        assert_eq!(receipt.session_id, capture.session.session_id);
        assert_eq!(receipt.snapshot_id, capture.snapshot.snapshot_id);
    }

    #[test]
    fn mobile_action_stale_ref_and_changed_foreground_do_not_emit_input() {
        let (_fixture, sdk, focus, input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let mut request = mobile_action_request(&capture);
        request.snapshot_id = "stale".into();
        let stale = host.execute_action(request, sdk.to_str(), &allowed);
        assert_eq!(
            stale.receipt.status,
            mobile_runtime::MobileActionStatus::Blocked
        );
        assert!(!input_log.exists());
        fs::write(
            &focus,
            "mCurrentFocus=Window{123 u0 com.example.other/.Main}",
        )
        .unwrap();
        let changed = host.execute_action(mobile_action_request(&capture), sdk.to_str(), &allowed);
        assert_eq!(
            changed.receipt.status,
            mobile_runtime::MobileActionStatus::Blocked
        );
        assert!(!input_log.exists());
    }

    #[test]
    fn mobile_action_tap_is_bounded_and_no_change_is_not_verified() {
        let (_fixture, sdk, _focus, input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let result = host.execute_action(mobile_action_request(&capture), sdk.to_str(), &allowed);
        assert_eq!(
            result.receipt.status,
            mobile_runtime::MobileActionStatus::Executed
        );
        assert_eq!(
            result.receipt.verification,
            Some(mobile_runtime::VerificationResult::NoChange)
        );
        assert!(result.capture.is_some());
        assert!(
            fs::read_to_string(input_log)
                .unwrap()
                .contains("shell input tap 455 60")
        );
    }

    #[test]
    fn mobile_action_failed_post_capture_never_verifies() {
        let (fixture, sdk, _focus, input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        fs::write(fixture.path().join("fail-post"), "1").unwrap();
        let result = host.execute_action(mobile_action_request(&capture), sdk.to_str(), &allowed);
        assert_eq!(
            result.receipt.status,
            mobile_runtime::MobileActionStatus::Failed
        );
        assert_ne!(
            result.receipt.verification,
            Some(mobile_runtime::VerificationResult::Verified)
        );
        assert!(result.capture.is_none());
        assert!(input_log.exists());
    }

    #[test]
    fn mobile_action_post_capture_on_different_emulator_is_unexpected() {
        let (fixture, sdk, _focus, _input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        fs::write(fixture.path().join("other-first"), "1").unwrap();
        let result = host.execute_action(mobile_action_request(&capture), sdk.to_str(), &allowed);
        assert_eq!(
            result.receipt.verification,
            Some(VerificationResult::UnexpectedState)
        );
    }

    #[test]
    fn mobile_action_all_six_structured_commands_are_fixed_and_bounded() {
        let (_fixture, sdk, _focus, input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let mut capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let targets = [
            MobileActionTarget::Tap {
                element_ref: "@e1".into(),
            },
            MobileActionTarget::Swipe {
                direction: SwipeDirection::Up,
            },
            MobileActionTarget::Type {
                element_ref: "@e2".into(),
            },
            MobileActionTarget::Back,
            MobileActionTarget::Home,
            MobileActionTarget::OpenApp {
                package_name: "com.example.news".into(),
            },
        ];
        for target in targets {
            let mut request = mobile_action_request(&capture);
            if matches!(target, MobileActionTarget::Type { .. }) {
                request.text = Some(mobile_runtime::SensitiveText::new("wifi".into()));
            }
            request.target = target;
            let result = host.execute_action(request, sdk.to_str(), &allowed);
            assert_eq!(result.receipt.status, MobileActionStatus::Executed);
            assert!(
                !serde_json::to_string(&result.receipt)
                    .unwrap()
                    .contains("wifi")
            );
            capture = result.capture.expect("fresh production capture");
        }
        let commands = fs::read_to_string(input_log).unwrap();
        assert_eq!(commands.lines().count(), 6);
        assert!(commands.contains("shell input tap 455 60"));
        assert!(commands.contains("shell input swipe 540 1680 540 720 350"));
        assert!(commands.contains("shell input text [REDACTED]"));
        assert!(!commands.contains("wifi"));
        assert!(commands.contains("shell input keyevent 4"));
        assert!(commands.contains("shell input keyevent 3"));
        assert!(
            commands
                .contains("shell am start --activity-clear-top -n com.example.news/.MainActivity")
        );
    }

    #[test]
    fn open_app_verification_requires_the_target_package_in_foreground() {
        let (_fixture, sdk, _focus, _input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let before = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let mut after = before.observation.clone();
        after.frame_hash = "different-frame".into();
        let request = MobileActionTarget::OpenApp {
            package_name: "com.android.settings".into(),
        };
        assert_eq!(
            verify_executed_action(&request, &before.observation, &after, None),
            VerificationResult::UnexpectedState
        );
        let home = MobileActionTarget::Home;
        assert_eq!(
            verify_executed_action(
                &home,
                &before.observation,
                &after,
                Some("com.example.launcher")
            ),
            VerificationResult::UnexpectedState
        );
        let tap = MobileActionTarget::Tap {
            element_ref: "@e1".into(),
        };
        assert_eq!(
            verify_executed_action(&tap, &before.observation, &after, None),
            VerificationResult::NoChange,
            "frame-only changes must not verify a tap"
        );
        let swipe = MobileActionTarget::Swipe {
            direction: SwipeDirection::Up,
        };
        assert_eq!(
            verify_executed_action(&swipe, &before.observation, &after, None),
            VerificationResult::NoChange,
            "frame-only changes must not verify a swipe"
        );
    }

    #[test]
    fn mobile_action_twenty_action_budget_blocks_before_adb_input() {
        let (_fixture, sdk, _focus, input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        host.action_state.lock().unwrap().executed_count = 20;
        let result = host.execute_action(mobile_action_request(&capture), sdk.to_str(), &allowed);
        assert_eq!(result.receipt.status, MobileActionStatus::Blocked);
        assert_eq!(
            result.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::BudgetExceeded)
        );
        assert!(!input_log.exists());
    }

    #[test]
    fn user_allowlist_cannot_enable_arbitrary_app_actions_in_production_host() {
        assert!(m2_navigation_package("com.android.settings"));
        assert!(m2_navigation_package(
            "com.google.android.settings.intelligence"
        ));
        assert!(!m2_navigation_package("com.example.wallet"));
        let (_fixture, sdk, _focus, input_log, mut host) = mobile_action_fixture();
        host.detector = None;
        let allowed = vec!["com.example.news".to_owned()];
        let capture = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let result = host.execute_action(mobile_action_request(&capture), sdk.to_str(), &allowed);
        assert_eq!(
            result.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::PackageNotAllowed)
        );
        assert!(!result.receipt.command_sent);
        assert!(!input_log.exists());
    }

    #[test]
    fn stopping_session_rotates_identity_and_resets_action_budget() {
        let (_fixture, sdk, _focus, _input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let first = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        host.action_state.lock().unwrap().executed_count = 20;
        assert!(host.stop_session());
        assert_eq!(host.action_state.lock().unwrap().executed_count, 0);
        let second = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        assert_ne!(first.session.session_id, second.session.session_id);
        let old_request = mobile_action_request(&first);
        let result = host.execute_action(old_request, sdk.to_str(), &allowed);
        assert_eq!(
            result.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::InactiveSession)
        );
        assert!(!result.receipt.command_sent);
    }

    #[test]
    fn mobile_action_type_plaintext_is_absent_from_post_capture() {
        let (_fixture, sdk, _focus, input_log, host) = mobile_action_fixture();
        let allowed = vec!["com.example.news".to_owned()];
        let before = host.observe_with_sdk(sdk.to_str(), &allowed).unwrap();
        let mut request = mobile_action_request(&before);
        request.target = MobileActionTarget::Type {
            element_ref: "@e2".into(),
        };
        request.text = Some(mobile_runtime::SensitiveText::new("wifi".into()));
        let result = host.execute_action(request, sdk.to_str(), &allowed);
        assert_eq!(result.receipt.status, MobileActionStatus::Executed);
        let post = result.capture.expect("fresh post-action capture");
        assert!(
            !serde_json::to_string(&post.snapshot)
                .unwrap()
                .contains("wifi")
        );
        assert!(
            !serde_json::to_string(&post.observation)
                .unwrap()
                .contains("wifi")
        );
        assert!(!fs::read_to_string(input_log).unwrap().contains("wifi"));
    }

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
    fn uiautomator_parser_skips_platform_nodes_with_reversed_bounds() {
        let xml = r#"<hierarchy><node text="offscreen" class="android.view.View" bounds="[0,2364][1080,2337]"/><node text="visible" class="android.widget.TextView" bounds="[10,20][100,80]"/></hierarchy>"#;

        let elements = parse_uiautomator_xml(xml).unwrap();

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].text.as_deref(), Some("visible"));
        assert_eq!(elements[0].bounds.width, 90);
        assert_eq!(elements[0].bounds.height, 60);
    }

    #[test]
    fn uiautomator_parser_fails_closed_for_sensitive_nodes_with_reversed_bounds() {
        let xml = r#"<hierarchy><node text="secret" password="true" class="android.widget.EditText" bounds="[0,2364][1080,2337]"/><node text="public" class="android.widget.TextView" bounds="[10,20][100,80]"/></hierarchy>"#;

        let error = parse_uiautomator_xml(xml).unwrap_err();

        assert_eq!(
            error,
            MobileHostError::InvalidUiTree("sensitive node has invalid bounds".into())
        );
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
    fn real_m2_navigation_smoke_is_explicitly_gated() {
        if std::env::var("ORDINCONN_MOBILE_M2_SMOKE").as_deref() != Ok("1") {
            return;
        }
        let host = MobileHost::discover();
        let diagnostics = host.environment_diagnostics(None);
        assert_eq!(diagnostics.online_devices.len(), 1);
        let device = &diagnostics.online_devices[0];
        assert!(device.id.starts_with("emulator-"));
        let adb = Path::new(diagnostics.adb_path.as_deref().unwrap());
        let avd = run_text(adb, &["-s", &device.id, "emu", "avd", "name"]).unwrap();
        assert!(avd.lines().any(|line| line.trim() == "OrdinConn_M1_5"));
        let launcher_package = resolve_home_package(adb, &device.id).unwrap();
        let allowed = vec![
            "com.android.settings".to_owned(),
            "com.google.android.settings.intelligence".to_owned(),
            launcher_package.clone(),
        ];
        let mut capture = host.observe_with_sdk(None, &allowed).unwrap();
        assert_eq!(capture.snapshot.package_name, "com.android.settings");
        let directory = TempDir::new().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let app = runtime
            .block_on(ordinconn_app::AppRuntime::initialize(
                &directory.path().join("m2-real-smoke.sqlite3"),
            ))
            .unwrap();
        runtime
            .block_on(crate::commands::record_mobile_capture_workspace(
                app.as_ref(),
                diagnostics,
                capture.clone(),
            ))
            .unwrap();
        let mut audit_events = app.subscribe();
        let perform = |before: &MobileCapture,
                       target: MobileActionTarget,
                       text: Option<mobile_runtime::SensitiveText>| {
            let mut request = mobile_action_request(before);
            request.target = target;
            request.text = text;
            runtime
                .block_on(app.record_mobile_action_intent(&request))
                .unwrap();
            let result = host.execute_action(request, None, &allowed);
            runtime
                .block_on(app.record_mobile_action(&result.receipt, result.capture.as_ref()))
                .unwrap();
            result
        };
        let network = capture
            .snapshot
            .elements
            .iter()
            .find(|element| element.text.as_deref() == Some("Network & internet"))
            .unwrap();
        let x = network.bounds.x + network.bounds.width / 2;
        let y = network.bounds.y + network.bounds.height / 2;
        let target = capture
            .snapshot
            .elements
            .iter()
            .filter(|element| {
                element.clickable
                    && element.enabled
                    && element.bounds.x <= x
                    && x < element.bounds.x + element.bounds.width
                    && element.bounds.y <= y
                    && y < element.bounds.y + element.bounds.height
            })
            .min_by_key(|element| element.bounds.width * element.bounds.height)
            .unwrap()
            .element_ref
            .clone();
        let original = capture.clone();
        let tap = perform(
            &capture,
            MobileActionTarget::Tap {
                element_ref: target.clone(),
            },
            None,
        );
        assert_eq!(tap.receipt.verification, Some(VerificationResult::Verified));
        println!("M2_REAL_TAP=PASS");
        capture = tap.capture.unwrap();
        assert_ne!(capture.snapshot.activity, original.snapshot.activity);
        let stale = perform(
            &original,
            MobileActionTarget::Tap {
                element_ref: target,
            },
            None,
        );
        assert_eq!(
            stale.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::StaleSnapshot)
        );
        assert!(!stale.receipt.command_sent);
        println!("M2_STALE_REF_BLOCK=PASS");
        let swipe = perform(
            &capture,
            MobileActionTarget::Swipe {
                direction: SwipeDirection::Up,
            },
            None,
        );
        assert_eq!(
            swipe.receipt.verification,
            Some(VerificationResult::Verified)
        );
        capture = swipe.capture.unwrap();
        println!("M2_REAL_SWIPE=PASS");
        let back = perform(&capture, MobileActionTarget::Back, None);
        assert_eq!(
            back.receipt.verification,
            Some(VerificationResult::Verified)
        );
        capture = back.capture.unwrap();
        assert_eq!(capture.snapshot.activity, original.snapshot.activity);
        println!("M2_REAL_BACK=PASS");
        let search = capture
            .snapshot
            .elements
            .iter()
            .find(|element| {
                element.clickable
                    && element
                        .resource_id
                        .as_deref()
                        .is_some_and(|id| id.ends_with(":id/search_action_bar"))
            })
            .unwrap()
            .element_ref
            .clone();
        capture = perform(
            &capture,
            MobileActionTarget::Tap {
                element_ref: search,
            },
            None,
        )
        .capture
        .unwrap();
        let editable = capture
            .snapshot
            .elements
            .iter()
            .find(|element| element.class_name.ends_with("EditText") && element.focused)
            .unwrap()
            .element_ref
            .clone();
        let typed = perform(
            &capture,
            MobileActionTarget::Type {
                element_ref: editable,
            },
            Some(mobile_runtime::SensitiveText::new("wifi".into())),
        );
        assert_eq!(
            typed.receipt.verification,
            Some(VerificationResult::Verified)
        );
        assert!(
            !serde_json::to_string(&typed.receipt)
                .unwrap()
                .contains("wifi")
        );
        assert!(
            !serde_json::to_string(&typed.capture)
                .unwrap()
                .contains("wifi")
        );
        capture = typed.capture.unwrap();
        println!("M2_REAL_TYPE_AND_PRIVACY=PASS");
        let workspace = runtime.block_on(app.mobile_workspace_data()).unwrap();
        assert_eq!(
            workspace
                .latest_action_receipt
                .as_ref()
                .map(|receipt| &receipt.action_id),
            Some(&typed.receipt.action_id)
        );
        println!("M2_RECEIPT_PERSISTENCE=PASS");
        let home = perform(&capture, MobileActionTarget::Home, None);
        capture = home.capture.unwrap();
        assert_eq!(
            home.receipt.verification,
            Some(VerificationResult::Verified)
        );
        assert_eq!(capture.snapshot.package_name, launcher_package);
        println!("M2_REAL_HOME=PASS");
        let open = perform(
            &capture,
            MobileActionTarget::OpenApp {
                package_name: "com.android.settings".into(),
            },
            None,
        );
        capture = open.capture.unwrap();
        assert_eq!(capture.snapshot.package_name, "com.android.settings");
        println!("M2_REAL_OPEN_APP=PASS");
        let mut wrong = mobile_action_request(&capture);
        wrong.expected_package = launcher_package;
        let wrong = host.execute_action(wrong, None, &allowed);
        assert_eq!(
            wrong.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::ForegroundChanged)
        );
        assert!(!wrong.receipt.command_sent);
        println!("M2_WRONG_PACKAGE_BLOCK=PASS");
        let disallowed = perform(
            &capture,
            MobileActionTarget::OpenApp {
                package_name: "com.example.disallowed".into(),
            },
            None,
        );
        assert_eq!(
            disallowed.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::PackageNotAllowed)
        );
        assert!(!disallowed.receipt.command_sent);
        println!("M2_UNALLOWLISTED_APP_BLOCK=PASS");
        let event_types = std::iter::from_fn(|| audit_events.try_recv().ok())
            .map(|event| event.event_type)
            .collect::<Vec<_>>();
        for required in [
            "mobile.action_intent",
            "mobile.action_requested",
            "mobile.action_blocked",
            "mobile.action_executed",
            "mobile.action_verified",
        ] {
            assert!(event_types.iter().any(|kind| kind == required));
        }
        println!("M2_AUDIT_EVENTS=PASS");
        assert!(host.stop_session());
    }

    #[test]
    fn action_preflight_rejects_changed_ui_in_same_activity() {
        let before = MobileUiSnapshot::from_elements(
            "session-1",
            "com.example",
            ".Main",
            1080,
            2400,
            vec![RawMobileElement {
                text: Some("Open details".into()),
                role: "button".into(),
                class_name: "android.widget.Button".into(),
                content_description: None,
                bounds: MobileBounds {
                    x: 100,
                    y: 200,
                    width: 400,
                    height: 80,
                },
                clickable: true,
                scrollable: false,
                enabled: true,
                focused: false,
                selected: false,
                resource_id: None,
                password: false,
            }],
        );
        let same = MobileUiSnapshot::from_elements(
            "session-1",
            "com.example",
            ".Main",
            1080,
            2400,
            vec![RawMobileElement {
                text: Some("Open details".into()),
                role: "button".into(),
                class_name: "android.widget.Button".into(),
                content_description: None,
                bounds: MobileBounds {
                    x: 100,
                    y: 200,
                    width: 400,
                    height: 80,
                },
                clickable: true,
                scrollable: false,
                enabled: true,
                focused: false,
                selected: false,
                resource_id: None,
                password: false,
            }],
        );
        assert!(pre_action_ui_matches(&before, &same));
        let mut moved = same.clone();
        moved.elements[0].bounds.y = 320;
        assert!(!pre_action_ui_matches(&before, &moved));
        moved = same;
        moved.elements[0].text = Some("Pay now".into());
        assert!(!pre_action_ui_matches(&before, &moved));
    }

    #[test]
    fn real_emulator_smoke_is_explicitly_gated() {
        if std::env::var("ORDINCONN_MOBILE_SMOKE").as_deref() != Ok("1") {
            return;
        }
        let host = MobileHost::discover();
        let mut diagnostics = host.environment_diagnostics(None);
        if diagnostics.online_devices.is_empty()
            && let Some(avd) = diagnostics.available_avds.first()
        {
            if let Err(error) = host.start_avd(None, &avd.name, Duration::from_secs(120)) {
                let report = m1_5_environment_gate(&host.environment_diagnostics(None));
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
                panic!("M1.5 AVD startup failed: {error}; M2 is forbidden");
            }
            diagnostics = host.environment_diagnostics(None);
        }
        let environment_report = m1_5_environment_gate(&diagnostics);
        if environment_report.checks[..3]
            .iter()
            .any(|check| check.status != MobileGateStatus::Pass)
        {
            println!(
                "{}",
                serde_json::to_string_pretty(&environment_report).unwrap()
            );
            panic!("M1.5 environment gate failed; M2 is forbidden");
        }
        let allowed = std::env::var("ORDINCONN_MOBILE_ALLOWED_APPS")
            .unwrap_or_default()
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let capture = match host.observe_with_sdk(None, &allowed) {
            Ok(capture) => capture,
            Err(error) => {
                let report = m1_5_capture_failure_gate(&diagnostics, error.to_string());
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
                panic!("M1.5 capture failed; M2 is forbidden");
            }
        };
        let directory = TempDir::new().unwrap();
        let async_runtime = tokio::runtime::Runtime::new().unwrap();
        let app_runtime = async_runtime
            .block_on(ordinconn_app::AppRuntime::initialize(
                &directory.path().join("real-mobile-smoke.sqlite3"),
            ))
            .unwrap();
        let mut events = app_runtime.subscribe();
        let workspace = async_runtime
            .block_on(crate::commands::record_mobile_capture_workspace(
                app_runtime.as_ref(),
                diagnostics.clone(),
                capture.clone(),
            ))
            .expect("real capture must persist and project through the production workspace path");
        let event_types = (0..3)
            .map(|_| events.try_recv().map(|event| event.event_type))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default();
        let ipc_serializable = serde_json::to_value(&workspace).is_ok()
            && workspace
                .observations
                .iter()
                .any(|item| item.id == capture.observation.id)
            && event_types
                == vec![
                    "mobile.session_started",
                    "mobile.snapshot",
                    "mobile.observation",
                ];
        let session_shutdown = host.stop_session()
            && async_runtime
                .block_on(app_runtime.end_mobile_session(&capture.session.session_id))
                .unwrap_or(false);
        let report = m1_5_capture_gate(&diagnostics, &capture, ipc_serializable, session_shutdown);
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        assert!(report.passed, "M1.5 gate failed; M2 is forbidden");
    }

    #[test]
    fn real_m2_sensitive_type_block_is_explicitly_gated() {
        if std::env::var("ORDINCONN_MOBILE_M2_SENSITIVE_SMOKE").as_deref() != Ok("1") {
            return;
        }
        let host = MobileHost::discover();
        let allowed = vec!["com.ordinconn.m2test".to_owned()];
        let capture = host.observe_with_sdk(None, &allowed).unwrap();
        assert_eq!(
            capture.snapshot.sensitive_state,
            Some(VerificationResult::SensitiveFieldBlocked)
        );
        let password_ref = capture
            .snapshot
            .elements
            .iter()
            .find(|element| element.class_name.ends_with("EditText"))
            .unwrap()
            .element_ref
            .clone();
        let test_value = format!("m2test{}", Uuid::now_v7().simple());
        let mut request = mobile_action_request(&capture);
        request.target = MobileActionTarget::Type {
            element_ref: password_ref,
        };
        request.text = Some(mobile_runtime::SensitiveText::new(test_value.clone()));
        let execution = host.execute_action(request, None, &allowed);
        assert_eq!(
            execution.receipt.decision,
            MobileActionDecision::Denied(MobileActionDenyReason::SensitiveScreen)
        );
        assert_eq!(execution.receipt.status, MobileActionStatus::Blocked);
        assert!(!execution.receipt.command_sent);
        assert!(execution.capture.is_none());
        let directory = TempDir::new().unwrap();
        let database = directory.path().join("m2-sensitive-smoke.sqlite3");
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let app = runtime
            .block_on(ordinconn_app::AppRuntime::initialize(&database))
            .unwrap();
        runtime
            .block_on(crate::commands::record_mobile_capture_workspace(
                app.as_ref(),
                host.environment_diagnostics(None),
                capture.clone(),
            ))
            .unwrap();
        let mut events = app.subscribe();
        runtime
            .block_on(app.record_mobile_action(&execution.receipt, None))
            .unwrap();
        assert_eq!(
            events.try_recv().unwrap().event_type,
            "mobile.action_requested"
        );
        assert_eq!(
            events.try_recv().unwrap().event_type,
            "mobile.action_blocked"
        );
        let serialized = serde_json::to_string(&(
            capture,
            execution.receipt,
            runtime.block_on(app.mobile_workspace_data()).unwrap(),
        ))
        .unwrap();
        assert!(!serialized.contains(&test_value));
        assert!(
            !fs::read(&database)
                .unwrap()
                .windows(test_value.len())
                .any(|window| { window == test_value.as_bytes() })
        );
        println!("M2_REAL_SENSITIVE_TYPE_BLOCK=PASS plaintext_persisted=false");
        assert!(host.stop_session());
    }

    #[test]
    fn real_sensitive_redaction_smoke_is_explicitly_gated() {
        if std::env::var("ORDINCONN_MOBILE_SENSITIVE_SMOKE").as_deref() != Ok("1") {
            return;
        }
        let secret = std::env::var("ORDINCONN_MOBILE_TEST_SECRET")
            .expect("the temporary sensitive-smoke value must be provided by the caller");
        let host = MobileHost::discover();
        let capture = host
            .observe_with_sdk(None, &["com.ordinconn.m15test".into()])
            .expect("the temporary sensitive test app must be visible on an online emulator");
        let serialized = serde_json::to_string(&capture).unwrap();

        assert!(!serialized.contains(&secret));
        assert!(!capture.snapshot.redactions.is_empty());
        assert_eq!(
            capture.snapshot.sensitive_state,
            Some(mobile_runtime::VerificationResult::SensitiveFieldBlocked)
        );
        assert!(
            capture
                .snapshot
                .elements
                .iter()
                .any(|element| element.text.as_deref() == Some("[REDACTED]"))
        );
        println!(
            "sensitive_redaction=PASS redactions={} serialized_secret_present=false",
            capture.snapshot.redactions.len()
        );
    }

    #[test]
    fn sdk_configuration_and_session_identity_are_stable() {
        let host = MobileHost::new(None);
        assert!(host.session_id().starts_with("mobile_session_"));
    }

    fn executable(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn android_environment_detects_tools_avds_and_only_online_emulators() {
        let fixture = TempDir::new().unwrap();
        let sdk = fixture.path().join("sdk");
        let avd_home = fixture.path().join(".android/avd");
        executable(
            &sdk.join("platform-tools/adb"),
            r##"#!/bin/sh
if [ "$1" = "version" ]; then echo "Android Debug Bridge version 1.0.41"; exit 0; fi
if [ "$1" = "devices" ]; then
  printf 'List of devices attached\nemulator-5554 device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64\nemulator-5556 offline\nR58M123 device product:phone\nemulator-5558 unauthorized\n'
  exit 0
fi
if [ "$3" = "emu" ] && [ "$4" = "avd" ]; then echo "Pixel_9_API_36"; echo "OK"; exit 0; fi
exit 1
"##,
        );
        executable(
            &sdk.join("emulator/emulator"),
            "#!/bin/sh\nif [ \"$1\" = \"-list-avds\" ]; then echo Pixel_9_API_36; fi\n",
        );
        executable(
            &sdk.join("cmdline-tools/latest/bin/sdkmanager"),
            "#!/bin/sh\nexit 0\n",
        );
        executable(
            &sdk.join("cmdline-tools/latest/bin/avdmanager"),
            "#!/bin/sh\nexit 0\n",
        );
        fs::create_dir_all(avd_home.join("Pixel_9_API_36.avd")).unwrap();
        fs::write(
            avd_home.join("Pixel_9_API_36.avd/config.ini"),
            "hw.device.name=pixel_9\nabi.type=arm64-v8a\n",
        )
        .unwrap();

        let diagnostics =
            AndroidEnvironmentDetector::new(vec![sdk.clone()], Vec::new(), Some(avd_home)).detect();

        assert_eq!(diagnostics.sdk_root.as_deref(), sdk.to_str());
        assert_eq!(
            diagnostics.adb_path.as_deref(),
            sdk.join("platform-tools/adb").to_str()
        );
        assert_eq!(
            diagnostics.adb_version.as_deref(),
            Some("Android Debug Bridge version 1.0.41")
        );
        assert_eq!(diagnostics.online_devices.len(), 1);
        assert_eq!(diagnostics.online_devices[0].id, "emulator-5554");
        assert_eq!(
            diagnostics.online_devices[0].avd_name.as_deref(),
            Some("Pixel_9_API_36")
        );
        assert_eq!(diagnostics.available_avds.len(), 1);
        assert_eq!(
            diagnostics.available_avds[0].device_profile.as_deref(),
            Some("pixel_9")
        );
        assert_eq!(
            diagnostics.available_avds[0].architecture.as_deref(),
            Some("arm64-v8a")
        );
        assert!(diagnostics.available_avds[0].running);
        assert!(diagnostics.sdkmanager_path.is_some());
        assert!(diagnostics.avdmanager_path.is_some());
    }

    #[test]
    fn android_environment_normalizes_direct_adb_and_reports_missing_siblings() {
        let fixture = TempDir::new().unwrap();
        let sdk = fixture.path().join("sdk");
        let adb = sdk.join("platform-tools/adb");
        executable(
            &adb,
            "#!/bin/sh\nif [ \"$1\" = \"version\" ]; then echo adb-test; exit 0; fi\nif [ \"$1\" = \"devices\" ]; then echo 'List of devices attached'; exit 0; fi\nexit 1\n",
        );

        let diagnostics = AndroidEnvironmentDetector::new(
            vec![adb.clone()],
            Vec::new(),
            Some(fixture.path().join("empty-avd-home")),
        )
        .detect();

        assert_eq!(diagnostics.sdk_root.as_deref(), sdk.to_str());
        assert_eq!(diagnostics.adb_path.as_deref(), adb.to_str());
        assert!(diagnostics.emulator_path.is_none());
        assert!(diagnostics.available_avds.is_empty());
        assert!(diagnostics.online_devices.is_empty());
    }

    #[test]
    fn android_environment_preserves_standalone_configured_adb() {
        let fixture = TempDir::new().unwrap();
        let adb = fixture.path().join("custom/bin/adb");
        executable(
            &adb,
            "#!/bin/sh\nif [ \"$1\" = \"version\" ]; then echo adb-custom; exit 0; fi\nif [ \"$1\" = \"devices\" ]; then echo 'List of devices attached'; exit 0; fi\nexit 1\n",
        );

        let diagnostics =
            AndroidEnvironmentDetector::new(vec![adb.clone()], Vec::new(), None).detect();

        assert_eq!(diagnostics.sdk_root, None);
        assert_eq!(diagnostics.adb_path.as_deref(), adb.to_str());
        assert_eq!(diagnostics.adb_status, "ready");
        assert_eq!(diagnostics.adb_version.as_deref(), Some("adb-custom"));
    }

    #[test]
    fn configured_sdk_drives_diagnostics_and_observation_consistently() {
        let fixture = TempDir::new().unwrap();
        let stale_adb = fixture.path().join("stale/adb");
        executable(&stale_adb, "#!/bin/sh\nexit 1\n");
        let sdk = fixture.path().join("selected-sdk");
        executable(
            &sdk.join("platform-tools/adb"),
            &format!(
                r##"#!/bin/sh
if [ "$1" = "version" ]; then echo adb-selected; exit 0; fi
if [ "$1" = "devices" ]; then printf 'List of devices attached\nemulator-5554 device\n'; exit 0; fi
case "$*" in
  *ro.build.version.release*) echo 15 ;;
  *"wm size"*) echo 'Physical size: 1080x2400' ;;
  *"dumpsys window"*) echo '{}' ;;
  *"uiautomator dump"*) echo dumped ;;
  *"cat /sdcard/ordinconn-ui.xml"*) printf '%s' '{}' ;;
  *"screencap -p"*) printf PNG ;;
  *) exit 1 ;;
esac
"##,
                FOCUS, XML
            ),
        );
        let host = MobileHost::new(Some(stale_adb));

        let capture = host
            .observe_with_sdk(sdk.to_str(), &["com.example.news".into()])
            .unwrap();

        assert_eq!(capture.session.device_id, "emulator-5554");
        assert_eq!(capture.observation.package_name, "com.example.news");
    }

    #[test]
    fn android_16_observation_uses_full_window_dump_for_focus() {
        let fixture = TempDir::new().unwrap();
        let sdk = fixture.path().join("sdk");
        executable(
            &sdk.join("platform-tools/adb"),
            &format!(
                r##"#!/bin/sh
if [ "$1" = "version" ]; then echo adb-selected; exit 0; fi
if [ "$1" = "devices" ]; then printf 'List of devices attached\nemulator-5554 device\n'; exit 0; fi
case "$*" in
  *ro.build.version.release*) echo 16 ;;
  *"wm size"*) echo 'Physical size: 1080x2400' ;;
  *"dumpsys window windows"*) exit 1 ;;
  *"dumpsys window"*) echo '{}' ;;
  *"uiautomator dump"*) echo dumped ;;
  *"cat /sdcard/ordinconn-ui.xml"*) printf '%s' '{}' ;;
  *"screencap -p"*) printf PNG ;;
  *) exit 1 ;;
esac
"##,
                FOCUS, XML
            ),
        );
        let host = MobileHost::new(None);

        let capture = host
            .observe_with_sdk(sdk.to_str(), &["com.example.news".into()])
            .unwrap();

        assert_eq!(capture.session.os_version, "16");
        assert_eq!(capture.observation.package_name, "com.example.news");
    }

    #[test]
    fn android_environment_skips_incomplete_configured_root_for_later_sdk() {
        let fixture = TempDir::new().unwrap();
        let incomplete = fixture.path().join("configured-but-empty");
        let standard = fixture.path().join("standard-sdk");
        fs::create_dir_all(&incomplete).unwrap();
        executable(
            &standard.join("platform-tools/adb"),
            "#!/bin/sh\nif [ \"$1\" = \"version\" ]; then echo adb-standard; exit 0; fi\nif [ \"$1\" = \"devices\" ]; then echo 'List of devices attached'; exit 0; fi\nexit 1\n",
        );
        executable(
            &standard.join("emulator/emulator"),
            "#!/bin/sh\nif [ \"$1\" = \"-list-avds\" ]; then exit 0; fi\nexit 1\n",
        );

        let diagnostics =
            AndroidEnvironmentDetector::new(vec![incomplete, standard.clone()], Vec::new(), None)
                .detect();

        assert_eq!(diagnostics.sdk_root.as_deref(), standard.to_str());
        assert_eq!(diagnostics.adb_version.as_deref(), Some("adb-standard"));
        assert_eq!(diagnostics.emulator_status, "ready");
    }

    #[test]
    fn android_environment_marks_present_but_unusable_tools_as_error() {
        let fixture = TempDir::new().unwrap();
        let sdk = fixture.path().join("sdk");
        fs::create_dir_all(sdk.join("platform-tools")).unwrap();
        fs::write(sdk.join("platform-tools/adb"), "not executable").unwrap();
        executable(&sdk.join("emulator/emulator"), "#!/bin/sh\nexit 1\n");

        let diagnostics = AndroidEnvironmentDetector::new(vec![sdk], Vec::new(), None).detect();

        assert_eq!(diagnostics.sdk_status, "detected");
        assert_eq!(diagnostics.adb_status, "error");
        assert_eq!(diagnostics.emulator_status, "error");
        assert!(diagnostics.adb_version.is_none());
        assert!(diagnostics.available_avds.is_empty());
    }

    #[test]
    fn android_environment_command_timeout_kills_a_hung_tool() {
        let fixture = TempDir::new().unwrap();
        let tool = fixture.path().join("hung-tool");
        executable(&tool, "#!/bin/sh\nwhile :; do :; done\n");
        let started = Instant::now();

        let output = run_command_text_with_timeout(&tool, &[], Duration::from_millis(30));

        assert_eq!(output, None);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn adb_command_drains_large_stdout_before_process_exit() {
        let fixture = TempDir::new().unwrap();
        let tool = fixture.path().join("large-output-tool");
        executable(
            &tool,
            "#!/bin/sh\ndd if=/dev/zero bs=1024 count=256 2>/dev/null\n",
        );

        let output = run_command_bytes_with_timeout(&tool, &[], Duration::from_secs(2)).unwrap();

        assert_eq!(output.len(), 256 * 1024);
    }

    #[test]
    fn command_returns_after_success_when_descendant_keeps_stdout_open() {
        let fixture = TempDir::new().unwrap();
        let tool = fixture.path().join("successful-tool-with-descendant");
        executable(
            &tool,
            "#!/bin/sh\n(trap '' HUP TERM; sleep 3) &\nexec /usr/bin/printf done\n",
        );
        let started = Instant::now();

        let output = run_command_bytes_with_timeout(&tool, &[], Duration::from_secs(2))
            .expect("direct child succeeded before its deadline");

        assert_eq!(output, b"done");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn command_timeout_does_not_wait_for_descendant_inherited_pipe() {
        let fixture = TempDir::new().unwrap();
        let tool = fixture.path().join("timed-out-tool-with-descendant");
        executable(
            &tool,
            "#!/bin/sh\n(trap '' HUP TERM; sleep 3) &\nwhile :; do :; done\n",
        );
        let started = Instant::now();

        let error = run_command_bytes_with_timeout(&tool, &[], Duration::from_millis(30))
            .expect_err("direct child must hit the deadline");

        assert_eq!(
            error,
            MobileHostError::CommandFailed("command timed out after 30 ms".into())
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    fn lifecycle_fixture(adb_body: &str) -> (TempDir, AndroidEnvironmentDetector) {
        let fixture = TempDir::new().unwrap();
        let sdk = fixture.path().join("sdk");
        executable(&sdk.join("platform-tools/adb"), adb_body);
        executable(
            &sdk.join("emulator/emulator"),
            "#!/bin/sh\nif [ \"$1\" = \"-list-avds\" ]; then echo Pixel_9_API_36; exit 0; fi\nexit 0\n",
        );
        let detector = AndroidEnvironmentDetector::new(
            vec![sdk],
            Vec::new(),
            Some(fixture.path().join(".android/avd")),
        );
        (fixture, detector)
    }

    #[test]
    fn concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup() {
        // Model cold parallel process startup without changing production deadlines.
        // Each worker owns its SDK, ADB script, Emulator script, and AVD home.
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let handles = (0..4)
            .map(|index| {
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let fixture = TempDir::new().unwrap();
                    let sdk = fixture.path().join("sdk");
                    let name = format!("Fixture_AVD_{index}");
                    executable(
                        &sdk.join("platform-tools/adb"),
                        &format!(
                            r##"#!/bin/sh
if [ "$1" = "version" ]; then sleep 0.2; echo adb-test; exit 0; fi
if [ "$1" = "devices" ]; then sleep 0.2; printf 'List of devices attached\nemulator-5554 device\n'; exit 0; fi
if [ "$3" = "emu" ] && [ "$4" = "avd" ]; then sleep 0.2; echo {name}; echo OK; exit 0; fi
if [ "$3" = "shell" ] && [ "$4" = "getprop" ]; then sleep 0.2; echo 1; exit 0; fi
exit 1
"##
                        ),
                    );
                    executable(
                        &sdk.join("emulator/emulator"),
                        &format!(
                            "#!/bin/sh\nif [ \"$1\" = \"-list-avds\" ]; then echo {name}; exit 0; fi\nexit 1\n"
                        ),
                    );
                    let detector = AndroidEnvironmentDetector::new(
                        vec![sdk],
                        Vec::new(),
                        Some(fixture.path().join(".android/avd")),
                    );
                    let host = MobileHost::with_detector(detector);
                    barrier.wait();
                    let device = host
                        .start_avd_with_timeout(&name, Duration::from_secs(3))
                        .expect("independent fixture should complete within its test budget");
                    assert_eq!(device.avd_name.as_deref(), Some(name.as_str()));
                    assert!(host.stop_session());
                    assert!(!host.stop_session());
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn avd_lifecycle_rejects_unknown_avd_and_times_out_without_boot() {
        let (_fixture, detector) = lifecycle_fixture(
            "#!/bin/sh\nif [ \"$1\" = \"version\" ]; then echo adb-test; exit 0; fi\nif [ \"$1\" = \"devices\" ]; then echo 'List of devices attached'; exit 0; fi\nexit 1\n",
        );
        let host = MobileHost::with_detector(detector);
        assert_eq!(
            host.start_avd_with_timeout("Missing_AVD", std::time::Duration::from_secs(3))
                .unwrap_err(),
            MobileHostError::AvdNotFound("Missing_AVD".into())
        );
        assert_eq!(
            host.start_avd_with_timeout("Pixel_9_API_36", std::time::Duration::from_millis(50),)
                .unwrap_err(),
            MobileHostError::AvdBootTimeout("Pixel_9_API_36".into())
        );
        assert!(!host.stop_session());
    }

    #[test]
    fn avd_lifecycle_waits_for_online_boot_and_shutdown_is_idempotent() {
        let (_fixture, detector) = lifecycle_fixture(
            r##"#!/bin/sh
if [ "$1" = "version" ]; then echo adb-test; exit 0; fi
if [ "$1" = "devices" ]; then printf 'List of devices attached\nemulator-5554 device model:sdk_gphone64_arm64\n'; exit 0; fi
if [ "$3" = "emu" ] && [ "$4" = "avd" ]; then echo Pixel_9_API_36; echo OK; exit 0; fi
if [ "$3" = "shell" ] && [ "$4" = "getprop" ]; then echo 1; exit 0; fi
exit 1
"##,
        );
        let host = MobileHost::with_detector(detector);

        let device = host
            .start_avd_with_timeout("Pixel_9_API_36", std::time::Duration::from_secs(3))
            .unwrap();

        assert_eq!(device.id, "emulator-5554");
        assert!(host.stop_session());
        assert!(!host.stop_session());
    }

    #[test]
    fn m1_5_gate_fails_missing_environment_and_blocks_capture_checks() {
        let report = m1_5_environment_gate(&AndroidEnvironmentDiagnostics::default());

        assert!(!report.passed);
        assert_eq!(report.status("ADB_READY"), Some(MobileGateStatus::Fail));
        assert_eq!(
            report.status("EMULATOR_READY"),
            Some(MobileGateStatus::Fail)
        );
        assert_eq!(report.status("DEVICE_ONLINE"), Some(MobileGateStatus::Fail));
        for name in [
            "FRAME_CAPTURE",
            "UI_TREE_CAPTURE",
            "SNAPSHOT_PARSE",
            "ELEMENT_REFS",
            "MOBILE_OBSERVATION",
            "WORKSPACE_PROJECTION",
            "SESSION_SHUTDOWN",
        ] {
            assert_eq!(report.status(name), Some(MobileGateStatus::Blocked));
        }
    }

    #[test]
    fn m1_5_gate_reports_capture_failure_without_aborting_the_report() {
        let diagnostics = AndroidEnvironmentDiagnostics {
            sdk_status: "detected".into(),
            adb_status: "ready".into(),
            emulator_status: "ready".into(),
            adb_path: Some("/sdk/platform-tools/adb".into()),
            emulator_path: Some("/sdk/emulator/emulator".into()),
            online_devices: vec![AndroidDeviceInfo {
                id: "emulator-5554".into(),
                status: "device".into(),
                model: None,
                avd_name: Some("Pixel_9_API_36".into()),
            }],
            ..AndroidEnvironmentDiagnostics::default()
        };

        let report = m1_5_capture_failure_gate(&diagnostics, "uiautomator failed");

        assert_eq!(report.checks.len(), 10);
        assert_eq!(report.status("ADB_READY"), Some(MobileGateStatus::Pass));
        assert_eq!(report.status("DEVICE_ONLINE"), Some(MobileGateStatus::Pass));
        assert_eq!(report.status("FRAME_CAPTURE"), Some(MobileGateStatus::Fail));
        assert_eq!(
            report.status("WORKSPACE_PROJECTION"),
            Some(MobileGateStatus::Blocked)
        );
    }

    #[test]
    fn mobile_ipc_projection_persists_capture_events_and_workspace_contract() {
        let directory = TempDir::new().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let app_runtime = runtime
            .block_on(ordinconn_app::AppRuntime::initialize(
                &directory.path().join("ipc.sqlite3"),
            ))
            .unwrap();
        let capture = capture_from_outputs(
            &AdbObservationOutput {
                device_id: "emulator-5554".into(),
                os_version: "15".into(),
                size_output: "Physical size: 1080x2400".into(),
                focus_output: FOCUS.into(),
                ui_xml: XML.into(),
                png: vec![1, 2, 3],
            },
            &["com.example.news".into()],
        )
        .unwrap();
        let observation_id = capture.observation.id.clone();
        let host = MobileHost::new(None);
        let mut events = app_runtime.subscribe();

        let workspace = runtime
            .block_on(crate::commands::record_mobile_capture_workspace(
                app_runtime.as_ref(),
                host.environment_diagnostics(None),
                capture,
            ))
            .unwrap();

        assert!(
            workspace
                .observations
                .iter()
                .any(|item| item.id == observation_id)
        );
        assert!(
            workspace
                .feed
                .iter()
                .any(|item| { item.id == observation_id && item.source_method == "MOBILE" })
        );
        assert!(serde_json::to_value(&workspace).is_ok());
        let event_types = (0..3)
            .map(|_| events.try_recv().unwrap().event_type)
            .collect::<Vec<_>>();
        assert_eq!(
            event_types,
            vec![
                "mobile.session_started",
                "mobile.snapshot",
                "mobile.observation"
            ]
        );
    }

    #[test]
    fn mobile_ipc_stop_ends_the_persisted_session_without_stopping_the_avd() {
        let directory = TempDir::new().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let app_runtime = runtime
            .block_on(ordinconn_app::AppRuntime::initialize(
                &directory.path().join("ipc-stop.sqlite3"),
            ))
            .unwrap();
        let host = MobileHost::new(None);
        let capture = bind_capture_to_session(
            capture_from_outputs(
                &AdbObservationOutput {
                    device_id: "emulator-5554".into(),
                    os_version: "15".into(),
                    size_output: "Physical size: 1080x2400".into(),
                    focus_output: FOCUS.into(),
                    ui_xml: XML.into(),
                    png: vec![1, 2, 3],
                },
                &["com.example.news".into()],
            )
            .unwrap(),
            &host.session_id(),
        );
        host.session_active.store(true, Ordering::SeqCst);
        runtime
            .block_on(crate::commands::record_mobile_capture_workspace(
                app_runtime.as_ref(),
                host.environment_diagnostics(None),
                capture,
            ))
            .unwrap();
        let mut events = app_runtime.subscribe();

        let workspace = runtime
            .block_on(crate::commands::stop_mobile_workspace(
                app_runtime.as_ref(),
                &host,
            ))
            .unwrap();

        assert_eq!(workspace.runtime_status, "disconnected");
        assert!(workspace.session.is_none());
        assert!(workspace.frame.is_none());
        assert!(workspace.ui_snapshot.is_none());
        assert!(!host.is_session_active());
        assert_eq!(
            events.try_recv().unwrap().event_type,
            "mobile.session_ended"
        );
    }
}
