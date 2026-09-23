//! The OpenRGB client session: the [`Mirror`] of the server's controllers plus
//! the writes that keep the selected controllers at their target colours.
//!
//! [`Session`] is a pure state machine — bytes in, bytes and events out — for
//! the owner loop to drive: it writes [`Session::pending_output`] to the socket
//! when writable, feeds what it reads to [`Session::receive`], hands palette
//! changes to [`Session::set_targets`] and a forced re-apply to
//! [`Session::force_reapply`], and closes once [`Session::should_close`] holds
//! after [`Session::begin_shutdown`] or a fault. Nothing here reads a clock,
//! sleeps or retries.
//!
//! # When it applies
//!
//! A reconcile applies every target to the controllers it selects. It runs
//! when a resync commits the list, when targets change, on
//! [`Session::force_reapply`], and — at protocol 6 — on `DETECTION_COMPLETE`,
//! which the server sends after its own startup profile and which also reports
//! every target that selects no controller. A reconcile requested while the list
//! is being resynced waits for the commit. Each controller runs at most one
//! apply at a time; a reconcile that reaches a busy controller marks it for one
//! more apply when the current one finishes.
//!
//! # What an apply sends
//!
//! Protocol 6, per controller, one packet at a time: `UPDATEMODE` when the plan
//! needs it, its `ACK`, `UPDATELEDS` on the per-LED path, its `ACK`, then
//! `REQUEST_CONTROLLER_DATA` as a read-back that is compared with the plan. A
//! write is acknowledged from the controller's own server thread after it is
//! applied, so the read-back is sent only after the last `ACK`. The result is
//! [`ApplyState::Confirmed`], a [`Mismatch`] naming what the server holds, or an
//! error naming a non-OK status. A write to an id the server no longer lists is
//! acknowledged OK without being applied; the read-back then finds no data and
//! the state is [`ApplyState::Vanished`].
//!
//! Protocol 5 has no acknowledgements: the writes go out back to back and the
//! state is [`ApplyState::Sent`]. Nothing is read back at protocol 5, so the
//! cache keeps the description from the last resync and a later apply writes
//! the mode again.

pub mod mirror;

pub use mirror::{Mirror, MirrorConfig, MirrorEvent, ReadBackToken};

use super::codec;
use super::model::{AckStatus, ProtocolVersion};
use super::select::{self, ApplyKind, Mismatch, Plan, PlanError, Selector, Writes};
use super::wire::{EncodeError, PacketId, RgbColor};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

/// What one device spec wants: the controllers its selector matches, in its
/// mode, at this colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub selector: Selector,
    pub colour: RgbColor,
}

/// The outcome of applying a target to one controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyState {
    /// Protocol 6 writes are in flight.
    Pending,
    /// Protocol 5 writes were sent; that protocol cannot confirm them.
    Sent,
    /// The read-back matches the plan.
    Confirmed,
    Mismatch(Mismatch),
    /// The server acknowledged a write with a non-OK status.
    Rejected {
        packet: PacketId,
        status: AckStatus,
    },
    /// The controller left the server's list during the apply.
    Vanished,
    NotPlanned(PlanError),
    /// An earlier device spec (in name order) already selects this controller.
    Conflict {
        claimed_by: String,
    },
    EncodeFailed(EncodeError),
}

impl ApplyState {
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::Mismatch(_)
                | Self::Rejected { .. }
                | Self::NotPlanned(_)
                | Self::Conflict { .. }
                | Self::EncodeFailed(_)
        )
    }
}

impl fmt::Display for ApplyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => f.write_str("pending"),
            Self::Sent => f.write_str("sent"),
            Self::Confirmed => f.write_str("confirmed"),
            Self::Mismatch(mismatch) => write!(f, "{mismatch}"),
            Self::Rejected { packet, status } => {
                write!(f, "OpenRGB answered {packet} with {status}")
            }
            Self::Vanished => f.write_str("the controller left OpenRGB's list"),
            Self::NotPlanned(error) => write!(f, "{error}"),
            Self::Conflict { claimed_by } => {
                write!(f, "controller already selected by {claimed_by}")
            }
            Self::EncodeFailed(error) => write!(f, "{error}"),
        }
    }
}

/// A device spec's overall state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceState {
    /// No controller list has been committed yet.
    Waiting,
    /// No controller matches the selector.
    Absent,
    Pending,
    Sent,
    Confirmed,
    Error,
}

