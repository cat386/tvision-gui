//! Перевод клавиатурных событий winit в события Turbo Vision.
//!
//! Правила те же, что у консольного бэкенда Windows в tvision-rs, чтобы
//! программы вели себя одинаково в терминале и в окне:
//!
//! * печатный символ без Ctrl/Alt → `Key::Char(c)`, Shift уже учтён в самом
//!   символе (поэтому флаг Shift снимается);
//! * Ctrl/Alt + буква или цифра → `Key::Char` с **латинской** буквой по
//!   физической клавише, чтобы Alt+X и Ctrl+K работали и в русской раскладке;
//! * служебные клавиши (стрелки, F1..F12, Ins/Del ...) — с модификаторами;
//! * AltGr (правый Alt на европейских раскладках) с печатным символом —
//!   просто символ.
//!
//! На macOS клавиша Option работает как Alt (Option+X = Alt+X), а Cmd —
//! тоже как Alt, потому что клавиши Alt в привычном месте там нет.

use tvision::{Key, KeyEvent, Mods};
use winit::event::{ElementState, KeyEvent as WinitKey};
use winit::keyboard::{Key as Logical, KeyCode, ModifiersState, NamedKey, PhysicalKey};

/// Модификаторы winit → модификаторы Turbo Vision.
pub fn mods(m: ModifiersState) -> Mods {
    Mods { shift: m.shift_key(), ctrl: m.control_key(), alt: m.alt_key() || (cfg!(target_os = "macos") && m.super_key()) }
}

/// Переводит нажатие клавиши в ноль, одно или несколько событий
/// (несколько — когда метод ввода выдаёт сразу строку символов).
pub fn translate(ev: &WinitKey, state: ModifiersState) -> Vec<KeyEvent> {
    if ev.state != ElementState::Pressed {
        return Vec::new();
    }
    let mut m = mods(state);

    // Служебные клавиши.
    if let Logical::Named(named) = &ev.logical_key {
        let key = match named {
            NamedKey::Enter => Some(Key::Enter),
            NamedKey::Escape => Some(Key::Esc),
            NamedKey::Tab => Some(Key::Tab),
            NamedKey::Backspace => Some(Key::Backspace),
            NamedKey::ArrowUp => Some(Key::Up),
            NamedKey::ArrowDown => Some(Key::Down),
            NamedKey::ArrowLeft => Some(Key::Left),
            NamedKey::ArrowRight => Some(Key::Right),
            NamedKey::Home => Some(Key::Home),
            NamedKey::End => Some(Key::End),
            NamedKey::PageUp => Some(Key::PgUp),
            NamedKey::PageDown => Some(Key::PgDn),
            NamedKey::Insert => Some(Key::Ins),
            NamedKey::Delete => Some(Key::Del),
            NamedKey::F1 => Some(Key::F(1)),
            NamedKey::F2 => Some(Key::F(2)),
            NamedKey::F3 => Some(Key::F(3)),
            NamedKey::F4 => Some(Key::F(4)),
            NamedKey::F5 => Some(Key::F(5)),
            NamedKey::F6 => Some(Key::F(6)),
            NamedKey::F7 => Some(Key::F(7)),
            NamedKey::F8 => Some(Key::F(8)),
            NamedKey::F9 => Some(Key::F(9)),
            NamedKey::F10 => Some(Key::F(10)),
            NamedKey::F11 => Some(Key::F(11)),
            NamedKey::F12 => Some(Key::F(12)),
            // Пробел обрабатывается ниже вместе с остальными символами.
            NamedKey::Space => None,
            // Сами модификаторы, CapsLock и прочее событий не дают.
            _ => return Vec::new(),
        };
        if let Some(k) = key {
            return vec![KeyEvent::new(k, m)];
        }
    }

    // AltGr: в Windows приходит как Ctrl+Alt. Если при этом получился
    // печатный символ (например, «@» на немецкой раскладке) — это просто символ.
    let printable = ev.text.as_deref().and_then(|t| t.chars().next()).filter(|&c| !c.is_control());
    if m.ctrl && m.alt && printable.is_some() && !is_letter_or_digit_key(ev.physical_key) {
        m.ctrl = false;
        m.alt = false;
    }

    if m.ctrl || m.alt {
        // Сочетание: берём латинский символ по физической клавише.
        let c = physical_char(ev.physical_key).or_else(|| match &ev.logical_key {
            Logical::Character(s) => s.chars().next().map(|c| c.to_ascii_lowercase()),
            _ => None,
        });
        return match c {
            Some(c) => vec![KeyEvent::new(Key::Char(c), m)],
            None => Vec::new(),
        };
    }

    // Обычный ввод текста. Shift уже учтён в самом символе.
    m.shift = false;
    match ev.text.as_deref() {
        Some(t) => t.chars().filter(|c| !c.is_control()).map(|c| KeyEvent::new(Key::Char(c), m)).collect(),
        None => match &ev.logical_key {
            Logical::Named(NamedKey::Space) => vec![KeyEvent::new(Key::Char(' '), m)],
            _ => Vec::new(),
        },
    }
}

/// Буква или цифра основной части клавиатуры.
fn is_letter_or_digit_key(p: PhysicalKey) -> bool {
    matches!(physical_char(p), Some(c) if c.is_ascii_alphanumeric())
}

/// Латинский символ, написанный на физической клавише (раскладка США).
fn physical_char(p: PhysicalKey) -> Option<char> {
    let code = match p {
        PhysicalKey::Code(c) => c,
        PhysicalKey::Unidentified(_) => return None,
    };
    use KeyCode::*;
    let c = match code {
        KeyA => 'a',
        KeyB => 'b',
        KeyC => 'c',
        KeyD => 'd',
        KeyE => 'e',
        KeyF => 'f',
        KeyG => 'g',
        KeyH => 'h',
        KeyI => 'i',
        KeyJ => 'j',
        KeyK => 'k',
        KeyL => 'l',
        KeyM => 'm',
        KeyN => 'n',
        KeyO => 'o',
        KeyP => 'p',
        KeyQ => 'q',
        KeyR => 'r',
        KeyS => 's',
        KeyT => 't',
        KeyU => 'u',
        KeyV => 'v',
        KeyW => 'w',
        KeyX => 'x',
        KeyY => 'y',
        KeyZ => 'z',
        Digit0 => '0',
        Digit1 => '1',
        Digit2 => '2',
        Digit3 => '3',
        Digit4 => '4',
        Digit5 => '5',
        Digit6 => '6',
        Digit7 => '7',
        Digit8 => '8',
        Digit9 => '9',
        Space => ' ',
        Minus => '-',
        Equal => '=',
        BracketLeft => '[',
        BracketRight => ']',
        Backslash => '\\',
        Semicolon => ';',
        Quote => '\'',
        Backquote => '`',
        Comma => ',',
        Period => '.',
        Slash => '/',
        _ => return None,
    };
    Some(c)
}

/// Код физической клавиши (для «горячих клавиш» самого окна).
pub fn code(p: PhysicalKey) -> Option<KeyCode> {
    match p {
        PhysicalKey::Code(c) => Some(c),
        PhysicalKey::Unidentified(_) => None,
    }
}
