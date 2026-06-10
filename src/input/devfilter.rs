//! Device-grab scope — which evdev devices the engine should OWN.
//!
//! The engine grabs keyboards exclusively (`EVIOCGRAB`) and re-emits through one
//! virtual device, so grabbing the WRONG device is harmful: a security key
//! (YubiKey) typing an OTP, or an audio mixer's transport buttons, must never be
//! intercepted. The original filter — "advertises `KEY_A`" — grabbed all of
//! those alongside the real keyboard.
//!
//! [`DeviceFilter::is_text_keyboard`] narrows the set to real text keyboards. It
//! combines the libinput-style capability *definition* of a keyboard (the typing
//! block + Enter + a modifier — compiled, since it is a cited definition rather
//! than user policy) with an EXCLUDE-ONLY policy layer (by vendor and by name
//! substring) loaded as data from the schema. Exclude-only is the lockout-safe
//! direction: the predicate can only ever REMOVE non-keyboards; it never relies
//! on an allow-list of "good" devices that a wrong guess could empty.
//!
//! Host note (verified 2026-06): a YubiKey advertises `EV_REP` and LEDs
//! *identically* to a real keyboard, so autorepeat/LED capabilities cannot tell
//! them apart — only the vendor (Yubico `0x1050`) and the name reliably do.
//! Hence vendor/name excludes, not a capability heuristic, are the security-key
//! discriminator; the capability floor is a secondary signal (a YubiKey's modhex
//! key set lacks the home-row letters, an audio device has no alpha keys).

use super::device::VIRTUAL_NAME;
use evdev::KeyCode;

/// Yubico USB vendor id. Security keys type their OTP as a HID keyboard, so they
/// pass any capability heuristic; only the vendor reliably identifies them.
const YUBICO_VENDOR: u16 = 0x1050;

/// Home-row letters every real text keyboard carries. A YubiKey (modhex uses
/// only `c b d e f g h i j k l n r t u v`) lacks `A` and `S`; an audio device has
/// no alpha keys at all — so requiring these is a cheap secondary keyboard
/// signal (defence-in-depth behind the vendor/name excludes).
const HOME_ROW: [KeyCode; 4] = [
    KeyCode::KEY_A,
    KeyCode::KEY_S,
    KeyCode::KEY_D,
    KeyCode::KEY_F,
];

/// Any one of these marks a device that can compose chords like a keyboard.
const MODIFIERS: [KeyCode; 6] = [
    KeyCode::KEY_LEFTSHIFT,
    KeyCode::KEY_RIGHTSHIFT,
    KeyCode::KEY_LEFTCTRL,
    KeyCode::KEY_RIGHTCTRL,
    KeyCode::KEY_LEFTALT,
    KeyCode::KEY_LEFTMETA,
];

/// Policy for which devices the engine may grab: an exclude-only list plus the
/// compiled capability floor. Built from [`DeviceFilter::default`] (a safe
/// baseline) merged with any schema-provided excludes.
#[derive(Debug, Clone)]
pub struct DeviceFilter {
    /// USB vendor ids never grabbed (security keys etc.).
    pub exclude_vendors: Vec<u16>,
    /// Device-name substrings never grabbed (audio HID, consumer-control nodes).
    pub exclude_name_substrings: Vec<String>,
}

impl Default for DeviceFilter {
    fn default() -> Self {
        Self {
            exclude_vendors: vec![YUBICO_VENDOR],
            exclude_name_substrings: [
                "Yubico",
                "YubiKey", // security key (also vendor-excluded — defence in depth)
                "MV-SILICON",
                "Maonocaster", // audio mixer
                "Consumer Control",
                "System Control", // media/system sibling HID nodes (NOT bare "Control")
            ]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        }
    }
}

impl DeviceFilter {
    /// True iff `d` is a real text keyboard the engine should grab.
    pub fn is_text_keyboard(&self, d: &evdev::Device) -> bool {
        let Some(keys) = d.supported_keys() else {
            return false; // no EV_KEY at all → not a keyboard
        };
        self.passes(d.name(), d.input_id().vendor(), |k| keys.contains(k))
    }

    /// The pure decision (no evdev `Device`), so it is unit-testable without
    /// root or `/dev` access. `has(k)` reports whether the device advertises `k`.
    fn passes(&self, name: Option<&str>, vendor: u16, has: impl Fn(KeyCode) -> bool) -> bool {
        if name == Some(VIRTUAL_NAME) {
            return false; // never grab our own re-emit device
        }
        if self.exclude_vendors.contains(&vendor) {
            return false;
        }
        if let Some(n) = name
            && self
                .exclude_name_substrings
                .iter()
                .any(|s| n.contains(s.as_str()))
        {
            return false;
        }
        capability_is_keyboard(&has)
    }
}

