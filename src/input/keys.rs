//! Keys, modifiers, and chord parsing for the device loop.
//!
//! Bridges three representations:
//! - evdev [`KeyCode`]s (what the kernel emits / we emit via uinput),
//! - the schema's textual key chords (`"h"`, `"super + 1"`, `"shift + 0"`,
//!   `"XF86AudioRaiseVolume"`), parsed into [`Chord`],
//! - tracked modifier state ([`Mods`]).
//!
//! CapsLock is deliberately *not* a [`Mods`] modifier: it is the mode trigger,
//! consumed by [`super::taphold`], so a binding like `"h"` (no modifiers) matches
//! while CapsLock is physically held.

use evdev::KeyCode;

/// A tracked modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    Super,
    Shift,
    Ctrl,
    Alt,
}

/// The set of modifiers currently held (used for exact chord matching).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub sup: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Mods {
    /// Set a modifier's held state.
    pub fn set(&mut self, m: Modifier, on: bool) {
        match m {
            Modifier::Super => self.sup = on,
            Modifier::Shift => self.shift = on,
            Modifier::Ctrl => self.ctrl = on,
            Modifier::Alt => self.alt = on,
        }
    }

    /// True when exactly one of `m` is the only modifier held.
    pub fn only(&self, m: Modifier) -> bool {
        *self == {
            let mut x = Mods::default();
            x.set(m, true);
            x
        }
    }

    /// True when no modifiers are held.
    pub fn none(&self) -> bool {
        *self == Mods::default()
    }
}

/// A parsed key chord: a set of modifiers plus a base keycode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub mods: Mods,
    pub code: u16,
}

/// Which modifier a keycode is, if any (both left and right variants).
pub fn modifier_of(code: KeyCode) -> Option<Modifier> {
    match code {
        KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => Some(Modifier::Super),
        KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => Some(Modifier::Shift),
        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => Some(Modifier::Ctrl),
        KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => Some(Modifier::Alt),
        _ => None,
    }
}

/// The left-hand keycode to emit for a modifier (for synthesized chords).
pub fn modifier_code(m: Modifier) -> KeyCode {
    match m {
        Modifier::Super => KeyCode::KEY_LEFTMETA,
        Modifier::Shift => KeyCode::KEY_LEFTSHIFT,
        Modifier::Ctrl => KeyCode::KEY_LEFTCTRL,
        Modifier::Alt => KeyCode::KEY_LEFTALT,
    }
}

/// Is this the CapsLock key (the mode trigger)?
pub fn is_capslock(code: KeyCode) -> bool {
    code == KeyCode::KEY_CAPSLOCK
}

/// Map a modifier token from a chord string to a [`Modifier`].
fn modifier_token(tok: &str) -> Option<Modifier> {
    match tok {
        "super" | "mod" | "meta" | "win" | "cmd" => Some(Modifier::Super),
        "shift" => Some(Modifier::Shift),
        "ctrl" | "control" => Some(Modifier::Ctrl),
        "alt" => Some(Modifier::Alt),
        _ => None,
    }
}

/// Map a base-key name (Hyprland/evdev style) to a [`KeyCode`].
pub fn key_name_to_code(name: &str) -> Option<KeyCode> {
    use KeyCode as K;
    let n = name.trim().to_ascii_lowercase();
    let code = match n.as_str() {
        "a" => K::KEY_A,
        "b" => K::KEY_B,
        "c" => K::KEY_C,
        "d" => K::KEY_D,
        "e" => K::KEY_E,
        "f" => K::KEY_F,
        "g" => K::KEY_G,
        "h" => K::KEY_H,
        "i" => K::KEY_I,
        "j" => K::KEY_J,
        "k" => K::KEY_K,
        "l" => K::KEY_L,
        "m" => K::KEY_M,
        "n" => K::KEY_N,
        "o" => K::KEY_O,
        "p" => K::KEY_P,
        "q" => K::KEY_Q,
        "r" => K::KEY_R,
        "s" => K::KEY_S,
        "t" => K::KEY_T,
        "u" => K::KEY_U,
        "v" => K::KEY_V,
        "w" => K::KEY_W,
        "x" => K::KEY_X,
        "y" => K::KEY_Y,
        "z" => K::KEY_Z,
        "0" => K::KEY_0,
        "1" => K::KEY_1,
        "2" => K::KEY_2,
        "3" => K::KEY_3,
        "4" => K::KEY_4,
        "5" => K::KEY_5,
        "6" => K::KEY_6,
        "7" => K::KEY_7,
        "8" => K::KEY_8,
        "9" => K::KEY_9,
        "left" => K::KEY_LEFT,
        "right" => K::KEY_RIGHT,
        "up" => K::KEY_UP,
        "down" => K::KEY_DOWN,
        "return" | "enter" => K::KEY_ENTER,
        "space" => K::KEY_SPACE,
        "tab" => K::KEY_TAB,
        "escape" | "esc" => K::KEY_ESC,
        "slash" => K::KEY_SLASH,
        "print" | "sysrq" => K::KEY_SYSRQ,
        "capslock" => K::KEY_CAPSLOCK,
        "f1" => K::KEY_F1,
        "f2" => K::KEY_F2,
        "f3" => K::KEY_F3,
        "f4" => K::KEY_F4,
        "f5" => K::KEY_F5,
        "f6" => K::KEY_F6,
        "f7" => K::KEY_F7,
        "f8" => K::KEY_F8,
        "f9" => K::KEY_F9,
        "f10" => K::KEY_F10,
        "f11" => K::KEY_F11,
        "f12" => K::KEY_F12,
        "f13" => K::KEY_F13,
        "f14" => K::KEY_F14,
        "f15" => K::KEY_F15,
        "f16" => K::KEY_F16,
        "f17" => K::KEY_F17,
        "f18" => K::KEY_F18,
        "f19" => K::KEY_F19,
        "f20" => K::KEY_F20,
        "f21" => K::KEY_F21,
        "f22" => K::KEY_F22,
        "f23" => K::KEY_F23,
        "f24" => K::KEY_F24,
        "xf86audioraisevolume" => K::KEY_VOLUMEUP,
        "xf86audiolowervolume" => K::KEY_VOLUMEDOWN,
        "xf86audiomute" => K::KEY_MUTE,
        "xf86audiomicmute" => K::KEY_MICMUTE,
        "xf86monbrightnessup" => K::KEY_BRIGHTNESSUP,
        "xf86monbrightnessdown" => K::KEY_BRIGHTNESSDOWN,
        "xf86kbdbrightnessup" => K::KEY_KBDILLUMUP,
        "xf86kbdbrightnessdown" => K::KEY_KBDILLUMDOWN,
        "xf86audioplay" => K::KEY_PLAYPAUSE,
        "xf86audionext" => K::KEY_NEXTSONG,
        "xf86audioprev" => K::KEY_PREVIOUSSONG,
        _ => return None,
    };
    Some(code)
}

