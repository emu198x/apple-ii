//! Encoded-key input; the real keyboard latch acknowledges every queued byte.
use emu198x_shell::{InputEvent, KeyTiming, KeyboardTarget, MachineError};
use std::collections::VecDeque;
#[derive(Default)]
pub(crate) struct Input {
    pub queue: VecDeque<u8>,
    ctrl: bool,
}
fn code(name: &str) -> Option<u8> {
    match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => Some(13),
        "escape" | "esc" => Some(27),
        "backspace" | "left" => Some(8),
        "right" => Some(21),
        "space" => Some(32),
        s if s.len() == 1 && s.as_bytes()[0].is_ascii_graphic() => {
            Some(s.as_bytes()[0].to_ascii_uppercase())
        }
        _ => None,
    }
}
pub(crate) fn apply_input_event(input: &mut Input, event: &InputEvent) -> Result<(), MachineError> {
    let InputEvent::Key { name, pressed } = event else {
        return Err(MachineError::InvalidRequest {
            reason: "only keyboard input is supported".into(),
        });
    };
    if name.eq_ignore_ascii_case("ctrl") {
        input.ctrl = *pressed;
        return Ok(());
    }
    let mut byte = code(name).ok_or_else(|| MachineError::InvalidRequest {
        reason: format!("unknown Apple II key {name}"),
    })?;
    if !pressed {
        return Ok(());
    }
    if input.ctrl {
        byte &= 31;
    }
    if input.queue.len() >= 256 {
        return Err(MachineError::InvalidRequest {
            reason: "keyboard queue is full (256 bytes); wait for the ROM to consume input".into(),
        });
    }
    input.queue.push_back(byte);
    Ok(())
}
pub(crate) struct Keyboard;
pub(crate) static KEYBOARD: Keyboard = Keyboard;
impl KeyboardTarget for Keyboard {
    fn key_name_is_valid(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("ctrl") || code(name).is_some()
    }
    fn key_names_hint(&self) -> &'static str {
        "ASCII keys, Enter, Escape, Backspace, Left, Right, Ctrl"
    }
    fn keys_for_char(&self, ch: char) -> Option<Vec<String>> {
        match ch {
            '\n' | '\r' => Some(vec!["enter".into()]),
            '\x1b' => Some(vec!["escape".into()]),
            '\x08' => Some(vec!["backspace".into()]),
            '\x01'..='\x1a' => Some(vec![
                "ctrl".into(),
                char::from(ch as u8 + b'a' - 1).to_string(),
            ]),
            ' ' => Some(vec!["space".into()]),
            '!'..='~' => Some(vec![ch.to_string()]),
            _ => None,
        }
    }
    fn key_timing(&self) -> KeyTiming {
        KeyTiming {
            default_hold_frames: 1,
            max_hold_frames: 600,
            press_settle_frames: 1,
            inter_key_settle_frames: 1,
            repeat_settle_frames: 0,
            default_type_settle_frames: 2,
        }
    }
}
