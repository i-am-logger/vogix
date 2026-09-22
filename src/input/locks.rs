//! Keyboard lock state (Caps, Num and Scroll Lock), published for the
//! desktop shell.
//!
//! The compositor owns the xkb lock state and lights it on every keyboard's
//! LEDs: Hyprland re-syncs all keyboards' LEDs after each key and modifier
//! change. A LED write to a keyboard this engine has grabbed still takes
//! effect, because every evdev client of a device writes through the device's
//! one input handle and that handle is the grab holder, and the resulting
//! EV_LED event is delivered to the grabbing client: this engine. The grabbed
//! keyboards' LED events are therefore a push source for the lock state.
//!
//! [`Leds`] is one keyboard's lock LEDs, [`fold`] merges the grabbed keyboards
//! into the published [`Locks`], and [`Publisher`] writes
//! `~/.local/state/vogix/input-locks.json` from its own thread, so the
//! engine's poll loop never waits on the filesystem. The document exists only
//! while an engine runs: the publisher removes it when the engine stops.

use crate::config::Config;
use evdev::{AttributeSet, AttributeSetRef, LedCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;

/// A lock whose state a keyboard LED shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lock {
    Caps,
    Num,
    Scroll,
}

impl Lock {
    const ALL: [Lock; 3] = [Lock::Caps, Lock::Num, Lock::Scroll];

    fn led(self) -> LedCode {
        match self {
            Lock::Caps => LedCode::LED_CAPSL,
            Lock::Num => LedCode::LED_NUML,
            Lock::Scroll => LedCode::LED_SCROLLL,
        }
    }

    fn of_led(code: LedCode) -> Option<Lock> {
        Lock::ALL.into_iter().find(|lock| lock.led() == code)
    }

    fn slot(self) -> usize {
        self as usize
    }
}

/// One keyboard's lock LEDs: which it has, and which are lit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Leds {
    has: [bool; 3],
    lit: [bool; 3],
}

impl Leds {
    /// From the LEDs a keyboard declares and the ones currently lit.
    pub fn from_sets(
        declared: Option<&AttributeSetRef<LedCode>>,
        lit: &AttributeSetRef<LedCode>,
    ) -> Self {
        let mut leds = Self::default();
        for lock in Lock::ALL {
            leds.has[lock.slot()] = declared.is_some_and(|set| set.contains(lock.led()));
            leds.lit[lock.slot()] = leds.has[lock.slot()] && lit.contains(lock.led());
        }
        leds
    }

    /// Read a keyboard's LEDs: the set it declares, and the lit ones as the
    /// kernel holds them (EVIOCGLED, an in-memory read).
    pub fn read(dev: &evdev::Device) -> Self {
        let lit = dev.get_led_state().unwrap_or_else(|e| {
            log::warn!(
                "vogix input: cannot read the LEDs of {:?}: {e}; taking them as off",
                dev.name().unwrap_or("?")
            );
            AttributeSet::new()
        });
        Self::from_sets(dev.supported_leds(), &lit)
    }

    /// Apply one EV_LED event from this keyboard. LEDs that show no lock
    /// (compose, kana, …) are ignored.
    pub fn apply(&mut self, code: u16, value: i32) {
        if let Some(lock) = Lock::of_led(LedCode(code)) {
            self.has[lock.slot()] = true;
            self.lit[lock.slot()] = value != 0;
        }
    }
}

/// The published lock state. `None` means no grabbed keyboard has that LED,
/// so the state cannot be observed; it is not the same as off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Locks {
    pub caps_lock: Option<bool>,
    pub num_lock: Option<bool>,
    pub scroll_lock: Option<bool>,
}

/// Merge the grabbed keyboards' LEDs: a lock is known when any keyboard has
/// its LED, and on when any keyboard has it lit. The compositor writes the
/// same state to every keyboard; a keyboard grabbed since the last write
/// carries its own fresh state until the next key, which the OR outvotes.
pub fn fold<'a>(keyboards: impl IntoIterator<Item = &'a Leds>) -> Locks {
    let mut merged = Leds::default();
    for leds in keyboards {
        for lock in Lock::ALL {
            merged.has[lock.slot()] |= leds.has[lock.slot()];
            merged.lit[lock.slot()] |= leds.lit[lock.slot()];
        }
    }
    let state = |lock: Lock| merged.has[lock.slot()].then_some(merged.lit[lock.slot()]);
    Locks {
        caps_lock: state(Lock::Caps),
        num_lock: state(Lock::Num),
        scroll_lock: state(Lock::Scroll),
    }
}

/// Where the lock document lives (next to `current-mode` and
/// `input-health.json`).
pub fn document_path() -> PathBuf {
    Config::state_dir().join("input-locks.json")
}

/// What the writer thread does next.
#[derive(Debug)]
enum Update {
    /// Write this state.
    Publish(Locks),
    /// Remove the document: nothing tracks the locks any more.
    Retract,
}

/// Writes the lock document off the engine's poll loop. [`Publisher::publish`]
/// only queues; dropping the publisher retracts the document and waits for the
/// writer to finish, on every exit path of the engine.
pub struct Publisher {
    tx: Option<mpsc::Sender<Update>>,
    worker: Option<JoinHandle<()>>,
    last: Option<Locks>,
}

impl Publisher {
    /// Start the writer thread for `path`. When the thread cannot be started
    /// the publisher stays inert: lock state is a display aid and never a
    /// reason to refuse input.
    pub fn spawn(path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("vogix-locks".into())
            .spawn(move || serve(&rx, &path))
        {
            Ok(worker) => Self {
                tx: Some(tx),
                worker: Some(worker),
                last: None,
            },
            Err(e) => {
                log::warn!("vogix input: lock-state publisher not started: {e}");
                Self {
                    tx: None,
                    worker: None,
                    last: None,
                }
            }
        }
    }

