//! Message passing between the web overlay (TypeScript) and the Bevy app.
//!
//! The browser owns the DOM, pointer lock and audio; it forwards input here as
//! commands. The app reports back through callbacks registered at startup.
//! WebAssembly runs single-threaded, so thread-local storage is sufficient.

use std::cell::RefCell;

use bevy::prelude::*;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Forward,
    Backward,
    Left,
    Right,
}

impl Move {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "forward" => Some(Self::Forward),
            "backward" => Some(Self::Backward),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Active(bool),
    Move(Move, bool),
    ClearMoves,
    Hurry(bool),
    Look { yaw: f32, pitch: f32 },
    Interact,
    Reset,
}

pub struct Callbacks {
    pub on_ready: js_sys::Function,
    pub on_target: js_sys::Function,
    pub on_toast: js_sys::Function,
    pub on_step: js_sys::Function,
    pub on_error: js_sys::Function,
}

thread_local! {
    static INBOX: RefCell<Vec<Command>> = const { RefCell::new(Vec::new()) };
    static CALLBACKS: RefCell<Option<Callbacks>> = const { RefCell::new(None) };
}

pub fn install(callbacks: Callbacks) {
    CALLBACKS.with(|slot| *slot.borrow_mut() = Some(callbacks));
}

fn push(command: Command) {
    INBOX.with(|inbox| inbox.borrow_mut().push(command));
}

/// Takes every command the browser has queued since the last frame.
pub fn drain() -> Vec<Command> {
    INBOX.with(|inbox| std::mem::take(&mut *inbox.borrow_mut()))
}

fn call(pick: impl FnOnce(&Callbacks) -> &js_sys::Function, argument: JsValue) {
    CALLBACKS.with(|slot| {
        if let Some(callbacks) = slot.borrow().as_ref()
            && let Err(error) = pick(callbacks).call1(&JsValue::NULL, &argument)
        {
            error!("overlay callback failed: {error:?}");
        }
    });
}

pub fn ready() {
    call(|c| &c.on_ready, JsValue::UNDEFINED);
}

/// Reports the label of the object in front of the visitor, or `None`.
pub fn target(label: Option<&str>) {
    call(|c| &c.on_target, label.map_or(JsValue::NULL, JsValue::from_str));
}

pub fn toast(message: &str) {
    call(|c| &c.on_toast, JsValue::from_str(message));
}

/// A footfall: `loudness` around 1 at walking pace, and whether the foot
/// landed on the rug (which muffles it) rather than bare wood.
pub fn step(loudness: f32, on_rug: bool) {
    CALLBACKS.with(|slot| {
        if let Some(callbacks) = slot.borrow().as_ref()
            && let Err(error) = callbacks.on_step.call2(&JsValue::NULL, &JsValue::from_f64(loudness.into()), &JsValue::from_bool(on_rug))
        {
            error!("overlay callback failed: {error:?}");
        }
    });
}

pub fn fail(message: &str) {
    call(|c| &c.on_error, JsValue::from_str(message));
}

#[wasm_bindgen]
pub fn set_active(active: bool) {
    push(Command::Active(active));
}

/// Starts or stops walking in a direction: "forward", "backward", "left" or "right".
#[wasm_bindgen]
pub fn set_move(direction: &str, pressed: bool) {
    if let Some(direction) = Move::parse(direction) {
        push(Command::Move(direction, pressed));
    }
}

/// Walks a little faster while `hurrying` (the page maps this to Shift).
#[wasm_bindgen]
pub fn set_hurry(hurrying: bool) {
    push(Command::Hurry(hurrying));
}

#[wasm_bindgen]
pub fn clear_moves() {
    push(Command::ClearMoves);
}

/// Turns the view by the given angles in radians.
#[wasm_bindgen]
pub fn look(yaw: f32, pitch: f32) {
    push(Command::Look { yaw, pitch });
}

#[wasm_bindgen]
pub fn interact() {
    push(Command::Interact);
}

#[wasm_bindgen]
pub fn reset_view() {
    push(Command::Reset);
}