impl fmt::Display for DeviceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Waiting => "waiting",
            Self::Absent => "absent",
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Confirmed => "confirmed",
            Self::Error => "error",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerReport {
    pub dev_id: u32,
    pub name: String,
    pub state: ApplyState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceReport {
    pub label: String,
    pub state: DeviceState,
    pub controllers: Vec<ControllerReport>,
}

/// What the session observed or did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Mirror(MirrorEvent),
    /// Whether any controller matches a target changed (including its first
    /// determination).
    Presence {
        label: String,
        present: bool,
        available: Vec<String>,
    },
    /// At protocol 6, once per `DETECTION_COMPLETE`: a target selects no
    /// controller.
    AbsentAtDetectionComplete {
        label: String,
        name_contains: String,
        available: Vec<String>,
    },
    /// A controller's apply state was set.
    Apply {
        label: String,
        dev_id: u32,
        controller: String,
        state: ApplyState,
    },
    /// An acknowledgement or read-back arrived for a controller that has since
    /// left the list; it changes nothing.
    StaleDiscarded {
        dev_id: u32,
        packet: PacketId,
    },
}

#[derive(Debug)]
struct Assignment {
    label: String,
    state: Option<ApplyState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    ModeAck,
    LedsAck,
    ReadBack,
}

#[derive(Debug)]
struct Pipeline {
    label: String,
    plan: Plan,
    step: Step,
    token: ReadBackToken,
    /// The controller left the list; the outstanding reply is discarded.
    cancelled: bool,
    /// Apply once more when this apply finishes.
    rerun: Option<ApplyKind>,
}

/// The OpenRGB client session.
#[derive(Debug)]
pub struct Session {
    mirror: Mirror,
    targets: BTreeMap<String, Target>,
    assignments: BTreeMap<u32, Assignment>,
    /// (label, dev_id) → the label that claimed the controller first.
    conflicts: BTreeMap<(String, u32), String>,
    pipelines: BTreeMap<u32, Pipeline>,
    presence: BTreeMap<String, bool>,
    deferred: Option<ApplyKind>,
    report_absence: bool,
    next_token: u64,
    events: VecDeque<Event>,
}

impl Session {
    pub fn new(config: MirrorConfig) -> Self {
        Self {
            mirror: Mirror::new(config),
            targets: BTreeMap::new(),
            assignments: BTreeMap::new(),
            conflicts: BTreeMap::new(),
            pipelines: BTreeMap::new(),
            presence: BTreeMap::new(),
            deferred: None,
            report_absence: false,
            next_token: 0,
            events: VecDeque::new(),
        }
    }

    pub fn mirror(&self) -> &Mirror {
        &self.mirror
    }

    pub fn protocol(&self) -> Option<ProtocolVersion> {
        self.mirror.protocol()
    }

    /// Bytes to write to the socket, oldest first.
    pub fn pending_output(&self) -> &[u8] {
        self.mirror.pending_output()
    }

    /// Record that the first `written` bytes of [`Session::pending_output`]
    /// reached the socket.
    pub fn consume_output(&mut self, written: usize) {
        self.mirror.consume_output(written);
    }

    /// Feed bytes read from the socket and act on every complete frame.
    pub fn receive(&mut self, bytes: &[u8]) {
        self.mirror.feed(bytes);
        while self.mirror.process_next() {
            self.drain_mirror();
        }
        self.drain_mirror();
    }

    /// Replace the targets, keyed by device name, and reconcile.
    pub fn set_targets(&mut self, targets: BTreeMap<String, Target>) {
        self.targets = targets;
        self.presence
            .retain(|label, _| self.targets.contains_key(label));
        self.reconcile(ApplyKind::Reconcile);
    }

    /// Re-apply every target, writing modes even where they are already active.
    pub fn force_reapply(&mut self) {
        self.reconcile(ApplyKind::Forced);
    }

    /// Start no further requests; [`Session::should_close`] reports when the
    /// socket can close.
    pub fn begin_shutdown(&mut self) {
        self.mirror.begin_shutdown();
        self.deferred = None;
    }

    /// The socket can close now (see [`Mirror::should_close`]).
    pub fn should_close(&self) -> bool {
        self.mirror.should_close()
    }

    /// Every pending event, oldest first.
    pub fn drain_events(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }

