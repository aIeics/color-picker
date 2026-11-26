//! The platform thread. It owns everything that must keep working while the
//! main window is hidden or busy: the global pick hotkey and the pick session
//! (low-level mouse and keyboard hooks plus the loupe). The app sends it
//! [`Command`]s and hears back through [`Event`]s; every event also wakes
//! the app with `request_repaint`.
//!
//! Hooks are installed only while picking, and the hook callbacks only store
//! the cursor and post a message: Windows silently drops slow hooks. Every
//! pick ends one way or another: click, Esc, right-click, the hotkey again,
//! a minute without moving the mouse, or the app quitting.

use std::cell::RefCell;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering::Relaxed};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui;
use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
    UnregisterHotKey, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, HHOOK, KBDLLHOOKSTRUCT, KillTimer, MSG, MSLLHOOKSTRUCT, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetCursorPos, SetTimer,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APP,
    WM_DESTROY, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

use super::capture;
use super::loupe::Loupe;
use super::tray::{self, Tray};
use super::wide;
use crate::hotkey::Hotkey;
use crate::platform::{Command, Event};
use crate::screen::PickStyle as Style;
use crate::settings::ZOOM_LEVELS;

/// Class of the hidden service window; a second instance finds the first
/// one by it.
pub(super) const CLASS: &str = "ColorPickerService";

const WM_APP_COMMAND: u32 = WM_APP + 1;
const WM_APP_MOVE: u32 = WM_APP + 2;
const WM_APP_CLICK: u32 = WM_APP + 3;
const WM_APP_CANCEL: u32 = WM_APP + 4;
const WM_APP_KEY: u32 = WM_APP + 5;
const WM_APP_ZOOM: u32 = WM_APP + 6;
const WM_APP_RELEASE: u32 = WM_APP + 7;
/// Sent by a second instance: show the window.
pub(super) const WM_APP_SHOW: u32 = WM_APP + 8;
const WM_APP_QUIT: u32 = WM_APP + 9;
const WM_APP_TRAY: u32 = WM_APP + 10;
/// Tray notifications (NOTIFYICON_VERSION_4): left click, keyboard select.
const NIN_SELECT: u32 = 0x400;
const NIN_KEYSELECT: u32 = 0x401;
const WM_CONTEXTMENU: u32 = 0x007B;

const HOTKEY_ID: i32 = 1;
const TIMER_WATCH: usize = 1;
/// A pick with no mouse movement for this long is cancelled.
const IDLE_LIMIT: Duration = Duration::from_secs(60);
/// The mouse hook waits this long for a swallowed button-up at most.
const RELEASE_LIMIT: Duration = Duration::from_secs(2);

/// Handle to the platform thread. Dropping it stops the thread.
pub struct Service {
    hwnd: isize,
    commands: Sender<Command>,
    events: Receiver<Event>,
    stopped: Receiver<()>,
}

impl Service {
    pub fn start(ctx: egui::Context, style: Style, hotkey: Option<Hotkey>) -> Option<Self> {
        let (commands_tx, commands) = mpsc::channel();
        let (events_tx, events) = mpsc::channel();
        let (ready_tx, ready) = mpsc::sync_channel(1);
        let (stopped_tx, stopped) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("platform".into())
            .spawn(move || {
                run(ctx, commands, events_tx, &ready_tx, style, hotkey);
                let _ = stopped_tx.send(());
            })
            .ok()?;
        let hwnd = ready.recv().ok()??;
        Some(Self {
            hwnd,
            commands: commands_tx,
            events,
            stopped,
        })
    }

    pub fn send(&self, command: Command) {
        if self.commands.send(command).is_ok() {
            // SAFETY: posting to our own window; fails harmlessly if gone.
            unsafe { PostMessageW(self.hwnd as HWND, WM_APP_COMMAND, 0, 0) };
        }
    }

