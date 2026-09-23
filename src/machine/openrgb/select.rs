//! Which OpenRGB controllers a device spec selects, and what vogix writes to
//! them.
//!
//! A [`Selector`] matches every controller whose display name contains
//! `name_contains` (ASCII case-insensitive), and names a mode by exact
//! case-insensitive name; an unknown mode is an error that lists the modes, never
//! a fallback. [`plan`] turns a controller, a mode and a colour into the
//! writes that set it, echoing brightness, speed and direction from the
//! server's own mode description. Before a mode echo is planned it passes
//! [`check_acceptance`], the rule the server applies to `UPDATEMODE`
//! (`RGBController::SetModeValuesFromMode`): the server drops a mode that fails
//! it without applying anything and still acknowledges the packet with status
//! OK, so vogix reports the failing clause instead of sending it.

use super::model::{ColorMode, ControllerDescription, Direction, ModeDescription, ModeFlags};
use super::wire::{RgbColor, WireString};
use std::fmt;

/// Selects controllers by display name and names the mode to set on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    name_contains: String,
    mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SelectorError {
    #[error("an OpenRGB device's nameContains is empty, which would select every controller")]
    EmptyNameContains,
    #[error("an OpenRGB device's mode is empty")]
    EmptyMode,
}

impl Selector {
    pub fn new(
        name_contains: impl Into<String>,
        mode: impl Into<String>,
    ) -> Result<Self, SelectorError> {
        let name_contains = name_contains.into();
        let mode = mode.into();
        if name_contains.is_empty() {
            return Err(SelectorError::EmptyNameContains);
        }
        if mode.is_empty() {
            return Err(SelectorError::EmptyMode);
        }
        Ok(Self {
            name_contains,
            mode,
        })
    }

    pub fn name_contains(&self) -> &str {
        &self.name_contains
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// The controller's display name contains `name_contains`, ignoring ASCII
    /// case.
    pub fn matches(&self, controller: &ControllerDescription) -> bool {
        controller
            .display_name()
            .contains_ignore_ascii_case(&self.name_contains)
    }
}

/// How an apply was triggered, which decides whether an already-active mode is
/// written again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApplyKind {
    /// Skip `UPDATEMODE` when the controller already runs the mode with the
    /// planned colour mode; the colour-carrying write is always sent.
    Reconcile,
    /// Write the mode even when it is already active.
    Forced,
}

/// Which colour list a mode takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColourPath {
    /// The controller's per-LED colours, set with `UPDATELEDS`.
    PerLed,
    /// The mode's own colour list, carried in `UPDATEMODE`.
    ModeSpecific,
}

/// The writes that set one controller, and what the server should hold after
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub mode_index: usize,
    pub writes: Writes,
    pub expect: Expectation,
}

/// The packets a plan sends, in order. Every plan carries the colour in at
/// least one packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writes {
    /// `UPDATEMODE` when the mode must be switched, then `UPDATELEDS` with one
    /// colour per LED.
    PerLed {
        mode: Option<ModeDescription>,
        leds: Vec<RgbColor>,
    },
    /// `UPDATEMODE` carrying the mode's colour list.
    ModeSpecific { mode: ModeDescription },
}

impl Plan {
    pub fn path(&self) -> ColourPath {
        match self.writes {
            Writes::PerLed { .. } => ColourPath::PerLed,
            Writes::ModeSpecific { .. } => ColourPath::ModeSpecific,
        }
    }

    /// The mode block for `UPDATEMODE`, if one is sent.
    pub fn mode_write(&self) -> Option<&ModeDescription> {
        match &self.writes {
            Writes::PerLed { mode, .. } => mode.as_ref(),
            Writes::ModeSpecific { mode } => Some(mode),
        }
    }

    /// The colours for `UPDATELEDS`, if it is sent.
    pub fn led_write(&self) -> Option<&[RgbColor]> {
        match &self.writes {
            Writes::PerLed { leds, .. } => Some(leds),
            Writes::ModeSpecific { .. } => None,
        }
    }
}

/// The server state a plan produces, compared against a read-back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    pub mode_index: usize,
    pub mode_name: WireString,
    pub color_mode: ColorMode,
    pub colours: ExpectedColours,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedColours {
    /// Every one of `count` LED colours equals `colour`.
    Leds { colour: RgbColor, count: usize },
    /// The mode's colour list equals these.
    Mode(Vec<RgbColor>),
}

/// Why no writes are planned for a controller.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("no mode named \"{requested}\" (modes: {})", available.join(", "))]
    UnknownMode {
        requested: String,
        available: Vec<String>,
    },
    #[error("mode \"{mode}\" takes no settable colour (flags {flags:?}, colour mode {color_mode})")]
    NoSettableColour {
        mode: String,
        flags: ModeFlags,
        color_mode: ColorMode,
    },
    #[error("mode \"{mode}\" is per-LED but the controller has no LEDs")]
    NoLeds { mode: String },
    #[error("mode \"{mode}\" needs {count} colours, more than an UPDATEMODE packet carries")]
    ColourCountUnencodable { mode: String, count: u64 },
    #[error("OpenRGB would drop the mode \"{mode}\" update without applying it: {clause}")]
    ServerWouldReject { mode: String, clause: RejectClause },
}