    /// Each target's state and the controllers it selects, in server order.
    pub fn device_reports(&self) -> Vec<DeviceReport> {
        self.targets
            .keys()
            .map(|label| {
                if !self.mirror.has_committed() {
                    return DeviceReport {
                        label: label.clone(),
                        state: DeviceState::Waiting,
                        controllers: Vec::new(),
                    };
                }
                let controllers: Vec<ControllerReport> = self
                    .mirror
                    .controllers()
                    .filter_map(|controller| {
                        let state = match self.assignments.get(&controller.dev_id) {
                            Some(assignment) if &assignment.label == label => {
                                assignment.state.clone().unwrap_or(ApplyState::Pending)
                            }
                            _ => ApplyState::Conflict {
                                claimed_by: self
                                    .conflicts
                                    .get(&(label.clone(), controller.dev_id))?
                                    .clone(),
                            },
                        };
                        Some(ControllerReport {
                            dev_id: controller.dev_id,
                            name: controller.description.display_name().to_string(),
                            state,
                        })
                    })
                    .collect();
                let state = if controllers.is_empty() {
                    DeviceState::Absent
                } else if controllers.iter().any(|c| c.state.is_error()) {
                    DeviceState::Error
                } else if controllers.iter().all(|c| c.state == ApplyState::Confirmed) {
                    DeviceState::Confirmed
                } else if controllers.iter().all(|c| c.state == ApplyState::Sent) {
                    DeviceState::Sent
                } else {
                    DeviceState::Pending
                };
                DeviceReport {
                    label: label.clone(),
                    state,
                    controllers,
                }
            })
            .collect()
    }

    fn emit(&mut self, event: Event) {
        self.events.push_back(event);
    }

    fn drain_mirror(&mut self) {
        while let Some(event) = self.mirror.pop_event() {
            let reaction = event.clone();
            self.emit(Event::Mirror(event));
            match reaction {
                MirrorEvent::WriteAck {
                    dev_id,
                    packet,
                    status,
                } => self.on_write_ack(dev_id, packet, status),
                MirrorEvent::ReadBack {
                    dev_id,
                    token,
                    found,
                } => self.on_read_back(dev_id, token, found),
                MirrorEvent::ControllerRemoved { dev_id } => self.on_removed(dev_id),
                MirrorEvent::ListCommitted { .. } => {
                    if self.mirror.protocol() == Some(ProtocolVersion::V5) {
                        // Protocol 5 addresses are list indices, which a new
                        // list reassigns.
                        self.assignments.clear();
                        self.conflicts.clear();
                    }
                    self.reconcile(ApplyKind::Reconcile);
                }
                MirrorEvent::DetectionComplete => {
                    self.report_absence = true;
                    self.reconcile(ApplyKind::Forced);
                }
                _ => {}
            }
        }
    }

    fn ready(&self) -> bool {
        self.mirror.is_settled() && !self.mirror.is_shutting_down()
    }

    fn reconcile(&mut self, kind: ApplyKind) {
        if self.mirror.is_shutting_down() || self.mirror.fault().is_some() {
            return;
        }
        let kind = self
            .deferred
            .take()
            .map_or(kind, |deferred| deferred.max(kind));
        if !self.ready() {
            self.deferred = Some(kind);
            return;
        }
        let report_absence = std::mem::take(&mut self.report_absence);

        let mut claims: BTreeMap<u32, String> = BTreeMap::new();
        let mut conflicts: BTreeMap<(String, u32), String> = BTreeMap::new();
        let mut matched: BTreeMap<&str, usize> = BTreeMap::new();
        for controller in self.mirror.controllers() {
            let mut matching = self
                .targets
                .iter()
                .filter(|(_, target)| target.selector.matches(&controller.description))
                .map(|(label, _)| label);
            if let Some(first) = matching.next() {
                claims.insert(controller.dev_id, first.clone());
                *matched.entry(first.as_str()).or_default() += 1;
                for other in matching {
                    conflicts.insert((other.clone(), controller.dev_id), first.clone());
                    *matched.entry(other.as_str()).or_default() += 1;
                }
            }
        }
        let available: Vec<String> = self
            .mirror
            .controllers()
            .map(|controller| controller.description.display_name().to_string())
            .collect();

        let mut presence_events = Vec::new();
        for (label, target) in &self.targets {
            let present = matched.contains_key(label.as_str());
            if self.presence.insert(label.clone(), present) != Some(present) {
                presence_events.push(Event::Presence {
                    label: label.clone(),
                    present,
                    available: available.clone(),
                });
            }
            if report_absence && !present {
                presence_events.push(Event::AbsentAtDetectionComplete {
                    label: label.clone(),
                    name_contains: target.selector.name_contains().to_owned(),
                    available: available.clone(),
                });
            }
        }
        for event in presence_events {
            self.emit(event);
        }

        self.assignments
            .retain(|dev_id, assignment| claims.get(dev_id) == Some(&assignment.label));
        for ((label, dev_id), claimed_by) in &conflicts {
            if self.conflicts.get(&(label.clone(), *dev_id)) != Some(claimed_by) {
                let controller = self.controller_name(*dev_id);
                self.events.push_back(Event::Apply {
                    label: label.clone(),
                    dev_id: *dev_id,
                    controller,
                    state: ApplyState::Conflict {
                        claimed_by: claimed_by.clone(),
                    },
                });
            }
        }
        self.conflicts = conflicts;

        for (dev_id, label) in claims {
            self.assignments
                .entry(dev_id)
                .or_insert_with(|| Assignment {
                    label: label.clone(),
                    state: None,
                });
            self.apply_one(dev_id, &label, kind);
        }
    }