    /// The next event, if any.
    pub fn poll(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: as in `send`. Destroying the window ends its thread.
        unsafe { PostMessageW(self.hwnd as HWND, WM_APP_QUIT, 0, 0) };
        // Wait briefly so hooks and the hotkey are removed cleanly; Windows
        // removes them anyway when the process exits.
        let _ = self.stopped.recv_timeout(Duration::from_millis(500));
    }
}

// The hook callbacks run on the platform thread but get no context, so
// what they need lives in statics.
static SERVICE_HWND: AtomicIsize = AtomicIsize::new(0);
static PICKING: AtomicBool = AtomicBool::new(false);
static SWALLOW_LEFT_UP: AtomicBool = AtomicBool::new(false);
static SWALLOW_RIGHT_UP: AtomicBool = AtomicBool::new(false);
static MOVE_PENDING: AtomicBool = AtomicBool::new(false);
static CURSOR_X: AtomicI32 = AtomicI32::new(0);
static CURSOR_Y: AtomicI32 = AtomicI32::new(0);

struct State {
    ctx: egui::Context,
    events: Sender<Event>,
    commands: Receiver<Command>,
    hwnd: HWND,
    style: Style,
    loupe: Option<Loupe>,
    mouse_hook: HHOOK,
    keyboard_hook: HHOOK,
    /// When the current pick last saw the mouse move.
    active_at: Option<Instant>,
    /// When the pick ended while a button-up still had to be swallowed.
    released_at: Option<Instant>,
    tray: Option<Tray>,
    /// Broadcast when Explorer (re)starts; the tray icon must be re-added.
    taskbar_created: u32,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Runs `f` on the thread's state. Returns `None` instead of panicking if
/// the state is missing or already borrowed (a re-entrant window message).
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

fn run(
    ctx: egui::Context,
    commands: Receiver<Command>,
    events: Sender<Event>,
    ready: &mpsc::SyncSender<Option<isize>>,
    style: Style,
    hotkey: Option<Hotkey>,
) {
    let class = wide(CLASS);
    // SAFETY: standard window creation on this thread, valid strings.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(null_mut());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class.as_ptr(),
            wide("Color Picker service").as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        let _ = ready.send(None);
        return;
    }
    SERVICE_HWND.store(hwnd as isize, Relaxed);
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(State {
            ctx,
            events,
            commands,
            hwnd,
            style,
            loupe: None,
            mouse_hook: null_mut(),
            keyboard_hook: null_mut(),
            active_at: None,
            released_at: None,
            tray: None,
            // SAFETY: registers (or looks up) a well-known message name.
            taskbar_created: unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) },
        });
    });
    with_state(|s| set_hotkey(s, hotkey));
    let _ = ready.send(Some(hwnd as isize));

    // SAFETY: the standard message loop for this thread's windows.
    unsafe {
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    with_state(|s| {
        end_pick(s);
        remove_hooks(s);
        s.loupe = None;
        s.tray = None;
    });
    STATE.with(|cell| cell.borrow_mut().take());
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APP_COMMAND => {
            while let Some(Some(command)) = with_state(|s| s.commands.try_recv().ok()) {
                with_state(|s| handle_command(s, command));
            }
        }
        WM_HOTKEY => {
            with_state(|s| {
                if PICKING.load(Relaxed) {
                    cancel_pick(s);
                } else {
                    start_pick(s);
                }
            });
        }
        WM_APP_MOVE => {
            MOVE_PENDING.store(false, Relaxed);
            with_state(|s| {
                s.active_at = Some(Instant::now());
                refresh_loupe(s, cursor_from_hook());
            });
        }
        WM_APP_CLICK => {
            with_state(|s| finish_pick(s, cursor_from_hook()));
        }
        WM_APP_CANCEL => {
            with_state(cancel_pick);
        }
        WM_APP_KEY => {
            with_state(|s| handle_key(s, wparam as u16));
        }
        WM_APP_ZOOM => {
            with_state(|s| change_zoom(s, wparam as isize as i16));
        }
        WM_APP_RELEASE => {
            with_state(|s| {
                if !PICKING.load(Relaxed) {
                    remove_hooks(s);
                }
            });
        }
        WM_APP_SHOW => {
            with_state(|s| emit(s, Event::ShowRequested));
        }
        WM_TIMER if wparam == TIMER_WATCH => {
            with_state(watch);
        }
        WM_APP_QUIT => {
            with_state(|s| {
                end_pick(s);
                remove_hooks(s);
                s.tray = None;
            });
            // SAFETY: our own window; sends WM_DESTROY, which ends the loop.
            unsafe { DestroyWindow(hwnd) };
        }
        WM_APP_TRAY => match (lparam as u32) & 0xFFFF {
            NIN_SELECT | NIN_KEYSELECT => {
                with_state(|s| emit(s, Event::ShowRequested));
            }
            WM_CONTEXTMENU => {
                // The menu runs a modal loop, so no state is borrowed here.
                let command = tray::show_menu(hwnd);
                with_state(|s| match command {
                    tray::MENU_SHOW => emit(s, Event::ShowRequested),
                    tray::MENU_PICK => start_pick(s),
                    tray::MENU_EXIT => emit(s, Event::ExitRequested),
                    _ => {}
                });
            }
            _ => {}
        },
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
        }
        _ if with_state(|s| s.taskbar_created != 0 && msg == s.taskbar_created) == Some(true) => {
            with_state(|s| {
                if let Some(tray) = &s.tray {
                    tray.readd(WM_APP_TRAY);
                }
            });
        }
        // SAFETY: default handling for everything else.
        _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
    0
}