/// The first clause of the server's mode acceptance rule a proposed mode
/// fails, in the order the server evaluates them. `held` is the server's
/// value, `sent` the proposed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectClause {
    Name {
        held: String,
        sent: String,
    },
    BrightnessMax {
        held: u32,
        sent: u32,
    },
    BrightnessMin {
        held: u32,
        sent: u32,
    },
    Brightness {
        value: u32,
        min: u32,
        max: u32,
    },
    ColourMode {
        color_mode: ColorMode,
        flags: ModeFlags,
    },
    ColoursMax {
        held: u32,
        sent: u32,
    },
    ColoursMin {
        held: u32,
        sent: u32,
    },
    ColourCount {
        count: usize,
        min: u32,
        max: u32,
    },
    Direction {
        direction: Direction,
        flags: ModeFlags,
    },
    SpeedMax {
        held: u32,
        sent: u32,
    },
    SpeedMin {
        held: u32,
        sent: u32,
    },
    Speed {
        value: u32,
        min: u32,
        max: u32,
    },
}

impl fmt::Display for RejectClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name { held, sent } => {
                write!(f, "name \"{sent}\" differs from the server's \"{held}\"")
            }
            Self::BrightnessMax { held, sent } => {
                write!(f, "brightness_max {sent} differs from the server's {held}")
            }
            Self::BrightnessMin { held, sent } => {
                write!(f, "brightness_min {sent} differs from the server's {held}")
            }
            Self::Brightness { value, min, max } => {
                write!(f, "brightness {value} is outside {min}..={max}")
            }
            Self::ColourMode { color_mode, flags } => {
                write!(
                    f,
                    "colour mode {color_mode} is not allowed by flags {flags:?}"
                )
            }
            Self::ColoursMax { held, sent } => {
                write!(f, "colors_max {sent} differs from the server's {held}")
            }
            Self::ColoursMin { held, sent } => {
                write!(f, "colors_min {sent} differs from the server's {held}")
            }
            Self::ColourCount { count, min, max } => {
                write!(f, "{count} mode colours is outside {min}..={max}")
            }
            Self::Direction { direction, flags } => {
                write!(f, "direction {direction} is not allowed by flags {flags:?}")
            }
            Self::SpeedMax { held, sent } => {
                write!(f, "speed_max {sent} differs from the server's {held}")
            }
            Self::SpeedMin { held, sent } => {
                write!(f, "speed_min {sent} differs from the server's {held}")
            }
            Self::Speed { value, min, max } => write!(f, "speed {value} is outside {min}..={max}"),
        }
    }
}

/// `value` lies between `a` and `b` in either order; the server accepts ranges
/// written high-to-low (ENE's speed runs from 4, slowest, to 0, fastest).
fn between(value: u32, a: u32, b: u32) -> bool {
    a.min(b) <= value && value <= a.max(b)
}