    /// Queue `locks` for writing unless it is the state last queued. Never
    /// blocks.
    pub fn publish(&mut self, locks: Locks) {
        if self.last == Some(locks) {
            return;
        }
        self.last = Some(locks);
        if let Some(tx) = &self.tx {
            let _ = tx.send(Update::Publish(locks));
        }
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Update::Retract);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The writer: apply updates until the channel closes. A backlog collapses to
/// its newest entry, since only the latest state matters.
fn serve(rx: &mpsc::Receiver<Update>, path: &Path) {
    while let Ok(mut update) = rx.recv() {
        while let Ok(newer) = rx.try_recv() {
            update = newer;
        }
        let result = match update {
            Update::Publish(locks) => write_document(path, &locks),
            Update::Retract => remove_document(path),
        };
        if let Err(e) = result {
            log::warn!("vogix input: lock state at {}: {e}", path.display());
        }
    }
}

/// Replace the document atomically (tmp + rename): a reader never sees a
/// partial file, and a watcher on the path sees one change.
fn write_document(path: &Path, locks: &Locks) -> std::io::Result<()> {
    let json = serde_json::to_string(locks).map_err(std::io::Error::other)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

fn remove_document(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(codes: &[LedCode]) -> AttributeSet<LedCode> {
        let mut s = AttributeSet::new();
        for c in codes {
            s.insert(*c);
        }
        s
    }

    fn leds(declared: &[LedCode], lit: &[LedCode]) -> Leds {
        Leds::from_sets(Some(&set(declared)), &set(lit))
    }

    #[test]
    fn the_document_is_the_shape_the_shell_reads() {
        let locks = Locks {
            caps_lock: Some(true),
            num_lock: Some(false),
            scroll_lock: None,
        };
        assert_eq!(
            serde_json::to_value(locks).unwrap(),
            serde_json::json!({ "capsLock": true, "numLock": false, "scrollLock": null })
        );
    }

    #[test]
    fn a_lock_is_known_only_where_a_keyboard_has_its_led() {
        let kb = leds(
            &[LedCode::LED_CAPSL, LedCode::LED_NUML],
            &[LedCode::LED_CAPSL],
        );
        assert_eq!(
            fold([&kb]),
            Locks {
                caps_lock: Some(true),
                num_lock: Some(false),
                scroll_lock: None,
            }
        );
        assert_eq!(fold(std::iter::empty::<&Leds>()), Locks::default());
        assert_eq!(
            fold([&Leds::from_sets(None, &set(&[]))]),
            Locks::default(),
            "a keyboard without LEDs tells nothing"
        );
    }

    #[test]
    fn a_lit_led_the_keyboard_does_not_declare_is_not_reported() {
        let kb = leds(&[LedCode::LED_NUML], &[LedCode::LED_CAPSL]);
        assert_eq!(fold([&kb]).caps_lock, None);
    }

    #[test]
    fn led_events_update_the_keyboard() {
        let mut kb = leds(&[LedCode::LED_CAPSL], &[]);
        kb.apply(LedCode::LED_CAPSL.0, 1);
        assert_eq!(fold([&kb]).caps_lock, Some(true));
        kb.apply(LedCode::LED_CAPSL.0, 0);
        assert_eq!(fold([&kb]).caps_lock, Some(false));
        kb.apply(LedCode::LED_SCROLLL.0, 1);
        assert_eq!(fold([&kb]).scroll_lock, Some(true));
        let before = kb;
        kb.apply(LedCode::LED_COMPOSE.0, 1);
        assert_eq!(kb, before, "a LED that shows no lock changes nothing");
    }

    #[test]
    fn keyboards_merge_by_or() {
        let lit = leds(&[LedCode::LED_CAPSL], &[LedCode::LED_CAPSL]);
        let dark = leds(&[LedCode::LED_CAPSL], &[]);
        let bare = Leds::default();
        assert_eq!(fold([&dark, &lit]).caps_lock, Some(true));
        assert_eq!(fold([&dark, &bare]).caps_lock, Some(false));
    }

    #[test]
    fn the_writer_keeps_only_the_newest_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state/input-locks.json");
        let (tx, rx) = mpsc::channel();
        let on = Locks {
            caps_lock: Some(true),
            ..Locks::default()
        };
        let off = Locks {
            caps_lock: Some(false),
            ..Locks::default()
        };
        tx.send(Update::Publish(on)).unwrap();
        tx.send(Update::Publish(off)).unwrap();
        drop(tx);
        serve(&rx, &path);
        let written: Locks =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written, off);
        assert!(
            !path.with_extension("json.tmp").exists(),
            "the temporary file is renamed into place"
        );
    }

    #[test]
    fn the_writer_retracts_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input-locks.json");
        let (tx, rx) = mpsc::channel();
        tx.send(Update::Publish(Locks::default())).unwrap();
        drop(tx);
        serve(&rx, &path);
        assert!(path.exists());

        let (tx, rx) = mpsc::channel();
        tx.send(Update::Retract).unwrap();
        tx.send(Update::Retract).unwrap();
        drop(tx);
        serve(&rx, &path);
        assert!(!path.exists());
    }

    #[test]
    fn a_dropped_publisher_leaves_no_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input-locks.json");
        let mut publisher = Publisher::spawn(path.clone());
        publisher.publish(Locks {
            caps_lock: Some(true),
            ..Locks::default()
        });
        drop(publisher);
        assert!(!path.exists());
        assert!(!path.with_extension("json.tmp").exists());
    }
}