fn emit(state: &State, event: Event) {
    let _ = state.events.send(event);
    state.ctx.request_repaint();
}

fn handle_command(state: &mut State, command: Command) {
    match command {
        Command::SetHotkey(hotkey) => set_hotkey(state, hotkey),
        Command::SetStyle(style) => {
            state.style = style;
            if PICKING.load(Relaxed) {
                refresh_loupe(state, current_cursor());
            }
        }
        Command::StartPick => start_pick(state),
        Command::CancelPick => {
            if PICKING.load(Relaxed) {
                cancel_pick(state);
            }
        }
        Command::SetTray(show) => {
            if !show {
                state.tray = None;
            } else if state.tray.is_none() {
                state.tray = Tray::add(state.hwnd, WM_APP_TRAY);
            }
        }
    }
}

fn set_hotkey(state: &mut State, hotkey: Option<Hotkey>) {
    // SAFETY: (un)registering on our own window; harmless if not registered.
    unsafe { UnregisterHotKey(state.hwnd, HOTKEY_ID) };
    let Some(hotkey) = hotkey else {
        emit(state, Event::HotkeyStatus(None));
        return;
    };
    let Some(vk) = hotkey.virtual_key() else {
        emit(state, Event::HotkeyStatus(Some("Unsupported key".into())));
        return;
    };
    let mut modifiers = MOD_NOREPEAT;
    for (on, flag) in [
        (hotkey.ctrl, MOD_CONTROL),
        (hotkey.alt, MOD_ALT),
        (hotkey.shift, MOD_SHIFT),
        (hotkey.win, MOD_WIN),
    ] {
        if on {
            modifiers |= flag;
        }
    }
    // SAFETY: plain values.
    let ok = unsafe { RegisterHotKey(state.hwnd, HOTKEY_ID, modifiers, u32::from(vk)) } != 0;
    let error = (!ok).then(|| {
        // SAFETY: reads this thread's last error.
        match unsafe { GetLastError() } {
            1409 => "Used by another app".to_owned(), // ERROR_HOTKEY_ALREADY_REGISTERED
            code => format!("Couldn't register (error {code})"),
        }
    });
    emit(state, Event::HotkeyStatus(error));
}