/// The server's acceptance rule for a mode update, clause by clause
/// (`RGBController::SetModeValuesFromMode`): `current` is the server's mode at
/// the target index, `proposed` the block vogix would send.
pub fn check_acceptance(
    current: &ModeDescription,
    proposed: &ModeDescription,
) -> Result<(), RejectClause> {
    let flags = current.flags;
    if current.name != proposed.name {
        return Err(RejectClause::Name {
            held: current.name.to_string(),
            sent: proposed.name.to_string(),
        });
    }
    if current.brightness_max != proposed.brightness_max {
        return Err(RejectClause::BrightnessMax {
            held: current.brightness_max,
            sent: proposed.brightness_max,
        });
    }
    if current.brightness_min != proposed.brightness_min {
        return Err(RejectClause::BrightnessMin {
            held: current.brightness_min,
            sent: proposed.brightness_min,
        });
    }
    if !between(
        proposed.brightness,
        current.brightness_min,
        current.brightness_max,
    ) {
        return Err(RejectClause::Brightness {
            value: proposed.brightness,
            min: current.brightness_min,
            max: current.brightness_max,
        });
    }
    let colour_flags = ModeFlags::HAS_PER_LED_COLOR
        .union(ModeFlags::HAS_MODE_SPECIFIC_COLOR)
        .union(ModeFlags::HAS_RANDOM_COLOR);
    let colour_mode_allowed = match proposed.color_mode {
        ColorMode::None => !flags.intersects(colour_flags),
        ColorMode::PerLed => flags.contains(ModeFlags::HAS_PER_LED_COLOR),
        ColorMode::ModeSpecific => flags.contains(ModeFlags::HAS_MODE_SPECIFIC_COLOR),
        ColorMode::Random => flags.contains(ModeFlags::HAS_RANDOM_COLOR),
        ColorMode::Unknown(_) => false,
    };
    if !colour_mode_allowed {
        return Err(RejectClause::ColourMode {
            color_mode: proposed.color_mode,
            flags,
        });
    }
    if flags.contains(ModeFlags::HAS_MODE_SPECIFIC_COLOR) {
        if current.colors_max != proposed.colors_max {
            return Err(RejectClause::ColoursMax {
                held: current.colors_max,
                sent: proposed.colors_max,
            });
        }
        if current.colors_min != proposed.colors_min {
            return Err(RejectClause::ColoursMin {
                held: current.colors_min,
                sent: proposed.colors_min,
            });
        }
        let count = proposed.colors.len();
        let fits = u64::try_from(count).is_ok_and(|count| {
            u64::from(current.colors_min) <= count && count <= u64::from(current.colors_max)
        });
        if !fits {
            return Err(RejectClause::ColourCount {
                count,
                min: current.colors_min,
                max: current.colors_max,
            });
        }
    }
    let direction_flags = ModeFlags::HAS_DIRECTION_HV
        .union(ModeFlags::HAS_DIRECTION_LR)
        .union(ModeFlags::HAS_DIRECTION_UD);
    let direction = proposed.direction;
    let direction_allowed = (!flags.intersects(direction_flags) && direction == Direction(0))
        || (flags.contains(ModeFlags::HAS_DIRECTION_HV)
            && (direction == Direction::HORIZONTAL || direction == Direction::VERTICAL))
        || (flags.contains(ModeFlags::HAS_DIRECTION_LR)
            && (direction == Direction::LEFT || direction == Direction::RIGHT))
        || (flags.contains(ModeFlags::HAS_DIRECTION_UD)
            && (direction == Direction::UP || direction == Direction::DOWN));
    if !direction_allowed {
        return Err(RejectClause::Direction { direction, flags });
    }
    if current.speed_max != proposed.speed_max {
        return Err(RejectClause::SpeedMax {
            held: current.speed_max,
            sent: proposed.speed_max,
        });
    }
    if current.speed_min != proposed.speed_min {
        return Err(RejectClause::SpeedMin {
            held: current.speed_min,
            sent: proposed.speed_min,
        });
    }
    if !between(proposed.speed, current.speed_min, current.speed_max) {
        return Err(RejectClause::Speed {
            value: proposed.speed,
            min: current.speed_min,
            max: current.speed_max,
        });
    }
    Ok(())
}

/// The index of the mode named `name`, compared ASCII case-insensitively.
pub fn find_mode(controller: &ControllerDescription, name: &str) -> Result<usize, PlanError> {
    controller
        .modes
        .iter()
        .position(|mode| mode.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| PlanError::UnknownMode {
            requested: name.to_owned(),
            available: controller
                .modes
                .iter()
                .map(|mode| mode.name.to_string())
                .collect(),
        })
}

/// The colour list a mode takes. A colour mode the server already runs is kept
/// when the mode's flags allow it; otherwise per-LED is preferred over
/// mode-specific. A mode offering neither (only none or random colours) has no
/// colour to set.
pub fn colour_path(mode: &ModeDescription) -> Result<ColourPath, PlanError> {
    let per_led = mode.flags.contains(ModeFlags::HAS_PER_LED_COLOR);
    let mode_specific = mode.flags.contains(ModeFlags::HAS_MODE_SPECIFIC_COLOR);
    match mode.color_mode {
        ColorMode::PerLed if per_led => Ok(ColourPath::PerLed),
        ColorMode::ModeSpecific if mode_specific => Ok(ColourPath::ModeSpecific),
        _ if per_led => Ok(ColourPath::PerLed),
        _ if mode_specific => Ok(ColourPath::ModeSpecific),
        _ => Err(PlanError::NoSettableColour {
            mode: mode.name.to_string(),
            flags: mode.flags,
            color_mode: mode.color_mode,
        }),
    }
}

/// How many mode-specific colours to send: the current count clamped to
/// `colors_min..=colors_max`, and at least one.
fn mode_specific_count(mode: &ModeDescription) -> u64 {
    let current = u64::try_from(mode.colors.len()).unwrap_or(u64::MAX);
    current
        .max(u64::from(mode.colors_min))
        .min(u64::from(mode.colors_max))
        .max(1)
}