/// The compiled capability floor (the libinput-style *definition* of a keyboard,
/// not user policy): a typing device carries Enter + at least one modifier + the
/// home-row letters. `EV_REP` and a full `A..Z` block are deliberately NOT
/// required — some real keyboards (and the VM's virtual keyboard) omit them, and
/// a YubiKey advertises `EV_REP` anyway, so neither is a reliable signal.
fn capability_is_keyboard(has: &impl Fn(KeyCode) -> bool) -> bool {
    has(KeyCode::KEY_ENTER) && MODIFIERS.iter().any(|m| has(*m)) && HOME_ROW.iter().all(|k| has(*k))
}

/// Choose which candidates to grab from a "is this a text keyboard" flag per
/// candidate, returning the kept indices and whether the FAIL-SAFE widened the
/// set. The strict filter may only NARROW a non-empty candidate set; if it would
/// leave nothing (a misconfigured/over-aggressive filter), keep ALL candidates
/// instead — excluding the user's only keyboard is a total lockout. Pure, so the
/// lockout invariant is unit-tested without real devices.
pub fn selection(is_keyboard: &[bool]) -> (Vec<usize>, bool) {
    let kept: Vec<usize> = is_keyboard
        .iter()
        .enumerate()
        .filter_map(|(i, &k)| k.then_some(i))
        .collect();
    if kept.is_empty() && !is_keyboard.is_empty() {
        ((0..is_keyboard.len()).collect(), true) // fail-safe: widen to all
    } else {
        (kept, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evdev::KeyCode as K;

    /// A real keyboard's key set — enough to pass the floor.
    fn real_kbd() -> Vec<K> {
        vec![
            K::KEY_ENTER,
            K::KEY_LEFTSHIFT,
            K::KEY_A,
            K::KEY_S,
            K::KEY_D,
            K::KEY_F,
            K::KEY_SPACE,
        ]
    }

    /// YubiKey modhex set (no `A`, no `S`) + the Enter it appends.
    fn yubikey_keys() -> Vec<K> {
        vec![
            K::KEY_ENTER,
            K::KEY_C,
            K::KEY_B,
            K::KEY_D,
            K::KEY_E,
            K::KEY_F,
            K::KEY_LEFTSHIFT,
        ]
    }

    fn has(set: Vec<K>) -> impl Fn(K) -> bool {
        move |k| set.contains(&k)
    }

    #[test]
    fn real_keyboard_passes() {
        let f = DeviceFilter::default();
        assert!(f.passes(Some("Keychron K2 HE Keyboard"), 0x3434, has(real_kbd())));
    }

    #[test]
    fn yubikey_excluded_by_vendor() {
        let f = DeviceFilter::default();
        // Even if it advertised a full keyboard key set, the Yubico vendor is excluded.
        assert!(
            !f.passes(
                Some("Yubico YubiKey OTP+FIDO+CCID"),
                0x1050,
                has(real_kbd())
            ),
            "Yubico vendor must be excluded"
        );
    }

    #[test]
    fn yubikey_excluded_by_capability_even_without_vendor_policy() {
        // Defence in depth: clear vendor/name policy; the modhex set (no A/S)
        // still fails the keyboard capability floor.
        let f = DeviceFilter {
            exclude_vendors: vec![],
            exclude_name_substrings: vec![],
        };
        assert!(!f.passes(Some("some OTP token"), 0x9999, has(yubikey_keys())));
    }

    #[test]
    fn audio_mixer_excluded_by_name() {
        let f = DeviceFilter::default();
        assert!(!f.passes(Some("MV-SILICON Maonocaster E2"), 0x1235, has(real_kbd())));
    }

    #[test]
    fn never_grabs_our_own_virtual_device() {
        let f = DeviceFilter::default();
        assert!(!f.passes(Some(VIRTUAL_NAME), 0x0000, has(real_kbd())));
    }

    #[test]
    fn bare_control_substring_does_not_false_exclude() {
        // A keyboard whose name merely contains "Control" (not the two-word HID
        // node names) must NOT be excluded — bare "Control" is not a default.
        let f = DeviceFilter::default();
        assert!(f.passes(Some("Acme Control Deck Keyboard"), 0x1234, has(real_kbd())));
    }

    #[test]
    fn no_supported_keys_is_not_a_keyboard() {
        let f = DeviceFilter::default();
        assert!(!f.passes(Some("Power Button"), 0x0000, |_| false));
    }

    #[test]
    fn selection_narrows_to_keyboards() {
        assert_eq!(selection(&[true, false, false]), (vec![0], false));
        assert_eq!(selection(&[false, true, true]), (vec![1, 2], false));
    }

    #[test]
    fn selection_failsafe_widens_when_none_match() {
        // No candidate looks like a keyboard → keep them ALL (never lock out).
        assert_eq!(selection(&[false, false]), (vec![0, 1], true));
    }

    #[test]
    fn selection_empty_input_is_empty() {
        assert_eq!(selection(&[]), (Vec::<usize>::new(), false));
    }
}