fn start_pick(state: &mut State) {
    if PICKING.load(Relaxed) {
        return;
    }
    // SAFETY: low-level hooks need the module handle and a message loop on
    // this thread, which `run` provides.
    unsafe {
        let module = GetModuleHandleW(null_mut());
        if state.mouse_hook.is_null() {
            state.mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0);
        }
        if state.keyboard_hook.is_null() {
            state.keyboard_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0);
        }
    }
    if state.mouse_hook.is_null() {
        remove_hooks(state);
        emit(state, Event::PickCancelled);
        return;
    }
    if state.loupe.is_none() {
        state.loupe = Loupe::new();
    }
    SWALLOW_LEFT_UP.store(false, Relaxed);
    SWALLOW_RIGHT_UP.store(false, Relaxed);
    PICKING.store(true, Relaxed);
    state.active_at = Some(Instant::now());
    state.released_at = None;
    // SAFETY: a timer on our own window.
    unsafe { SetTimer(state.hwnd, TIMER_WATCH, 500, None) };
    refresh_loupe(state, current_cursor());
    emit(state, Event::PickStarted);
}

fn finish_pick(state: &mut State, point: POINT) {
    if state.active_at.is_none() {
        return;
    }
    let color = capture::sample_at(point, state.style.sample);
    end_pick(state);
    match color {
        Some(color) => emit(state, Event::Picked(color)),
        None => emit(state, Event::PickCancelled),
    }
}

fn cancel_pick(state: &mut State) {
    if state.active_at.is_none() {
        return;
    }
    end_pick(state);
    emit(state, Event::PickCancelled);
}

/// Stops picking. The mouse hook stays until a swallowed button-up arrives
/// (or [`RELEASE_LIMIT`] passes), so the click never reaches the window
/// under the cursor.
fn end_pick(state: &mut State) {
    PICKING.store(false, Relaxed);
    state.active_at = None;
    if let Some(loupe) = &mut state.loupe {
        loupe.hide();
    }
    if !state.keyboard_hook.is_null() {
        // SAFETY: our own hook handle.
        unsafe { UnhookWindowsHookEx(state.keyboard_hook) };
        state.keyboard_hook = null_mut();
    }
    if SWALLOW_LEFT_UP.load(Relaxed) || SWALLOW_RIGHT_UP.load(Relaxed) {
        state.released_at = Some(Instant::now());
    } else {
        remove_hooks(state);
    }
}

fn remove_hooks(state: &mut State) {
    // SAFETY: our own hook handles and timer.
    unsafe {
        for hook in [&mut state.mouse_hook, &mut state.keyboard_hook] {
            if !hook.is_null() {
                UnhookWindowsHookEx(*hook);
                *hook = null_mut();
            }
        }
        KillTimer(state.hwnd, TIMER_WATCH);
    }
    SWALLOW_LEFT_UP.store(false, Relaxed);
    SWALLOW_RIGHT_UP.store(false, Relaxed);
    state.released_at = None;
}

/// Twice a second while hooks are installed: cancels idle picks and removes
/// a mouse hook still waiting for a button-up that never came.
fn watch(state: &mut State) {
    if state.active_at.is_some_and(|at| at.elapsed() > IDLE_LIMIT) {
        cancel_pick(state);
    }
    if !PICKING.load(Relaxed)
        && state
            .released_at
            .is_none_or(|at| at.elapsed() > RELEASE_LIMIT)
    {
        remove_hooks(state);
    }
}

fn handle_key(state: &mut State, vk: u16) {
    let step = |d: i32| {
        // SAFETY: reads the key state.
        let shift = unsafe { GetAsyncKeyState(i32::from(VK_SHIFT)) } < 0;
        if shift { d * 10 } else { d }
    };
    let (dx, dy) = match vk {
        VK_ESCAPE => return cancel_pick(state),
        VK_RETURN | VK_SPACE => return finish_pick(state, current_cursor()),
        VK_LEFT => (step(-1), 0),
        VK_RIGHT => (step(1), 0),
        VK_UP => (0, step(-1)),
        VK_DOWN => (0, step(1)),
        _ => return,
    };
    let cursor = current_cursor();
    let moved = POINT {
        x: cursor.x + dx,
        y: cursor.y + dy,
    };
    // SAFETY: plain values.
    unsafe { SetCursorPos(moved.x, moved.y) };
    state.active_at = Some(Instant::now());
    refresh_loupe(state, current_cursor());
}