    fn controller_name(&self, dev_id: u32) -> String {
        self.mirror
            .controller(dev_id)
            .map(|controller| controller.description.display_name().to_string())
            .unwrap_or_default()
    }

    fn set_state(&mut self, dev_id: u32, state: ApplyState) {
        let Some(assignment) = self.assignments.get_mut(&dev_id) else {
            return;
        };
        // A failure to plan repeats on every reconcile until the server or the
        // spec changes; it is reported when it first appears.
        let repeat = assignment.state.as_ref() == Some(&state)
            && matches!(
                state,
                ApplyState::NotPlanned(_) | ApplyState::EncodeFailed(_)
            );
        assignment.state = Some(state.clone());
        let label = assignment.label.clone();
        if !repeat {
            let controller = self.controller_name(dev_id);
            self.emit(Event::Apply {
                label,
                dev_id,
                controller,
                state,
            });
        }
    }

    fn apply_one(&mut self, dev_id: u32, label: &str, kind: ApplyKind) {
        if let Some(pipeline) = self.pipelines.get_mut(&dev_id) {
            pipeline.rerun = Some(pipeline.rerun.map_or(kind, |rerun| rerun.max(kind)));
            return;
        }
        let (Some(target), Some(controller), Some(protocol)) = (
            self.targets.get(label),
            self.mirror.controller(dev_id),
            self.mirror.protocol(),
        ) else {
            return;
        };
        let plan = match select::plan(
            &controller.description,
            target.selector.mode(),
            target.colour,
            kind,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                self.set_state(dev_id, ApplyState::NotPlanned(error));
                return;
            }
        };
        match protocol {
            ProtocolVersion::V5 => self.send_unacknowledged(dev_id, &plan),
            ProtocolVersion::V6 => self.start_pipeline(dev_id, label, plan),
        }
    }

    /// Protocol 5: mode then LEDs, back to back.
    fn send_unacknowledged(&mut self, dev_id: u32, plan: &Plan) {
        let mut frames = Vec::new();
        if let Some(mode) = plan.mode_write() {
            match codec::update_mode(dev_id, plan.mode_index, mode, ProtocolVersion::V5) {
                Ok(frame) => frames.push((PacketId::UpdateMode, frame)),
                Err(error) => return self.set_state(dev_id, ApplyState::EncodeFailed(error)),
            }
        }
        if let Some(leds) = plan.led_write() {
            match codec::update_leds(dev_id, leds) {
                Ok(frame) => frames.push((PacketId::UpdateLeds, frame)),
                Err(error) => return self.set_state(dev_id, ApplyState::EncodeFailed(error)),
            }
        }
        for (packet, frame) in frames {
            self.mirror.send_write(dev_id, packet, &frame);
        }
        self.set_state(dev_id, ApplyState::Sent);
    }

    fn start_pipeline(&mut self, dev_id: u32, label: &str, plan: Plan) {
        let token = ReadBackToken(self.next_token);
        self.next_token += 1;
        let first = match &plan.writes {
            Writes::PerLed {
                mode: Some(mode), ..
            }
            | Writes::ModeSpecific { mode } => {
                codec::update_mode(dev_id, plan.mode_index, mode, ProtocolVersion::V6)
                    .map(|frame| (Step::ModeAck, PacketId::UpdateMode, frame))
            }
            Writes::PerLed { mode: None, leds } => codec::update_leds(dev_id, leds)
                .map(|frame| (Step::LedsAck, PacketId::UpdateLeds, frame)),
        };
        let (step, packet, frame) = match first {
            Ok(first) => first,
            Err(error) => return self.set_state(dev_id, ApplyState::EncodeFailed(error)),
        };
        self.mirror.send_write(dev_id, packet, &frame);
        self.pipelines.insert(
            dev_id,
            Pipeline {
                label: label.to_owned(),
                plan,
                step,
                token,
                cancelled: false,
                rerun: None,
            },
        );
        self.set_state(dev_id, ApplyState::Pending);
    }

    fn on_write_ack(&mut self, dev_id: u32, packet: PacketId, status: AckStatus) {
        let Some(pipeline) = self.pipelines.get_mut(&dev_id) else {
            return;
        };
        let awaited = match pipeline.step {
            Step::ModeAck => PacketId::UpdateMode,
            Step::LedsAck => PacketId::UpdateLeds,
            Step::ReadBack => return,
        };
        if packet != awaited {
            return;
        }
        if pipeline.cancelled {
            self.pipelines.remove(&dev_id);
            self.emit(Event::StaleDiscarded { dev_id, packet });
            return;
        }
        if status != AckStatus::Ok {
            let outcome = if status == AckStatus::ErrorInvalidId {
                // The controller's server thread processed the write after the
                // id left the list.
                ApplyState::Vanished
            } else {
                ApplyState::Rejected { packet, status }
            };
            return self.finish(dev_id, outcome);
        }
        if self.mirror.is_shutting_down() || self.mirror.fault().is_some() {
            self.pipelines.remove(&dev_id);
            return;
        }
        let next_leds = match (&pipeline.step, &pipeline.plan.writes) {
            (Step::ModeAck, Writes::PerLed { leds, .. }) => Some(codec::update_leds(dev_id, leds)),
            _ => None,
        };
        match next_leds {
            Some(Ok(frame)) => {
                pipeline.step = Step::LedsAck;
                self.mirror.send_write(dev_id, PacketId::UpdateLeds, &frame);
            }
            Some(Err(error)) => self.finish(dev_id, ApplyState::EncodeFailed(error)),
            None => {
                pipeline.step = Step::ReadBack;
                let token = pipeline.token;
                self.mirror.request_readback(dev_id, token);
            }
        }
    }

    fn on_read_back(&mut self, dev_id: u32, token: ReadBackToken, found: bool) {
        let Some(pipeline) = self
            .pipelines
            .get(&dev_id)
            .filter(|pipeline| pipeline.token == token && pipeline.step == Step::ReadBack)
        else {
            self.emit(Event::StaleDiscarded {
                dev_id,
                packet: PacketId::RequestControllerData,
            });
            return;
        };
        if pipeline.cancelled {
            self.pipelines.remove(&dev_id);
            self.emit(Event::StaleDiscarded {
                dev_id,
                packet: PacketId::RequestControllerData,
            });
            return;
        }
        let outcome = match (found, self.mirror.controller(dev_id)) {
            (true, Some(controller)) => {
                match select::confirm(&pipeline.plan.expect, &controller.description) {
                    Ok(()) => ApplyState::Confirmed,
                    Err(mismatch) => ApplyState::Mismatch(mismatch),
                }
            }
            _ => ApplyState::Vanished,
        };
        self.finish(dev_id, outcome);
    }

    fn finish(&mut self, dev_id: u32, outcome: ApplyState) {
        let Some(pipeline) = self.pipelines.remove(&dev_id) else {
            return;
        };
        let current_label = self
            .assignments
            .get(&dev_id)
            .map(|assignment| assignment.label.clone());
        if current_label.as_deref() == Some(pipeline.label.as_str()) {
            self.set_state(dev_id, outcome);
        }
        let (Some(kind), Some(label)) = (pipeline.rerun, current_label) else {
            return;
        };
        if self.ready() {
            self.apply_one(dev_id, &label, kind);
        } else if !self.mirror.is_shutting_down() && self.mirror.fault().is_none() {
            self.deferred = Some(self.deferred.map_or(kind, |deferred| deferred.max(kind)));
        }
    }

    fn on_removed(&mut self, dev_id: u32) {
        if let Some(pipeline) = self.pipelines.get_mut(&dev_id) {
            pipeline.cancelled = true;
            pipeline.rerun = None;
        }
        self.assignments.remove(&dev_id);
        self.conflicts
            .retain(|(_, conflicted), _| *conflicted != dev_id);
    }
}

#[cfg(test)]
mod tests;
