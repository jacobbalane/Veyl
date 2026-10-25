//! Poke actions (§7.7) behind a trait, so tests never send real input.
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PokeAction {
    #[default]
    Click,
    PlayPause,
    NextTrack,
    Space,
    None,
}
impl PokeAction {
    pub fn parse(value: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(value.into())).ok()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    /// The target runs as administrator, so Windows drops input from Veyl.
    Blocked,
    Failed,
}
pub trait Actions: Send + Sync {
    fn perform(&self, action: PokeAction) -> Result<(), ActionError>;
}
/// Real input on Windows; elsewhere every action fails.
pub struct SystemActions;
impl Actions for SystemActions {
    fn perform(&self, action: PokeAction) -> Result<(), ActionError> {
        #[cfg(windows)]
        return native::perform(action);
        #[cfg(not(windows))]
        {
            let _ = action;
            Err(ActionError::Failed)
        }
    }
}
/// Reports each action's outcome: the action and whether Windows blocked it.
pub type ActionReport = Arc<dyn Fn(PokeAction, Result<(), ActionError>) + Send + Sync>;
/// Whether a poke acts, and the worker that performs it off the engine thread.
pub struct ActionsGate {
    enabled: AtomicBool,
    testing: AtomicBool,
    action: Mutex<PokeAction>,
    worker: Mutex<mpsc::Sender<PokeAction>>,
}
impl ActionsGate {
    pub fn new(
        enabled: bool,
        action: PokeAction,
        actions: Arc<dyn Actions>,
        report: ActionReport,
    ) -> Arc<Self> {
        let (worker, jobs) = mpsc::channel::<PokeAction>();
        std::thread::Builder::new()
            .name("Veyl-actions".into())
            .spawn(move || {
                for action in jobs {
                    let result = actions.perform(action);
                    if let Err(error) = result {
                        tracing::warn!("A poke action did not run: {error:?}.");
                    }
                    report(action, result);
                }
            })
            .expect("Actions worker could not start.");
        Arc::new(Self {
            enabled: AtomicBool::new(enabled),
            testing: AtomicBool::new(false),
            action: Mutex::new(action),
            worker: Mutex::new(worker),
        })
    }
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
    /// Returns whether the state changed.
    pub fn set_enabled(&self, enabled: bool) -> bool {
        self.enabled.swap(enabled, Ordering::AcqRel) != enabled
    }
    pub fn action(&self) -> PokeAction {
        *self
            .action
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
    pub fn set_action(&self, action: PokeAction) {
        *self
            .action
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = action;
    }
    /// The poke test and the onboarding practice never run actions.
    pub fn set_testing(&self, testing: bool) {
        self.testing.store(testing, Ordering::Release);
    }
    pub fn testing(&self) -> bool {
        self.testing.load(Ordering::Acquire)
    }
    /// Starts the poke's action when actions are on and no test runs. Returns whether it acts.
    pub fn poke(&self) -> bool {
        let action = self.action();
        if !self.enabled() || self.testing() || action == PokeAction::None {
            return false;
        }
        self.worker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .send(action)
            .is_ok()
    }
}

#[cfg(windows)]
mod native {
    use super::{ActionError, PokeAction};
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE, HWND, POINT},
        Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
            TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        },
        System::Threading::{
            GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::{
            Input::KeyboardAndMouse::{
                MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
                KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MOUSEEVENTF_LEFTDOWN,
                MOUSEEVENTF_LEFTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
                VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_SPACE,
            },
            WindowsAndMessaging::{
                GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, WindowFromPoint,
            },
        },
    };
    pub fn perform(action: PokeAction) -> Result<(), ActionError> {
        let (inputs, target) = match action {
            PokeAction::None => return Ok(()),
            PokeAction::Click => (
                vec![mouse(MOUSEEVENTF_LEFTDOWN), mouse(MOUSEEVENTF_LEFTUP)],
                pointer_window(),
            ),
            PokeAction::PlayPause => (key(VK_MEDIA_PLAY_PAUSE, 0), foreground()),
            PokeAction::NextTrack => (key(VK_MEDIA_NEXT_TRACK, 0), foreground()),
            PokeAction::Space => {
                // SAFETY: MapVirtualKeyW only reads its arguments.
                let scan = unsafe { MapVirtualKeyW(VK_SPACE.0 as u32, MAPVK_VK_TO_VSC) };
                (key(VK_SPACE, scan as u16), foreground())
            }
        };
        // Windows drops input to a more trusted window without saying so (UIPI).
        if target.is_some_and(elevated) {
            return Err(ActionError::Blocked);
        }
        // SAFETY: the slice holds initialized INPUT values and the size is that of one INPUT.
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize == inputs.len() {
            Ok(())
        } else {
            Err(ActionError::Blocked)
        }
    }
    fn mouse(flags: MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        }
    }
    fn key(key: VIRTUAL_KEY, scan: u16) -> Vec<INPUT> {
        [KEYBD_EVENT_FLAGS(0), KEYEVENTF_KEYUP]
            .into_iter()
            .map(|flags| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: scan,
                        dwFlags: flags,
                        ..Default::default()
                    },
                },
            })
            .collect()
    }
    fn pointer_window() -> Option<HWND> {
        let mut point = POINT::default();
        // SAFETY: both calls only read the pointer position.
        unsafe {
            GetCursorPos(&mut point).ok()?;
            let window = WindowFromPoint(point);
            (!window.is_invalid()).then_some(window)
        }
    }
    fn foreground() -> Option<HWND> {
        // SAFETY: no arguments; the handle may be null, which is checked.
        let window = unsafe { GetForegroundWindow() };
        (!window.is_invalid()).then_some(window)
    }
    /// Whether the window's process runs at a higher integrity level than Veyl.
    fn elevated(window: HWND) -> bool {
        let mut process_id = 0;
        // SAFETY: the window handle came from Windows; the out pointer is valid.
        unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
        if process_id == 0 {
            return false;
        }
        // SAFETY: a limited query handle is opened and closed here.
        let target =
            match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) } {
                Ok(process) => {
                    let level = integrity(process);
                    // SAFETY: the handle was opened above.
                    let _ = unsafe { CloseHandle(process) };
                    level
                }
                Err(_) => None,
            };
        // SAFETY: the pseudo handle of the current process needs no closing.
        let own = integrity(unsafe { GetCurrentProcess() });
        matches!((target, own), (Some(target), Some(own)) if target > own)
    }
    fn integrity(process: HANDLE) -> Option<u32> {
        let mut token = HANDLE::default();
        // SAFETY: token query on a valid process handle; the token is closed below.
        unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.ok()?;
        let mut size = 0;
        // SAFETY: the first call only reports the size; the buffer is u64-aligned for the label.
        let level = unsafe {
            let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut size);
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let read = GetTokenInformation(
                token,
                TokenIntegrityLevel,
                Some(buffer.as_mut_ptr().cast()),
                size,
                &mut size,
            );
            read.ok().and_then(|_| {
                let label = &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
                let count = *GetSidSubAuthorityCount(label.Label.Sid);
                (count > 0).then(|| *GetSidSubAuthority(label.Label.Sid, count as u32 - 1))
            })
        };
        // SAFETY: the token was opened above.
        let _ = unsafe { CloseHandle(token) };
        level
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::time::Duration;
    #[derive(Default)]
    pub(crate) struct FakeActions {
        pub performed: Mutex<Vec<PokeAction>>,
    }
    impl Actions for FakeActions {
        fn perform(&self, action: PokeAction) -> Result<(), ActionError> {
            self.performed.lock().unwrap().push(action);
            if action == PokeAction::Space {
                Err(ActionError::Blocked)
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn pokes_act_only_when_on_outside_tests_and_with_an_action() {
        let fake = Arc::new(FakeActions::default());
        let (reports, received) = mpsc::channel();
        let reports = Mutex::new(reports);
        let gate = ActionsGate::new(
            false,
            PokeAction::Click,
            fake.clone(),
            Arc::new(move |action, result| {
                reports.lock().unwrap().send((action, result)).unwrap();
            }),
        );
        assert!(!gate.poke());
        assert!(gate.set_enabled(true));
        assert!(!gate.set_enabled(true));
        assert!(gate.poke());
        gate.set_testing(true);
        assert!(!gate.poke());
        gate.set_testing(false);
        gate.set_action(PokeAction::None);
        assert!(!gate.poke());
        gate.set_action(PokeAction::Space);
        assert!(gate.poke());
        let wait = Duration::from_secs(2);
        assert_eq!(
            received.recv_timeout(wait).unwrap(),
            (PokeAction::Click, Ok(()))
        );
        assert_eq!(
            received.recv_timeout(wait).unwrap(),
            (PokeAction::Space, Err(ActionError::Blocked))
        );
        assert_eq!(
            *fake.performed.lock().unwrap(),
            [PokeAction::Click, PokeAction::Space]
        );
        assert_eq!(PokeAction::parse("play_pause"), Some(PokeAction::PlayPause));
        assert_eq!(PokeAction::parse("next_track"), Some(PokeAction::NextTrack));
        assert_eq!(PokeAction::parse("jump"), None);
    }
}