fn change_zoom(state: &mut State, wheel: i16) {
    let current = ZOOM_LEVELS
        .iter()
        .position(|&z| z >= state.style.zoom)
        .unwrap_or(ZOOM_LEVELS.len() - 1);
    let next = if wheel > 0 {
        (current + 1).min(ZOOM_LEVELS.len() - 1)
    } else {
        current.saturating_sub(1)
    };
    if ZOOM_LEVELS[next] != state.style.zoom {
        state.style.zoom = ZOOM_LEVELS[next];
        emit(state, Event::ZoomChanged(state.style.zoom));
        refresh_loupe(state, current_cursor());
    }
}

fn refresh_loupe(state: &mut State, cursor: POINT) {
    if !PICKING.load(Relaxed) {
        return;
    }
    let style = state.style;
    if let Some(loupe) = &mut state.loupe {
        loupe.update(cursor, style);
    }
}

fn current_cursor() -> POINT {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: valid out pointer.
    unsafe { GetCursorPos(&mut point) };
    point
}

fn cursor_from_hook() -> POINT {
    POINT {
        x: CURSOR_X.load(Relaxed),
        y: CURSOR_Y.load(Relaxed),
    }
}

fn post(msg: u32, wparam: WPARAM) {
    let hwnd = SERVICE_HWND.load(Relaxed) as HWND;
    // SAFETY: posting is asynchronous and safe from a hook callback.
    unsafe { PostMessageW(hwnd, msg, wparam, 0) };
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        // SAFETY: for WH_MOUSE_LL, `lparam` points to an MSLLHOOKSTRUCT.
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let picking = PICKING.load(Relaxed);
        let store = || {
            CURSOR_X.store(info.pt.x, Relaxed);
            CURSOR_Y.store(info.pt.y, Relaxed);
        };
        match wparam as u32 {
            WM_MOUSEMOVE if picking => {
                store();
                if !MOVE_PENDING.swap(true, Relaxed) {
                    post(WM_APP_MOVE, 0);
                }
            }
            WM_LBUTTONDOWN if picking => {
                store();
                PICKING.store(false, Relaxed);
                SWALLOW_LEFT_UP.store(true, Relaxed);
                post(WM_APP_CLICK, 0);
                return 1;
            }
            WM_RBUTTONDOWN if picking => {
                PICKING.store(false, Relaxed);
                SWALLOW_RIGHT_UP.store(true, Relaxed);
                post(WM_APP_CANCEL, 0);
                return 1;
            }
            WM_LBUTTONUP if SWALLOW_LEFT_UP.swap(false, Relaxed) => {
                post(WM_APP_RELEASE, 0);
                return 1;
            }
            WM_RBUTTONUP if SWALLOW_RIGHT_UP.swap(false, Relaxed) => {
                post(WM_APP_RELEASE, 0);
                return 1;
            }
            WM_MOUSEWHEEL if picking => {
                let delta = (info.mouseData >> 16) as u16 as i16;
                post(WM_APP_ZOOM, delta as isize as WPARAM);
                return 1;
            }
            _ => {}
        }
    }
    // SAFETY: passes the event on to the next hook.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && PICKING.load(Relaxed) {
        // SAFETY: for WH_KEYBOARD_LL, `lparam` points to a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode as u16;
        if matches!(
            vk,
            VK_ESCAPE | VK_RETURN | VK_SPACE | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN
        ) {
            if matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
                post(WM_APP_KEY, vk as WPARAM);
            }
            // Swallowed: these keys belong to the pick, not the window below.
            return 1;
        }
    }
    // SAFETY: passes the event on to the next hook.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}
