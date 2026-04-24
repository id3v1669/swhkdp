use crate::config::{KeyAction, MacroDef, MacroStep, MacroType, MovePath, MoveType};
use evdev::uinput::VirtualDevice;
use evdev::{InputEvent, KeyCode, RelativeAxisCode};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

const STEP_INTERVAL_MS: u64 = 8;

pub fn interpolate_direct(
    total_x: i32,
    total_y: i32,
    n: usize,
    speed: MoveType,
) -> Vec<(i32, i32)> {
    if n == 0 {
        return vec![];
    }
    let mut result = Vec::with_capacity(n);
    let mut acc_x = 0i64;
    let mut acc_y = 0i64;
    let nf = n as f64;
    for i in 1..=n {
        let t = i as f64 / nf;
        let frac = match speed {
            MoveType::Constant => t,
        };
        let pos_x = (total_x as f64 * frac).round() as i64;
        let pos_y = (total_y as f64 * frac).round() as i64;
        result.push(((pos_x - acc_x) as i32, (pos_y - acc_y) as i32));
        acc_x = pos_x;
        acc_y = pos_y;
    }
    result
}

fn key_press_event(key: KeyCode) -> InputEvent {
    InputEvent::new(evdev::EventType::KEY.0, key.0, 1)
}

fn key_release_event(key: KeyCode) -> InputEvent {
    InputEvent::new(evdev::EventType::KEY.0, key.0, 0)
}

fn rel_event(axis: RelativeAxisCode, value: i32) -> InputEvent {
    InputEvent::new(evdev::EventType::RELATIVE.0, axis.0, value)
}

async fn emit(uinput: &Mutex<VirtualDevice>, events: &[InputEvent]) {
    let mut dev = uinput.lock().await;
    let _ = dev.emit(events);
}

async fn execute_key_action(
    key: KeyCode,
    action: KeyAction,
    uinput: &Mutex<VirtualDevice>,
    pressed: &mut Vec<KeyCode>,
) {
    match action {
        KeyAction::Down => {
            emit(uinput, &[key_press_event(key)]).await;
            if !pressed.contains(&key) {
                pressed.push(key);
            }
        }
        KeyAction::Up => {
            if !pressed.contains(&key) {
                log::warn!("macro 'up' on key {:?} that is not down — skipping", key);
                return;
            }
            emit(uinput, &[key_release_event(key)]).await;
            pressed.retain(|&k| k != key);
        }
        KeyAction::Click => {
            if pressed.contains(&key) {
                log::warn!("macro 'click' on key {:?} that is already down — skipping", key);
                return;
            }
            emit(uinput, &[key_press_event(key), key_release_event(key)]).await;
        }
    }
}

async fn release_all_pressed(pressed: &[KeyCode], uinput: &Mutex<VirtualDevice>) {
    if pressed.is_empty() {
        return;
    }
    let events: Vec<InputEvent> = pressed.iter().map(|&k| key_release_event(k)).collect();
    emit(uinput, &events).await;
}

async fn execute_move(
    x: i32,
    y: i32,
    duration: u32,
    move_type: MoveType,
    path: &MovePath,
    uinput: &Mutex<VirtualDevice>,
    stop: &AtomicBool,
) {
    let n_steps = ((duration as u64) / STEP_INTERVAL_MS).max(1) as usize;
    let deltas = match path {
        MovePath::Direct => interpolate_direct(x, y, n_steps, move_type),
    };
    for (dx, dy) in deltas {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let mut events = vec![];
        if dx != 0 {
            events.push(rel_event(RelativeAxisCode::REL_X, dx));
        }
        if dy != 0 {
            events.push(rel_event(RelativeAxisCode::REL_Y, dy));
        }
        if !events.is_empty() {
            emit(uinput, &events).await;
        }
        sleep(Duration::from_millis(STEP_INTERVAL_MS)).await;
    }
}

fn execute_steps<'a>(
    steps: &'a [MacroStep],
    uinput: &'a Mutex<VirtualDevice>,
    stop: &'a AtomicBool,
    pressed: &'a mut Vec<KeyCode>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        for step in steps {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            match step {
                MacroStep::KeyAction { key, action } => {
                    execute_key_action(*key, *action, uinput, pressed).await;
                }
                MacroStep::Move { x, y, duration, move_type, path } => {
                    execute_move(*x, *y, *duration, *move_type, path, uinput, stop).await;
                }
                MacroStep::Repeat { count, steps: inner } => {
                    for _ in 0..*count {
                        if stop.load(Ordering::Relaxed) {
                            return;
                        }
                        execute_steps(inner, uinput, stop, pressed).await;
                    }
                }
            }
        }
    })
}

pub async fn run_macro(
    macro_def: MacroDef,
    uinput: Arc<Mutex<VirtualDevice>>,
    stop: Arc<AtomicBool>,
) {
    let mut pressed: Vec<KeyCode> = vec![];
    match macro_def.macro_type {
        MacroType::Simple => {
            execute_steps(&macro_def.steps, &uinput, &stop, &mut pressed).await;
        }
        MacroType::Endless | MacroType::Hold => loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            execute_steps(&macro_def.steps, &uinput, &stop, &mut pressed).await;
        },
    }
    release_all_pressed(&pressed, &uinput).await;
}