/// Parse a chord string like `"super + 1"`, `"shift + print"`, `"h"`.
///
/// The last `+`-separated token is the base key; earlier tokens are modifiers.
/// Returns `None` if the base key is unknown or a modifier token is invalid.
pub fn parse_chord(s: &str) -> Option<Chord> {
    let parts: Vec<&str> = s
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let (base, mod_toks) = parts.split_last()?;

    let mut mods = Mods::default();
    for tok in mod_toks {
        let m = modifier_token(&tok.to_ascii_lowercase())?;
        mods.set(m, true);
    }
    let code = key_name_to_code(base)?;
    Some(Chord { mods, code: code.0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_key_has_no_mods() {
        let c = parse_chord("h").unwrap();
        assert!(c.mods.none());
        assert_eq!(c.code, KeyCode::KEY_H.0);
    }

    #[test]
    fn super_combo_parses() {
        let c = parse_chord("super + 1").unwrap();
        assert!(c.mods.only(Modifier::Super));
        assert_eq!(c.code, KeyCode::KEY_1.0);
    }

    #[test]
    fn shift_combo_and_named_keys() {
        assert_eq!(
            parse_chord("shift + 0").unwrap(),
            Chord {
                mods: Mods {
                    shift: true,
                    ..Default::default()
                },
                code: KeyCode::KEY_0.0
            }
        );
        assert_eq!(parse_chord("left").unwrap().code, KeyCode::KEY_LEFT.0);
        assert_eq!(parse_chord("return").unwrap().code, KeyCode::KEY_ENTER.0);
        assert_eq!(parse_chord("F12").unwrap().code, KeyCode::KEY_F12.0);
        assert_eq!(parse_chord("slash").unwrap().code, KeyCode::KEY_SLASH.0);
    }

    #[test]
    fn xf86_media_keys_parse() {
        assert_eq!(
            parse_chord("XF86AudioRaiseVolume").unwrap().code,
            KeyCode::KEY_VOLUMEUP.0
        );
        assert_eq!(
            parse_chord("XF86MonBrightnessDown").unwrap().code,
            KeyCode::KEY_BRIGHTNESSDOWN.0
        );
    }

    #[test]
    fn unknown_key_is_none() {
        assert!(parse_chord("nonexistent").is_none());
        assert!(parse_chord("super + nope").is_none());
    }

    #[test]
    fn modifiers_classify() {
        assert_eq!(modifier_of(KeyCode::KEY_LEFTMETA), Some(Modifier::Super));
        assert_eq!(modifier_of(KeyCode::KEY_RIGHTCTRL), Some(Modifier::Ctrl));
        assert_eq!(modifier_of(KeyCode::KEY_H), None);
        assert!(is_capslock(KeyCode::KEY_CAPSLOCK));
        assert!(!is_capslock(KeyCode::KEY_A));
    }

    #[test]
    fn mods_set_and_query() {
        let mut m = Mods::default();
        assert!(m.none());
        m.set(Modifier::Super, true);
        assert!(m.only(Modifier::Super));
        m.set(Modifier::Shift, true);
        assert!(!m.only(Modifier::Super));
        m.set(Modifier::Super, false);
        assert!(m.only(Modifier::Shift));
    }
}