/// The writes that set `controller` to `colour` in the mode named `mode_name`.
///
/// Per-LED: `UPDATEMODE` with the server's mode switched to per-LED colours —
/// skipped on a [`ApplyKind::Reconcile`] when that mode already runs with
/// per-LED colours — then `UPDATELEDS` with one copy of the colour per LED,
/// always. Mode-specific: `UPDATEMODE` alone, its colour list set to the colour
/// repeated [`mode_specific_count`] times.
pub fn plan(
    controller: &ControllerDescription,
    mode_name: &str,
    colour: RgbColor,
    kind: ApplyKind,
) -> Result<Plan, PlanError> {
    let mode_index = find_mode(controller, mode_name)?;
    let mode = &controller.modes[mode_index];
    let reject = |clause| PlanError::ServerWouldReject {
        mode: mode.name.to_string(),
        clause,
    };
    match colour_path(mode)? {
        ColourPath::PerLed => {
            let count = controller.colors.len();
            if count == 0 {
                return Err(PlanError::NoLeds {
                    mode: mode.name.to_string(),
                });
            }
            let active = usize::try_from(controller.active_mode).ok() == Some(mode_index);
            let mode_write =
                if kind == ApplyKind::Forced || !active || mode.color_mode != ColorMode::PerLed {
                    let mut echo = mode.clone();
                    echo.color_mode = ColorMode::PerLed;
                    check_acceptance(mode, &echo).map_err(reject)?;
                    Some(echo)
                } else {
                    None
                };
            Ok(Plan {
                mode_index,
                writes: Writes::PerLed {
                    mode: mode_write,
                    leds: vec![colour; count],
                },
                expect: Expectation {
                    mode_index,
                    mode_name: mode.name.clone(),
                    color_mode: ColorMode::PerLed,
                    colours: ExpectedColours::Leds { colour, count },
                },
            })
        }
        ColourPath::ModeSpecific => {
            let wanted = mode_specific_count(mode);
            let count = usize::try_from(wanted)
                .ok()
                .filter(|count| *count <= usize::from(u16::MAX))
                .ok_or_else(|| PlanError::ColourCountUnencodable {
                    mode: mode.name.to_string(),
                    count: wanted,
                })?;
            let mut echo = mode.clone();
            echo.color_mode = ColorMode::ModeSpecific;
            echo.colors = vec![colour; count];
            check_acceptance(mode, &echo).map_err(reject)?;
            let colours = echo.colors.clone();
            Ok(Plan {
                mode_index,
                writes: Writes::ModeSpecific { mode: echo },
                expect: Expectation {
                    mode_index,
                    mode_name: mode.name.clone(),
                    color_mode: ColorMode::ModeSpecific,
                    colours: ExpectedColours::Mode(colours),
                },
            })
        }
    }
}

/// A read-back differs from what the writes should have produced.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Mismatch {
    #[error("server holds {held} modes, expected mode {index} (\"{name}\")")]
    ModeMissing {
        index: usize,
        name: String,
        held: usize,
    },
    #[error(
        "server holds active mode {held} ({held_name}), expected {expected} (\"{expected_name}\")"
    )]
    ActiveMode {
        held: i32,
        held_name: String,
        expected: usize,
        expected_name: String,
    },
    #[error("server holds {held} colours in mode \"{mode}\", expected {expected}")]
    ColourMode {
        mode: String,
        held: ColorMode,
        expected: ColorMode,
    },
    #[error("server holds {held} LED colours, expected {expected}")]
    LedCount { held: usize, expected: usize },
    #[error("server holds {held} on LED {led}, expected {expected}")]
    LedColour {
        led: usize,
        held: RgbColor,
        expected: RgbColor,
    },
    #[error("server holds mode colours {held:?}, expected {expected:?}")]
    ModeColours {
        held: Vec<RgbColor>,
        expected: Vec<RgbColor>,
    },
}

/// Compare a read-back description with what a plan should have produced.
pub fn confirm(expect: &Expectation, controller: &ControllerDescription) -> Result<(), Mismatch> {
    let expected_name = expect.mode_name.to_string();
    let Some(mode) = controller.modes.get(expect.mode_index) else {
        return Err(Mismatch::ModeMissing {
            index: expect.mode_index,
            name: expected_name,
            held: controller.modes.len(),
        });
    };
    if usize::try_from(controller.active_mode).ok() != Some(expect.mode_index) {
        return Err(Mismatch::ActiveMode {
            held: controller.active_mode,
            held_name: controller.active_mode().map_or_else(
                || "no such mode".to_owned(),
                |mode| format!("\"{}\"", mode.name),
            ),
            expected: expect.mode_index,
            expected_name,
        });
    }
    if mode.color_mode != expect.color_mode {
        return Err(Mismatch::ColourMode {
            mode: expected_name,
            held: mode.color_mode,
            expected: expect.color_mode,
        });
    }
    match &expect.colours {
        ExpectedColours::Leds { colour, count } => {
            if controller.colors.len() != *count {
                return Err(Mismatch::LedCount {
                    held: controller.colors.len(),
                    expected: *count,
                });
            }
            if let Some((led, held)) = controller
                .colors
                .iter()
                .enumerate()
                .find(|(_, held)| *held != colour)
            {
                return Err(Mismatch::LedColour {
                    led,
                    held: *held,
                    expected: *colour,
                });
            }
        }
        ExpectedColours::Mode(expected) => {
            if &mode.colors != expected {
                return Err(Mismatch::ModeColours {
                    held: mode.colors.clone(),
                    expected: expected.clone(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::model::{ControllerFlags, ControllerV6, ProtocolVersion};
    use super::super::testkit;
    use super::*;

    const RED: RgbColor = RgbColor::from_rgb(0xff, 0, 0);

    fn base_mode() -> ModeDescription {
        let mut mode = testkit::static_per_led_mode(ProtocolVersion::V6);
        mode.flags = ModeFlags::HAS_PER_LED_COLOR
            .union(ModeFlags::HAS_MODE_SPECIFIC_COLOR)
            .union(ModeFlags::HAS_BRIGHTNESS)
            .union(ModeFlags::HAS_SPEED)
            .union(ModeFlags::HAS_DIRECTION_LR);
        mode.brightness_min = 0;
        mode.brightness_max = 100;
        mode.brightness = 50;
        mode.speed_min = 1;
        mode.speed_max = 10;
        mode.speed = 5;
        mode.colors_min = 1;
        mode.colors_max = 3;
        mode.colors = vec![RED];
        mode.direction = Direction::RIGHT;
        mode
    }

    #[test]
    fn acceptance_passes_an_exact_echo() {
        let mode = base_mode();
        assert_eq!(check_acceptance(&mode, &mode), Ok(()));
    }

    #[test]
    fn acceptance_rejects_each_clause() {
        let current = base_mode();
        let check = |change: fn(&mut ModeDescription)| {
            let mut proposed = current.clone();
            change(&mut proposed);
            check_acceptance(&current, &proposed)
        };

        assert_eq!(
            check(|m| m.name = "static".into()),
            Err(RejectClause::Name {
                held: "Static".into(),
                sent: "static".into()
            })
        );
        assert_eq!(
            check(|m| m.brightness_max = 99),
            Err(RejectClause::BrightnessMax {
                held: 100,
                sent: 99
            })
        );
        assert_eq!(
            check(|m| m.brightness_min = 1),
            Err(RejectClause::BrightnessMin { held: 0, sent: 1 })
        );
        assert_eq!(
            check(|m| m.brightness = 101),
            Err(RejectClause::Brightness {
                value: 101,
                min: 0,
                max: 100
            })
        );
        assert_eq!(
            check(|m| m.color_mode = ColorMode::Random),
            Err(RejectClause::ColourMode {
                color_mode: ColorMode::Random,
                flags: current.flags
            })
        );
        assert_eq!(
            check(|m| m.color_mode = ColorMode::None),
            Err(RejectClause::ColourMode {
                color_mode: ColorMode::None,
                flags: current.flags
            })
        );
        assert_eq!(
            check(|m| m.color_mode = ColorMode::Unknown(9)),
            Err(RejectClause::ColourMode {
                color_mode: ColorMode::Unknown(9),
                flags: current.flags
            })
        );
        assert_eq!(
            check(|m| m.colors_max = 4),
            Err(RejectClause::ColoursMax { held: 3, sent: 4 })
        );
        assert_eq!(
            check(|m| m.colors_min = 0),
            Err(RejectClause::ColoursMin { held: 1, sent: 0 })
        );
        assert_eq!(
            check(|m| m.colors = vec![RED; 4]),
            Err(RejectClause::ColourCount {
                count: 4,
                min: 1,
                max: 3
            })
        );
        assert_eq!(
            check(|m| m.colors.clear()),
            Err(RejectClause::ColourCount {
                count: 0,
                min: 1,
                max: 3
            })
        );
        assert_eq!(
            check(|m| m.direction = Direction::UP),
            Err(RejectClause::Direction {
                direction: Direction::UP,
                flags: current.flags
            })
        );
        assert_eq!(
            check(|m| m.speed_max = 11),
            Err(RejectClause::SpeedMax { held: 10, sent: 11 })
        );
        assert_eq!(
            check(|m| m.speed_min = 0),
            Err(RejectClause::SpeedMin { held: 1, sent: 0 })
        );
        assert_eq!(
            check(|m| m.speed = 0),
            Err(RejectClause::Speed {
                value: 0,
                min: 1,
                max: 10
            })
        );
    }

    #[test]
    fn acceptance_colour_mode_follows_each_flag() {
        let mut current = base_mode();
        for (flags, color_mode, allowed) in [
            (ModeFlags::default(), ColorMode::None, true),
            (ModeFlags::default(), ColorMode::PerLed, false),
            (ModeFlags::HAS_RANDOM_COLOR, ColorMode::None, false),
            (ModeFlags::HAS_RANDOM_COLOR, ColorMode::Random, true),
            (ModeFlags::HAS_PER_LED_COLOR, ColorMode::PerLed, true),
            (ModeFlags::HAS_PER_LED_COLOR, ColorMode::ModeSpecific, false),
            (
                ModeFlags::HAS_MODE_SPECIFIC_COLOR,
                ColorMode::ModeSpecific,
                true,
            ),
        ] {
            current.flags = flags;
            let mut proposed = current.clone();
            proposed.color_mode = color_mode;
            proposed.direction = Direction(0);
            current.direction = Direction(0);
            assert_eq!(
                check_acceptance(&current, &proposed).is_ok(),
                allowed,
                "{flags:?} with {color_mode}"
            );
        }
    }

    #[test]
    fn acceptance_skips_colour_counts_without_the_mode_specific_flag() {
        let mut current = base_mode();
        current.flags = ModeFlags::HAS_PER_LED_COLOR;
        current.direction = Direction(0);
        let mut proposed = current.clone();
        proposed.colors_max = 42;
        proposed.colors = vec![RED; 9];
        assert_eq!(check_acceptance(&current, &proposed), Ok(()));
    }

    #[test]
    fn acceptance_direction_follows_each_flag() {
        let mut current = base_mode();
        for (flags, direction, allowed) in [
            (ModeFlags::default(), Direction(0), true),
            (ModeFlags::default(), Direction::RIGHT, false),
            (ModeFlags::HAS_DIRECTION_DIAG, Direction::UP_LEFT, false),
            (ModeFlags::HAS_DIRECTION_DIAG, Direction(0), true),
            (ModeFlags::HAS_DIRECTION_HV, Direction::HORIZONTAL, true),
            (ModeFlags::HAS_DIRECTION_HV, Direction::VERTICAL, true),
            (ModeFlags::HAS_DIRECTION_HV, Direction::LEFT, false),
            (ModeFlags::HAS_DIRECTION_LR, Direction::LEFT, true),
            (ModeFlags::HAS_DIRECTION_LR, Direction::RIGHT, true),
            (ModeFlags::HAS_DIRECTION_LR, Direction::UP, false),
            (ModeFlags::HAS_DIRECTION_UD, Direction::UP, true),
            (ModeFlags::HAS_DIRECTION_UD, Direction::DOWN, true),
            (ModeFlags::HAS_DIRECTION_UD, Direction::HORIZONTAL, false),
            (
                ModeFlags::HAS_DIRECTION_LR.union(ModeFlags::HAS_DIRECTION_DIAG),
                Direction::DOWN_RIGHT,
                false,
            ),
        ] {
            current.flags = flags.union(ModeFlags::HAS_PER_LED_COLOR);
            let mut proposed = current.clone();
            proposed.direction = direction;
            proposed.colors.clear();
            current.colors.clear();
            assert_eq!(
                check_acceptance(&current, &proposed).is_ok(),
                allowed,
                "{flags:?} with {direction}"
            );
        }
    }

    #[test]
    fn acceptance_allows_ranges_written_high_to_low() {
        let breathing = testkit::ene_dram(ProtocolVersion::V6, 0).modes[3].clone();
        assert_eq!((breathing.speed_min, breathing.speed_max), (4, 0));
        assert_eq!(check_acceptance(&breathing, &breathing), Ok(()));
        let mut faster = breathing.clone();
        faster.speed = 0;
        assert_eq!(check_acceptance(&breathing, &faster), Ok(()));
        let mut outside = breathing.clone();
        outside.speed = 5;
        assert_eq!(
            check_acceptance(&breathing, &outside),
            Err(RejectClause::Speed {
                value: 5,
                min: 4,
                max: 0
            })
        );
        let mut current = base_mode();
        current.brightness_min = 100;
        current.brightness_max = 0;
        current.brightness = 30;
        assert_eq!(check_acceptance(&current, &current), Ok(()));
    }

    #[test]
    fn selector_matches_the_display_name_substring_ignoring_case() {
        let selector = Selector::new("ene dram", "Static").unwrap();
        let first = testkit::ene_dram(ProtocolVersion::V6, 0);
        let second = testkit::ene_dram(ProtocolVersion::V6, 2);
        let govee = testkit::govee(ProtocolVersion::V6);
        let selected: Vec<_> = [&first, &govee, &second]
            .into_iter()
            .filter(|controller| selector.matches(controller))
            .collect();
        assert_eq!(selected.len(), 2, "every matching controller is selected");

        let mut renamed = testkit::ene_dram(ProtocolVersion::V6, 0);
        renamed.v6 = Some(ControllerV6 {
            display_name: "Left stick".into(),
            configuration: WireString::default(),
        });
        assert!(
            selector.matches(&renamed),
            "display_name is ignored without the flag"
        );
        renamed.flags = renamed
            .flags
            .union(ControllerFlags::MANUALLY_CONFIGURED_NAME);
        assert!(!selector.matches(&renamed));
        assert!(Selector::new("LEFT", "Static").unwrap().matches(&renamed));
    }

    #[test]
    fn selector_rejects_empty_fields() {
        assert_eq!(
            Selector::new("", "Static"),
            Err(SelectorError::EmptyNameContains)
        );
        assert_eq!(Selector::new("ENE", ""), Err(SelectorError::EmptyMode));
    }

    #[test]
    fn mode_lookup_is_exact_ignoring_case_with_no_fallback() {
        let dram = testkit::ene_dram(ProtocolVersion::V6, 0);
        assert_eq!(find_mode(&dram, "static"), Ok(2));
        assert_eq!(find_mode(&dram, "STATIC"), Ok(2));
        assert_eq!(
            find_mode(&dram, "Stat"),
            Err(PlanError::UnknownMode {
                requested: "Stat".into(),
                available: vec![
                    "Direct".into(),
                    "Off".into(),
                    "Static".into(),
                    "Breathing".into()
                ],
            })
        );
        assert_eq!(
            plan(&dram, "Rainbow", RED, ApplyKind::Reconcile)
                .unwrap_err()
                .to_string(),
            "no mode named \"Rainbow\" (modes: Direct, Off, Static, Breathing)"
        );
    }

    #[test]
    fn colour_path_prefers_the_running_colour_mode_then_per_led() {
        let mut mode = base_mode();
        mode.color_mode = ColorMode::ModeSpecific;
        assert_eq!(colour_path(&mode), Ok(ColourPath::ModeSpecific));
        mode.color_mode = ColorMode::PerLed;
        assert_eq!(colour_path(&mode), Ok(ColourPath::PerLed));
        mode.color_mode = ColorMode::Random;
        assert_eq!(colour_path(&mode), Ok(ColourPath::PerLed));
        mode.flags = ModeFlags::HAS_MODE_SPECIFIC_COLOR.union(ModeFlags::HAS_RANDOM_COLOR);
        assert_eq!(colour_path(&mode), Ok(ColourPath::ModeSpecific));
        mode.flags = ModeFlags::HAS_RANDOM_COLOR;
        assert!(matches!(
            colour_path(&mode),
            Err(PlanError::NoSettableColour { .. })
        ));
        mode.flags = ModeFlags::default();
        mode.color_mode = ColorMode::None;
        assert!(matches!(
            colour_path(&mode),
            Err(PlanError::NoSettableColour { .. })
        ));
    }

    #[test]
    fn per_led_plan_skips_an_active_mode_unless_forced_and_always_sends_leds() {
        let active = testkit::ene_dram(ProtocolVersion::V6, 2);
        let reconcile = plan(&active, "Static", RED, ApplyKind::Reconcile).unwrap();
        assert_eq!(reconcile.path(), ColourPath::PerLed);
        assert_eq!(reconcile.mode_write(), None);
        assert_eq!(reconcile.led_write(), Some(&[RED; 8][..]));
        assert_eq!(
            reconcile.expect.colours,
            ExpectedColours::Leds {
                colour: RED,
                count: 8
            }
        );

        let forced = plan(&active, "Static", RED, ApplyKind::Forced).unwrap();
        assert_eq!(forced.mode_write(), Some(&active.modes[2]));
        assert_eq!(forced.led_write(), Some(&[RED; 8][..]));

        let inactive = testkit::ene_dram(ProtocolVersion::V6, 0);
        let switch = plan(&inactive, "Static", RED, ApplyKind::Reconcile).unwrap();
        assert_eq!(switch.mode_index, 2);
        assert_eq!(switch.mode_write(), Some(&inactive.modes[2]));

        let mut random = testkit::ene_dram(ProtocolVersion::V6, 3);
        random.modes[3].color_mode = ColorMode::Random;
        let recolour = plan(&random, "Breathing", RED, ApplyKind::Reconcile).unwrap();
        let echo = recolour.mode_write().expect("colour mode differs");
        assert_eq!(echo.color_mode, ColorMode::PerLed);
        assert_eq!(
            (echo.speed, echo.speed_min, echo.speed_max),
            (2, 4, 0),
            "speed is echoed"
        );
    }

    #[test]
    fn per_led_plan_needs_leds() {
        let mut empty = testkit::ene_dram(ProtocolVersion::V5, 0);
        empty.colors.clear();
        assert_eq!(
            plan(&empty, "Static", RED, ApplyKind::Reconcile),
            Err(PlanError::NoLeds {
                mode: "Static".into()
            })
        );
    }

    #[test]
    fn mode_specific_plan_clamps_the_colour_count_and_sends_only_the_mode() {
        let govee = testkit::govee(ProtocolVersion::V6);
        let fixed = plan(&govee, "static", RED, ApplyKind::Reconcile).unwrap();
        assert_eq!(fixed.path(), ColourPath::ModeSpecific);
        assert_eq!(fixed.led_write(), None);
        let echo = fixed.mode_write().expect("the mode carries the colour");
        assert_eq!(echo.colors, vec![RED]);
        assert_eq!(echo.brightness, 100, "brightness is echoed, not forced");
        assert_eq!(fixed.expect.colours, ExpectedColours::Mode(vec![RED]));

        let mut controller = testkit::govee(ProtocolVersion::V6);
        let cases = [
            (0usize, 1u32, 1u32, Some(1usize)),
            (5, 1, 3, Some(3)),
            (2, 1, 4, Some(2)),
            (0, 2, 4, Some(2)),
            (0, 0, 0, None),
            (0, 3, 1, None),
        ];
        for (len, min, max, expected) in cases {
            let mode = &mut controller.modes[1];
            mode.colors = vec![RgbColor::default(); len];
            mode.colors_min = min;
            mode.colors_max = max;
            let result = plan(&controller, "Static", RED, ApplyKind::Reconcile);
            match expected {
                Some(count) => assert_eq!(
                    result.unwrap().mode_write().unwrap().colors,
                    vec![RED; count],
                    "len {len} in {min}..={max}"
                ),
                None => assert!(
                    matches!(
                        result,
                        Err(PlanError::ServerWouldReject {
                            clause: RejectClause::ColourCount { .. },
                            ..
                        })
                    ),
                    "len {len} in {min}..={max}"
                ),
            }
        }

        let mode = &mut controller.modes[1];
        mode.colors.clear();
        mode.colors_min = 70_000;
        mode.colors_max = 80_000;
        assert_eq!(
            plan(&controller, "Static", RED, ApplyKind::Reconcile),
            Err(PlanError::ColourCountUnencodable {
                mode: "Static".into(),
                count: 70_000
            })
        );
    }

    #[test]
    fn a_mode_the_server_would_drop_is_not_planned() {
        let mut dram = testkit::ene_dram(ProtocolVersion::V6, 0);
        dram.modes[2].brightness = 7;
        assert_eq!(
            plan(&dram, "Static", RED, ApplyKind::Reconcile),
            Err(PlanError::ServerWouldReject {
                mode: "Static".into(),
                clause: RejectClause::Brightness {
                    value: 7,
                    min: 0,
                    max: 0
                }
            })
        );
        let mut off = testkit::ene_dram(ProtocolVersion::V6, 0);
        off.modes[1].flags = ModeFlags::HAS_RANDOM_COLOR;
        assert!(matches!(
            plan(&off, "Off", RED, ApplyKind::Reconcile),
            Err(PlanError::NoSettableColour { .. })
        ));
    }

    #[test]
    fn confirmation_compares_mode_colour_mode_and_colours() {
        let before = testkit::ene_dram(ProtocolVersion::V6, 0);
        let planned = plan(&before, "Static", RED, ApplyKind::Reconcile).unwrap();
        let mut after = before.clone();
        after.active_mode = 2;
        after.colors = vec![RED; 8];
        assert_eq!(confirm(&planned.expect, &after), Ok(()));

        let mismatch = confirm(&planned.expect, &before).unwrap_err();
        assert_eq!(
            mismatch.to_string(),
            "server holds active mode 0 (\"Direct\"), expected 2 (\"Static\")"
        );

        let mut one_led_off = after.clone();
        one_led_off.colors[5] = RgbColor::from_rgb(0, 0, 0);
        assert_eq!(
            confirm(&planned.expect, &one_led_off)
                .unwrap_err()
                .to_string(),
            "server holds #000000 on LED 5, expected #ff0000"
        );

        let mut shorter = after.clone();
        shorter.colors.pop();
        assert_eq!(
            confirm(&planned.expect, &shorter),
            Err(Mismatch::LedCount {
                held: 7,
                expected: 8
            })
        );

        let mut random = after.clone();
        random.modes[2].color_mode = ColorMode::Random;
        assert!(matches!(
            confirm(&planned.expect, &random),
            Err(Mismatch::ColourMode { .. })
        ));

        let mut fewer_modes = after;
        fewer_modes.modes.truncate(2);
        assert!(matches!(
            confirm(&planned.expect, &fewer_modes),
            Err(Mismatch::ModeMissing { index: 2, .. })
        ));

        let govee = testkit::govee(ProtocolVersion::V6);
        let fixed = plan(&govee, "Static", RED, ApplyKind::Reconcile).unwrap();
        let mut applied = govee.clone();
        applied.active_mode = 1;
        applied.modes[1].colors = vec![RED];
        assert_eq!(confirm(&fixed.expect, &applied), Ok(()));
        applied.modes[1].colors = vec![RgbColor::from_rgb(0, 0, 1)];
        assert!(matches!(
            confirm(&fixed.expect, &applied),
            Err(Mismatch::ModeColours { .. })
        ));
    }
}
