//! State-machine tests of the session's own logic, driven by scripted server
//! byte streams built with `testkit::server`. They are not evidence of protocol
//! compatibility with OpenRGB; the real-server VM checks are.

use super::super::codec;
use super::super::model::{
    AckStatus, ClientName, ColorMode, ControllerDescription, ProtocolVersion, UnsupportedProtocol,
    UpdateReason,
};
use super::super::select::{Mismatch, PlanError, RejectClause, Selector};
use super::super::testkit::{self, server};
use super::super::wire::{Frame, Framer, PacketId, Reader, RgbColor, WireString};
use super::mirror::{Fault, Phase};
use super::*;

const RED: RgbColor = RgbColor::from_rgb(0xff, 0, 0);
const BLUE: RgbColor = RgbColor::from_rgb(0, 0, 0xff);

fn config(max_protocol: ProtocolVersion) -> MirrorConfig {
    MirrorConfig {
        max_protocol,
        client_name: ClientName::new("vogix").unwrap(),
    }
}

/// Take and frame everything the session queued for the socket.
fn sent(session: &mut Session) -> Vec<Frame> {
    let bytes = session.pending_output().to_vec();
    session.consume_output(bytes.len());
    let mut framer = Framer::new();
    framer.push(&bytes);
    let mut frames = Vec::new();
    while let Some(frame) = framer.next_frame().unwrap() {
        frames.push(frame);
    }
    assert_eq!(framer.buffered(), 0);
    frames
}

fn ids(frames: &[Frame]) -> Vec<(PacketId, u32)> {
    frames
        .iter()
        .map(|frame| (frame.pkt_id, frame.dev_id))
        .collect()
}

fn feed(session: &mut Session, frames: &[Vec<u8>]) {
    for frame in frames {
        session.receive(frame);
    }
}

fn target(name_contains: &str, mode: &str, colour: RgbColor) -> Target {
    Target {
        selector: Selector::new(name_contains, mode).unwrap(),
        colour,
    }
}

fn targets(entries: &[(&str, Target)]) -> BTreeMap<String, Target> {
    entries
        .iter()
        .map(|(label, target)| ((*label).to_owned(), target.clone()))
        .collect()
}

fn apply_states(events: &[Event]) -> Vec<(String, u32, ApplyState)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Apply {
                label,
                dev_id,
                state,
                ..
            } => Some((label.clone(), *dev_id, state.clone())),
            _ => None,
        })
        .collect()
}

/// The mode index and block an `UPDATEMODE` frame carries.
fn update_mode_body(
    frame: &Frame,
    version: ProtocolVersion,
) -> (i32, super::super::model::ModeDescription) {
    assert_eq!(frame.pkt_id, PacketId::UpdateMode);
    let mut r = Reader::new(&frame.payload);
    assert_eq!(r.u32("data_size").unwrap() as usize, frame.payload.len());
    let index = r.i32("mode_idx").unwrap();
    let mode = codec::decode_mode(&mut r, version).unwrap();
    r.finish("update mode").unwrap();
    (index, mode)
}

/// The colours an `UPDATELEDS` frame carries.
fn update_leds_body(frame: &Frame) -> Vec<RgbColor> {
    assert_eq!(frame.pkt_id, PacketId::UpdateLeds);
    let mut r = Reader::new(&frame.payload);
    assert_eq!(r.u32("data_size").unwrap() as usize, frame.payload.len());
    let count = r.u16("num_colors").unwrap();
    let colours = r.u32_array(usize::from(count), "colors").unwrap();
    r.finish("update leds").unwrap();
    colours.into_iter().map(RgbColor::from_wire).collect()
}

/// A protocol 6 session whose list holds `controllers`, committed, with its
/// handshake output and events consumed.
fn v6_ready(
    controllers: &[(u32, ControllerDescription)],
    wanted: BTreeMap<String, Target>,
) -> Session {
    let mut session = Session::new(config(ProtocolVersion::V6));
    session.set_targets(wanted);
    sent(&mut session);
    let ids: Vec<u32> = controllers.iter().map(|(id, _)| *id).collect();
    feed(
        &mut session,
        &[
            server::version_reply(6),
            server::server_name("OpenRGB"),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&ids),
            server::ok(0, PacketId::RequestControllerCount),
            server::server_flags(0x7f),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
        ],
    );
    sent(&mut session);
    for (id, description) in controllers {
        feed(
            &mut session,
            &[
                server::controller_data(*id, description, ProtocolVersion::V6),
                server::ok(*id, PacketId::RequestControllerData),
            ],
        );
    }
    assert_eq!(session.mirror().phase(), Phase::Ready);
    session
}

/// A protocol 5 session whose list holds `controllers` at indices 0.., committed.
fn v5_ready(controllers: &[ControllerDescription], wanted: BTreeMap<String, Target>) -> Session {
    let mut session = Session::new(config(ProtocolVersion::V6));
    session.set_targets(wanted);
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::version_reply(5),
            server::count_v5(controllers.len() as u32),
        ],
    );
    sent(&mut session);
    for (index, description) in controllers.iter().enumerate() {
        feed(
            &mut session,
            &[server::controller_data(
                index as u32,
                description,
                ProtocolVersion::V5,
            )],
        );
    }
    feed(&mut session, &[server::count_v5(controllers.len() as u32)]);
    assert_eq!(session.mirror().phase(), Phase::Ready);
    session
}

/// Complete the protocol 6 read-back for `id` with `description`.
fn read_back(session: &mut Session, id: u32, description: &ControllerDescription) {
    feed(
        session,
        &[
            server::controller_data(id, description, ProtocolVersion::V6),
            server::ok(id, PacketId::RequestControllerData),
        ],
    );
}

fn applied_dram(colour: RgbColor) -> ControllerDescription {
    let mut dram = testkit::ene_dram(ProtocolVersion::V6, 2);
    dram.colors = vec![colour; 8];
    dram
}

#[test]
fn v6_handshake_sends_flags_and_name_after_the_version_reply_and_tolerates_interleaving() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    let hello = sent(&mut session);
    assert_eq!(
        ids(&hello),
        [
            (PacketId::RequestProtocolVersion, 0),
            (PacketId::RequestControllerCount, 0)
        ]
    );
    assert_eq!(hello[0].payload, 6u32.to_le_bytes());
    assert_eq!(session.mirror().phase(), Phase::Handshaking);

    // The server name (51) and the version ACK follow the version reply; a
    // SIGNALUPDATE for an unknown controller and an unknown packet from other
    // server threads fall in between.
    feed(
        &mut session,
        &[
            server::version_reply(6),
            server::signal_update_leds(99, &[RED]),
            server::server_name("OpenRGB"),
            server::frame(0, PacketId::Unknown(160), vec![0; 6]),
            server::ok(0, PacketId::RequestProtocolVersion),
        ],
    );
    let after_version = sent(&mut session);
    assert_eq!(
        ids(&after_version),
        [(PacketId::SetClientFlags, 0), (PacketId::SetClientName, 0)]
    );
    assert_eq!(
        after_version[0].payload,
        1u32.to_le_bytes(),
        "SUPPORTS_RGBCONTROLLER"
    );
    assert_eq!(after_version[1].payload, b"vogix\0");
    assert_eq!(session.protocol(), Some(ProtocolVersion::V6));
    assert_eq!(
        session.mirror().server_name(),
        Some(&WireString::from("OpenRGB"))
    );

    // The count fence seeds the first enumeration; the flags and name ACKs
    // arrive after it, in request order.
    feed(
        &mut session,
        &[
            server::count_v6(&[7, 9]),
            server::ok(0, PacketId::RequestControllerCount),
            server::server_flags(0x7f),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
        ],
    );
    let data = sent(&mut session);
    assert_eq!(
        ids(&data),
        [
            (PacketId::RequestControllerData, 7),
            (PacketId::RequestControllerData, 9)
        ]
    );
    assert_eq!(data[0].payload, 6u32.to_le_bytes());
    assert!(matches!(
        session.mirror().phase(),
        Phase::Enumerating { .. }
    ));

    feed(
        &mut session,
        &[
            server::controller_data(
                7,
                &testkit::ene_dram(ProtocolVersion::V6, 0),
                ProtocolVersion::V6,
            ),
            server::ok(7, PacketId::RequestControllerData),
            server::controller_data(9, &testkit::govee(ProtocolVersion::V6), ProtocolVersion::V6),
            server::ok(9, PacketId::RequestControllerData),
        ],
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
    let events = session.drain_events();
    assert!(events.contains(&Event::Mirror(MirrorEvent::Negotiated {
        server_max: 6,
        protocol: ProtocolVersion::V6
    })));
    assert!(events.contains(&Event::Mirror(MirrorEvent::Skipped {
        packet: PacketId::SignalUpdate,
        dev_id: 99,
        size: 14
    })));
    assert!(events.contains(&Event::Mirror(MirrorEvent::Skipped {
        packet: PacketId::Unknown(160),
        dev_id: 0,
        size: 6
    })));
    assert!(events.contains(&Event::Mirror(MirrorEvent::ListCommitted {
        controllers: 2
    })));
    let names: Vec<String> = session
        .mirror()
        .controllers()
        .map(|c| c.description.display_name().to_string())
        .collect();
    assert_eq!(names, ["ENE DRAM", "Govee H6199"]);
}

#[test]
fn device_list_updates_before_the_version_reply_are_followed_by_one_resync() {
    // A client that connects while the server's detection is registering
    // controllers receives a DEVICE_LIST_UPDATED per controller before the
    // reply to its version request.
    let mut session = Session::new(config(ProtocolVersion::V6));
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::device_list_updated(),
            server::device_list_updated(),
            server::device_list_updated(),
            server::version_reply(6),
            server::server_name("OpenRGB"),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[1, 2, 3]),
            server::ok(0, PacketId::RequestControllerCount),
            server::server_flags(0x7f),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
        ],
    );
    assert_eq!(session.mirror().fault(), None);
    assert_eq!(
        ids(&sent(&mut session)),
        [
            (PacketId::SetClientFlags, 0),
            (PacketId::SetClientName, 0),
            (PacketId::RequestControllerData, 1),
            (PacketId::RequestControllerData, 2),
            (PacketId::RequestControllerData, 3)
        ]
    );

    feed(
        &mut session,
        &[
            server::controller_data(
                1,
                &testkit::ene_dram(ProtocolVersion::V6, 0),
                ProtocolVersion::V6,
            ),
            server::ok(1, PacketId::RequestControllerData),
            server::controller_data(
                2,
                &testkit::ene_dram(ProtocolVersion::V6, 1),
                ProtocolVersion::V6,
            ),
            server::ok(2, PacketId::RequestControllerData),
            server::controller_data(3, &testkit::govee(ProtocolVersion::V6), ProtocolVersion::V6),
            server::ok(3, PacketId::RequestControllerData),
        ],
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)],
        "the updates arrived during the first enumeration, so one resync follows"
    );

    feed(
        &mut session,
        &[
            server::count_v6(&[1, 2, 3]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert!(sent(&mut session).is_empty(), "the ids are unchanged");
    assert_eq!(session.mirror().phase(), Phase::Ready);
    assert_eq!(session.mirror().fault(), None);
    let events = session.drain_events();
    assert!(events.contains(&Event::Mirror(MirrorEvent::Negotiated {
        server_max: 6,
        protocol: ProtocolVersion::V6
    })));
    assert!(events.contains(&Event::Mirror(MirrorEvent::ListCommitted {
        controllers: 3
    })));
}

#[test]
fn a_count_reply_before_any_version_reply_means_protocol_zero() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    feed(&mut session, &[server::count_v5(3)]);
    assert_eq!(
        session.mirror().phase(),
        Phase::Faulted(Fault::ProtocolZero)
    );
    assert!(!session.should_close(), "the handshake is not flushed yet");
    sent(&mut session);
    assert!(session.should_close());
}

#[test]
fn a_server_below_protocol_five_is_unsupported() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    feed(
        &mut session,
        &[server::version_reply(4), server::count_v5(1)],
    );
    assert!(matches!(
        session.mirror().phase(),
        Phase::Faulted(Fault::Unsupported(UnsupportedProtocol { server_max: 4 }))
    ));
}

#[test]
fn v5_enumerates_every_index_then_a_count_fence() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    sent(&mut session);
    feed(&mut session, &[server::version_reply(5)]);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::SetClientName, 0)],
        "no client flags below protocol 6"
    );
    feed(&mut session, &[server::count_v5(2)]);
    let requests = sent(&mut session);
    assert_eq!(
        ids(&requests),
        [
            (PacketId::RequestControllerData, 0),
            (PacketId::RequestControllerData, 1),
            (PacketId::RequestControllerCount, 0)
        ]
    );
    assert_eq!(requests[0].payload, 5u32.to_le_bytes());
    feed(
        &mut session,
        &[
            server::controller_data(
                0,
                &testkit::ene_dram(ProtocolVersion::V5, 0),
                ProtocolVersion::V5,
            ),
            server::controller_data(1, &testkit::govee(ProtocolVersion::V5), ProtocolVersion::V5),
            server::count_v5(2),
        ],
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
    assert!(
        session
            .drain_events()
            .contains(&Event::Mirror(MirrorEvent::ListCommitted {
                controllers: 2
            }))
    );
}

#[test]
fn a_list_change_during_a_v5_resync_discards_its_results() {
    let mut session = Session::new(config(ProtocolVersion::V5));
    session.set_targets(targets(&[("dram", target("ENE DRAM", "Static", RED))]));
    sent(&mut session);
    feed(
        &mut session,
        &[server::version_reply(6), server::count_v5(2)],
    );
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::controller_data(
                0,
                &testkit::ene_dram(ProtocolVersion::V5, 0),
                ProtocolVersion::V5,
            ),
            server::device_list_updated(),
            server::controller_data(1, &testkit::govee(ProtocolVersion::V5), ProtocolVersion::V5),
            server::count_v5(2),
        ],
    );
    assert!(matches!(
        session.mirror().phase(),
        Phase::Enumerating { .. }
    ));
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)],
        "a fresh count, and no writes to indices that may have moved"
    );
    assert!(
        !session
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::Mirror(MirrorEvent::ListCommitted { .. })))
    );

    feed(&mut session, &[server::count_v5(1)]);
    assert_eq!(
        ids(&sent(&mut session)),
        [
            (PacketId::RequestControllerData, 0),
            (PacketId::RequestControllerCount, 0)
        ]
    );
    feed(
        &mut session,
        &[
            server::controller_data(
                0,
                &testkit::ene_dram(ProtocolVersion::V5, 0),
                ProtocolVersion::V5,
            ),
            server::count_v5(1),
        ],
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::UpdateMode, 0), (PacketId::UpdateLeds, 0)]
    );
}

#[test]
fn a_v5_data_reply_missing_before_the_fence_waits_for_the_list_notification() {
    let mut session = Session::new(config(ProtocolVersion::V5));
    sent(&mut session);
    feed(
        &mut session,
        &[server::version_reply(5), server::count_v5(2)],
    );
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::controller_data(
                0,
                &testkit::ene_dram(ProtocolVersion::V5, 0),
                ProtocolVersion::V5,
            ),
            server::count_v5(1),
        ],
    );
    assert!(
        session
            .drain_events()
            .contains(&Event::Mirror(MirrorEvent::ListInconsistent))
    );
    assert!(matches!(
        session.mirror().phase(),
        Phase::Enumerating { .. }
    ));
    assert!(
        sent(&mut session).is_empty(),
        "no request until the list notification"
    );

    feed(&mut session, &[server::device_list_updated()]);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)]
    );
}

#[test]
fn v6_ack_without_data_means_the_controller_vanished() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::version_reply(6),
            server::server_name("OpenRGB"),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[7, 9]),
            server::ok(0, PacketId::RequestControllerCount),
            server::server_flags(0),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
            server::controller_data(
                7,
                &testkit::ene_dram(ProtocolVersion::V6, 0),
                ProtocolVersion::V6,
            ),
            server::ok(7, PacketId::RequestControllerData),
            server::ok(9, PacketId::RequestControllerData),
        ],
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
    let listed: Vec<u32> = session.mirror().controllers().map(|c| c.dev_id).collect();
    assert_eq!(listed, [7]);
    assert!(
        session
            .drain_events()
            .contains(&Event::Mirror(MirrorEvent::ListCommitted {
                controllers: 1
            }))
    );
}

#[test]
fn v6_list_updates_diff_ids_and_request_only_new_ones() {
    let mut session = v6_ready(
        &[
            (7, testkit::ene_dram(ProtocolVersion::V6, 0)),
            (9, testkit::govee(ProtocolVersion::V6)),
        ],
        BTreeMap::new(),
    );
    session.drain_events();
    feed(&mut session, &[server::device_list_updated()]);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)]
    );
    feed(
        &mut session,
        &[
            server::count_v6(&[9, 12]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerData, 12)]
    );
    feed(
        &mut session,
        &[
            server::controller_data(
                12,
                &testkit::ene_dram(ProtocolVersion::V6, 0),
                ProtocolVersion::V6,
            ),
            server::ok(12, PacketId::RequestControllerData),
        ],
    );
    let events = session.drain_events();
    assert!(events.contains(&Event::Mirror(MirrorEvent::ControllerRemoved { dev_id: 7 })));
    assert!(events.contains(&Event::Mirror(MirrorEvent::ListCommitted {
        controllers: 2
    })));
    let listed: Vec<u32> = session.mirror().controllers().map(|c| c.dev_id).collect();
    assert_eq!(listed, [9, 12]);
}

#[test]
fn list_updates_during_a_resync_coalesce_into_exactly_one_more() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        BTreeMap::new(),
    );
    feed(&mut session, &[server::device_list_updated()]);
    assert_eq!(sent(&mut session).len(), 1);
    feed(
        &mut session,
        &[
            server::device_list_updated(),
            server::device_list_updated(),
            server::device_list_updated(),
        ],
    );
    assert!(sent(&mut session).is_empty(), "one resync at a time");
    feed(
        &mut session,
        &[
            server::count_v6(&[7]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)],
        "exactly one follow-up resync"
    );
    session.drain_events();
    feed(
        &mut session,
        &[
            server::count_v6(&[7]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert!(sent(&mut session).is_empty());
    assert_eq!(session.mirror().phase(), Phase::Ready);
    assert!(
        session
            .drain_events()
            .contains(&Event::Mirror(MirrorEvent::ListCommitted {
                controllers: 1
            }))
    );
}

#[test]
fn v6_apply_sends_mode_then_leds_then_reads_back_one_step_at_a_time() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ene dram", "static", RED))]),
    );
    let mode = sent(&mut session);
    assert_eq!(ids(&mode), [(PacketId::UpdateMode, 7)]);
    let (index, block) = update_mode_body(&mode[0], ProtocolVersion::V6);
    assert_eq!(index, 2);
    assert_eq!(block.name, WireString::from("Static"));
    assert_eq!(block.color_mode, ColorMode::PerLed);
    assert_eq!(session.device_reports()[0].state, DeviceState::Pending);

    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    let leds = sent(&mut session);
    assert_eq!(ids(&leds), [(PacketId::UpdateLeds, 7)]);
    assert_eq!(update_leds_body(&leds[0]), vec![RED; 8]);

    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerData, 7)]
    );

    session.drain_events();
    read_back(&mut session, 7, &applied_dram(RED));
    assert!(sent(&mut session).is_empty());
    assert_eq!(
        apply_states(&session.drain_events()),
        [("dram-rgb".to_owned(), 7, ApplyState::Confirmed)]
    );
    let report = &session.device_reports()[0];
    assert_eq!(report.state, DeviceState::Confirmed);
    assert_eq!(report.controllers[0].name, "ENE DRAM");
}

#[test]
fn every_matching_controller_is_selected() {
    let mut session = v6_ready(
        &[
            (7, testkit::ene_dram(ProtocolVersion::V6, 2)),
            (8, testkit::govee(ProtocolVersion::V6)),
            (9, testkit::ene_dram(ProtocolVersion::V6, 2)),
        ],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::UpdateLeds, 7), (PacketId::UpdateLeds, 9)]
    );
    for id in [7, 9] {
        feed(&mut session, &[server::ok(id, PacketId::UpdateLeds)]);
    }
    sent(&mut session);
    for id in [7, 9] {
        read_back(&mut session, id, &applied_dram(RED));
    }
    let report = &session.device_reports()[0];
    assert_eq!(report.state, DeviceState::Confirmed);
    assert_eq!(report.controllers.len(), 2);
}

#[test]
fn an_active_mode_skips_updatemode_unless_forced_and_leds_are_always_sent() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateLeds, 7)]);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(RED));

    // The same colour again: the LEDs are still written.
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]));
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateLeds, 7)]);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(RED));

    session.force_reapply();
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateMode, 7)]);
}

#[test]
fn a_changed_target_is_pending_until_applied_even_while_a_resync_defers_it() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(RED));
    assert_eq!(session.device_reports()[0].state, DeviceState::Confirmed);

    // A list change starts a resync; the new colour cannot be applied
    // before it commits, and the report must not keep RED's confirmation.
    feed(&mut session, &[server::device_list_updated()]);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)]
    );
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", BLUE))]));
    assert!(
        sent(&mut session).is_empty(),
        "the apply waits for the commit"
    );
    let report = &session.device_reports()[0];
    assert_eq!(report.state, DeviceState::Pending);
    assert_eq!(report.controllers[0].state, ApplyState::Pending);

    feed(
        &mut session,
        &[
            server::count_v6(&[7]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateLeds, 7)]);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(BLUE));
    assert_eq!(session.device_reports()[0].state, DeviceState::Confirmed);

    // During the next resync, the same target again keeps its confirmation.
    feed(&mut session, &[server::device_list_updated()]);
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", BLUE))]));
    assert_eq!(
        session.device_reports()[0].controllers[0].state,
        ApplyState::Confirmed
    );
}

#[test]
fn a_mode_specific_apply_reads_back_after_the_mode_ack() {
    let mut session = v6_ready(
        &[(9, testkit::govee(ProtocolVersion::V6))],
        targets(&[("govee", target("Govee", "Static", BLUE))]),
    );
    let mode = sent(&mut session);
    assert_eq!(ids(&mode), [(PacketId::UpdateMode, 9)]);
    let (index, block) = update_mode_body(&mode[0], ProtocolVersion::V6);
    assert_eq!(index, 1);
    assert_eq!(block.color_mode, ColorMode::ModeSpecific);
    assert_eq!(block.colors, vec![BLUE]);
    assert_eq!(block.brightness, 100);

    feed(&mut session, &[server::ok(9, PacketId::UpdateMode)]);
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerData, 9)],
        "no UPDATELEDS on the mode-specific path"
    );
    let mut applied = testkit::govee(ProtocolVersion::V6);
    applied.active_mode = 1;
    applied.modes[1].colors = vec![BLUE];
    read_back(&mut session, 9, &applied);
    assert_eq!(session.device_reports()[0].state, DeviceState::Confirmed);
}

#[test]
fn a_non_ok_ack_is_an_error_naming_the_status() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    session.drain_events();
    feed(
        &mut session,
        &[server::ack(
            7,
            PacketId::UpdateMode,
            AckStatus::ErrorInvalidData,
        )],
    );
    assert!(sent(&mut session).is_empty());
    let state = ApplyState::Rejected {
        packet: PacketId::UpdateMode,
        status: AckStatus::ErrorInvalidData,
    };
    assert_eq!(
        apply_states(&session.drain_events()),
        [("dram-rgb".to_owned(), 7, state.clone())]
    );
    assert_eq!(
        state.to_string(),
        "OpenRGB answered UpdateMode (1101) with ERROR_INVALID_DATA"
    );
    assert_eq!(session.device_reports()[0].state, DeviceState::Error);
}

#[test]
fn an_invalid_id_ack_means_the_controller_vanished_mid_write() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    session.drain_events();
    feed(
        &mut session,
        &[server::ack(
            7,
            PacketId::UpdateMode,
            AckStatus::ErrorInvalidId,
        )],
    );
    assert_eq!(
        apply_states(&session.drain_events()),
        [("dram-rgb".to_owned(), 7, ApplyState::Vanished)]
    );
}

#[test]
fn a_write_acknowledged_without_being_applied_is_caught_by_the_read_back() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    session.drain_events();
    // The server no longer lists 7: no data, then the ACK.
    feed(
        &mut session,
        &[server::ok(7, PacketId::RequestControllerData)],
    );
    assert_eq!(
        apply_states(&session.drain_events()),
        [("dram-rgb".to_owned(), 7, ApplyState::Vanished)]
    );
}

#[test]
fn a_read_back_mismatch_names_what_the_server_holds() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    session.drain_events();
    // The server kept mode 0: it dropped the mode update.
    let mut held = testkit::ene_dram(ProtocolVersion::V6, 0);
    held.colors = vec![RED; 8];
    read_back(&mut session, 7, &held);
    let states = apply_states(&session.drain_events());
    let [(_, _, ApplyState::Mismatch(mismatch))] = states.as_slice() else {
        panic!("expected one mismatch, got {states:?}");
    };
    assert_eq!(
        mismatch.to_string(),
        "server holds active mode 0 (\"Direct\"), expected 2 (\"Static\")"
    );
    assert!(matches!(mismatch, Mismatch::ActiveMode { held: 0, .. }));
    assert_eq!(session.device_reports()[0].state, DeviceState::Error);
}

#[test]
fn acknowledgements_from_an_older_list_generation_are_discarded() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateMode, 7)]);
    let generation = session.mirror().generation();
    feed(&mut session, &[server::device_list_updated()]);
    assert_eq!(session.mirror().generation(), generation + 1);
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::count_v6(&[]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    session.drain_events();
    assert!(!session.should_close());
    // The write queued before 7 left the list is acknowledged afterwards.
    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    let events = session.drain_events();
    assert!(events.contains(&Event::StaleDiscarded {
        dev_id: 7,
        packet: PacketId::UpdateMode
    }));
    assert!(apply_states(&events).is_empty());
    assert!(
        sent(&mut session).is_empty(),
        "no UPDATELEDS for a removed controller"
    );
    assert_eq!(session.device_reports()[0].state, DeviceState::Absent);
}

#[test]
fn a_read_back_for_a_removed_controller_is_discarded() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateLeds, 7)]);
    feed(&mut session, &[server::device_list_updated()]);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    assert_eq!(
        ids(&sent(&mut session)),
        [
            (PacketId::RequestControllerCount, 0),
            (PacketId::RequestControllerData, 7)
        ]
    );
    feed(
        &mut session,
        &[
            server::count_v6(&[]),
            server::ok(0, PacketId::RequestControllerCount),
            server::ok(7, PacketId::RequestControllerData),
        ],
    );
    let events = session.drain_events();
    assert!(events.contains(&Event::StaleDiscarded {
        dev_id: 7,
        packet: PacketId::RequestControllerData
    }));
    assert!(
        !apply_states(&events)
            .iter()
            .any(|(_, _, state)| *state == ApplyState::Vanished)
    );
}

#[test]
fn unknown_packets_are_skipped_by_their_size() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        BTreeMap::new(),
    );
    session.drain_events();
    let mut stream = server::frame(3, PacketId::Unknown(304), vec![0xab; 12]);
    stream.extend(server::device_list_updated());
    session.receive(&stream);
    let events = session.drain_events();
    assert_eq!(
        events[0],
        Event::Mirror(MirrorEvent::Skipped {
            packet: PacketId::Unknown(304),
            dev_id: 3,
            size: 12
        })
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerCount, 0)]
    );
}

#[test]
fn a_bad_magic_faults_and_closes_at_once() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    session.receive(b"XRGB\0\0\0\0\0\0\0\0\0\0\0\0");
    assert!(matches!(
        session.mirror().phase(),
        Phase::Faulted(Fault::Framing(_))
    ));
    assert!(
        session.should_close(),
        "nothing after a framing error can be matched"
    );
    session.force_reapply();
    assert!(
        !session.pending_output().is_empty(),
        "the UPDATEMODE queued before the fault is still pending"
    );
}

#[test]
fn a_decode_fault_holds_the_close_until_outstanding_writes_are_acknowledged() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    // A controller-data reply nobody asked for.
    feed(
        &mut session,
        &[server::controller_data(
            7,
            &testkit::ene_dram(ProtocolVersion::V6, 0),
            ProtocolVersion::V6,
        )],
    );
    assert!(matches!(
        session.mirror().phase(),
        Phase::Faulted(Fault::Unexpected { .. })
    ));
    assert!(
        !session.should_close(),
        "the UPDATEMODE is still unacknowledged"
    );
    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    assert!(session.should_close());
    assert!(sent(&mut session).is_empty(), "no UPDATELEDS after a fault");
}

#[test]
fn an_ack_for_a_write_never_sent_is_a_fault() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        BTreeMap::new(),
    );
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    assert_eq!(
        session.mirror().phase(),
        Phase::Faulted(Fault::UnexpectedAck {
            dev_id: 7,
            acked: PacketId::UpdateLeds
        })
    );
}

#[test]
fn shutdown_at_v6_drains_outstanding_acks_before_closing() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    session.begin_shutdown();
    assert!(!session.should_close(), "the UPDATEMODE is not flushed");
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateMode, 7)]);
    assert!(
        !session.should_close(),
        "the UPDATEMODE is not acknowledged"
    );
    feed(&mut session, &[server::device_list_updated()]);
    assert!(
        sent(&mut session).is_empty(),
        "no resync while shutting down"
    );
    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    assert!(
        sent(&mut session).is_empty(),
        "the apply stops at the acknowledged write"
    );
    assert!(session.should_close());
}

#[test]
fn shutdown_at_v6_waits_for_a_read_back_in_flight() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    session.begin_shutdown();
    assert!(!session.should_close());
    read_back(&mut session, 7, &applied_dram(RED));
    assert!(session.should_close());
}

#[test]
fn shutdown_at_v5_closes_once_the_last_send_is_flushed() {
    let mut session = v5_ready(
        &[testkit::ene_dram(ProtocolVersion::V5, 0)],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    session.begin_shutdown();
    assert!(!session.should_close());
    let bytes = session.pending_output().len();
    session.consume_output(bytes - 1);
    assert!(
        !session.should_close(),
        "one byte of the last write is unsent"
    );
    session.consume_output(1);
    assert!(session.should_close());
}

#[test]
fn v5_writes_go_out_back_to_back_echoing_the_mode_value() {
    let mut session = v5_ready(
        &[testkit::ene_dram(ProtocolVersion::V5, 0)],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    let writes = sent(&mut session);
    assert_eq!(
        ids(&writes),
        [(PacketId::UpdateMode, 0), (PacketId::UpdateLeds, 0)]
    );
    let (index, block) = update_mode_body(&writes[0], ProtocolVersion::V5);
    assert_eq!(index, 2);
    assert_eq!(block.value, Some(0), "the mode value the server sent");
    assert_eq!(update_leds_body(&writes[1]), vec![RED; 8]);
    assert_eq!(session.device_reports()[0].state, DeviceState::Sent);
    assert_eq!(
        apply_states(&session.drain_events()),
        [("dram-rgb".to_owned(), 0, ApplyState::Sent)]
    );
}

#[test]
fn a_reconcile_during_a_busy_apply_reruns_once_with_the_latest_target() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateMode, 7)]);
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", BLUE))]));
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", BLUE))]));
    assert!(
        sent(&mut session).is_empty(),
        "one apply per controller at a time"
    );

    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    assert_eq!(update_leds_body(&sent(&mut session)[0]), vec![RED; 8]);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(RED));
    let rerun = sent(&mut session);
    assert_eq!(
        ids(&rerun),
        [(PacketId::UpdateLeds, 7)],
        "the read-back shows Static active, so only the LEDs change"
    );
    assert_eq!(update_leds_body(&rerun[0]), vec![BLUE; 8]);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(BLUE));
    assert!(sent(&mut session).is_empty(), "exactly one rerun");
    assert_eq!(session.device_reports()[0].state, DeviceState::Confirmed);
}

#[test]
fn detection_complete_forces_a_reapply_and_reports_absent_targets() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[
            ("dram-rgb", target("ENE DRAM", "Static", RED)),
            ("keychron-k2-he", target("Keychron K2 HE", "Static", RED)),
        ]),
    );
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(RED));
    session.drain_events();

    feed(&mut session, &[server::detection_complete()]);
    let events = session.drain_events();
    assert!(events.contains(&Event::AbsentAtDetectionComplete {
        label: "keychron-k2-he".into(),
        name_contains: "Keychron K2 HE".into(),
        available: vec!["ENE DRAM".into()],
    }));
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::UpdateMode, 7)],
        "forced: the active mode is written again"
    );
    let reports = session.device_reports();
    assert_eq!(reports[1].label, "keychron-k2-he");
    assert_eq!(reports[1].state, DeviceState::Absent);

    // The absence report belongs to that DETECTION_COMPLETE only.
    feed(&mut session, &[server::ok(7, PacketId::UpdateMode)]);
    sent(&mut session);
    feed(&mut session, &[server::ok(7, PacketId::UpdateLeds)]);
    sent(&mut session);
    read_back(&mut session, 7, &applied_dram(RED));
    session.set_targets(targets(&[
        ("dram-rgb", target("ENE DRAM", "Static", RED)),
        ("keychron-k2-he", target("Keychron K2 HE", "Static", BLUE)),
    ]));
    assert!(
        !session
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::AbsentAtDetectionComplete { .. }))
    );
}

#[test]
fn detection_complete_during_a_resync_waits_for_the_commit() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("keychron", target("Keychron", "Static", RED))]),
    );
    session.drain_events();
    feed(
        &mut session,
        &[server::device_list_updated(), server::detection_complete()],
    );
    assert!(
        !session
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::AbsentAtDetectionComplete { .. }))
    );
    feed(
        &mut session,
        &[
            server::count_v6(&[7]),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert!(
        session
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::AbsentAtDetectionComplete { .. }))
    );
}

#[test]
fn presence_is_reported_on_transitions() {
    let mut session = v5_ready(
        &[],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    let events = session.drain_events();
    assert!(events.contains(&Event::Presence {
        label: "dram-rgb".into(),
        present: false,
        available: Vec::new(),
    }));
    assert_eq!(session.device_reports()[0].state, DeviceState::Absent);

    feed(&mut session, &[server::device_list_updated()]);
    sent(&mut session);
    feed(&mut session, &[server::count_v5(1)]);
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::controller_data(
                0,
                &testkit::ene_dram(ProtocolVersion::V5, 0),
                ProtocolVersion::V5,
            ),
            server::count_v5(1),
        ],
    );
    assert!(session.drain_events().contains(&Event::Presence {
        label: "dram-rgb".into(),
        present: true,
        available: vec!["ENE DRAM".into()],
    }));

    session.force_reapply();
    assert!(
        !session
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::Presence { .. }))
    );
}

#[test]
fn a_controller_selected_by_two_specs_goes_to_the_first_and_conflicts_for_the_second() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[
            ("a-dram", target("DRAM", "Static", RED)),
            ("b-ene", target("ENE", "Static", BLUE)),
        ]),
    );
    let writes = sent(&mut session);
    assert_eq!(ids(&writes), [(PacketId::UpdateLeds, 7)]);
    assert_eq!(update_leds_body(&writes[0]), vec![RED; 8]);
    let reports = session.device_reports();
    assert_eq!(reports[1].state, DeviceState::Error);
    assert_eq!(
        reports[1].controllers[0].state,
        ApplyState::Conflict {
            claimed_by: "a-dram".into()
        }
    );
}

#[test]
fn signal_update_refreshes_the_cache_only() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        BTreeMap::new(),
    );
    session.drain_events();
    feed(
        &mut session,
        &[
            server::signal_update_device(
                7,
                UpdateReason::UpdateMode,
                &applied_dram(BLUE),
                ProtocolVersion::V6,
            ),
            server::signal_update_leds(7, &[RED; 8]),
        ],
    );
    assert!(sent(&mut session).is_empty());
    let cached = &session.mirror().controller(7).unwrap().description;
    assert_eq!(cached.active_mode, 2);
    assert_eq!(cached.colors, vec![RED; 8]);
    assert!(
        session
            .drain_events()
            .contains(&Event::Mirror(MirrorEvent::CacheRefreshed {
                dev_id: 7,
                reason: UpdateReason::UpdateLeds
            }))
    );

    // The refreshed cache shows Static active, so an apply skips the mode.
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", BLUE))]));
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateLeds, 7)]);
}

#[test]
fn a_mode_update_the_server_would_drop_is_reported_once_and_not_sent() {
    let mut dram = testkit::ene_dram(ProtocolVersion::V6, 0);
    dram.modes[2].brightness = 7;
    let mut session = v6_ready(
        &[(7, dram)],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    assert!(sent(&mut session).is_empty());
    let states = apply_states(&session.drain_events());
    assert_eq!(
        states,
        [(
            "dram-rgb".to_owned(),
            7,
            ApplyState::NotPlanned(PlanError::ServerWouldReject {
                mode: "Static".into(),
                clause: RejectClause::Brightness {
                    value: 7,
                    min: 0,
                    max: 0
                }
            })
        )]
    );
    session.force_reapply();
    assert!(sent(&mut session).is_empty());
    assert!(
        apply_states(&session.drain_events()).is_empty(),
        "reported once"
    );
    assert_eq!(session.device_reports()[0].state, DeviceState::Error);
}

#[test]
fn an_unknown_mode_is_an_error_listing_the_modes() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Rainbow", RED))]),
    );
    assert!(sent(&mut session).is_empty(), "no fallback to another mode");
    let report = &session.device_reports()[0];
    assert_eq!(report.state, DeviceState::Error);
    assert_eq!(
        report.controllers[0].state.to_string(),
        "no mode named \"Rainbow\" (modes: Direct, Off, Static, Breathing)"
    );
}

#[test]
fn targets_set_before_the_list_is_committed_apply_at_the_commit() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    session.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]));
    assert_eq!(session.device_reports()[0].state, DeviceState::Waiting);
    assert_eq!(sent(&mut session).len(), 2, "only the handshake");
    feed(
        &mut session,
        &[
            server::version_reply(6),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[7]),
            server::ok(0, PacketId::RequestControllerCount),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
        ],
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [
            (PacketId::SetClientFlags, 0),
            (PacketId::SetClientName, 0),
            (PacketId::RequestControllerData, 7)
        ]
    );
    read_back(&mut session, 7, &testkit::ene_dram(ProtocolVersion::V6, 0));
    assert_eq!(ids(&sent(&mut session)), [(PacketId::UpdateMode, 7)]);
}

#[test]
fn a_v6_session_capped_at_five_speaks_protocol_five() {
    let mut session = Session::new(config(ProtocolVersion::V5));
    let hello = sent(&mut session);
    assert_eq!(hello[0].payload, 5u32.to_le_bytes());
    feed(&mut session, &[server::version_reply(6)]);
    assert_eq!(session.protocol(), Some(ProtocolVersion::V5));
    assert_eq!(ids(&sent(&mut session)), [(PacketId::SetClientName, 0)]);
}

#[test]
fn an_ack_that_answers_nothing_outstanding_is_a_fault() {
    let mut session = v6_ready(&[], BTreeMap::new());
    feed(
        &mut session,
        &[server::ok(0, PacketId::RequestControllerCount)],
    );
    assert_eq!(
        session.mirror().phase(),
        Phase::Faulted(Fault::UnexpectedAck {
            dev_id: 0,
            acked: PacketId::RequestControllerCount
        })
    );
}

#[test]
fn a_rejected_client_flags_request_is_reported_not_fatal() {
    let mut session = Session::new(config(ProtocolVersion::V6));
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::version_reply(6),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[]),
            server::ok(0, PacketId::RequestControllerCount),
            server::ack(0, PacketId::SetClientFlags, AckStatus::ErrorInvalidData),
            server::ok(0, PacketId::SetClientName),
        ],
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
    assert!(
        session
            .drain_events()
            .contains(&Event::Mirror(MirrorEvent::RequestRejected {
                packet: PacketId::SetClientFlags,
                dev_id: 0,
                status: AckStatus::ErrorInvalidData
            }))
    );
}

#[test]
fn byte_at_a_time_delivery_reaches_the_same_state() {
    let mut whole = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    let mut trickled = Session::new(config(ProtocolVersion::V6));
    trickled.set_targets(targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]));
    let script = [
        server::version_reply(6),
        server::server_name("OpenRGB"),
        server::ok(0, PacketId::RequestProtocolVersion),
        server::count_v6(&[7]),
        server::ok(0, PacketId::RequestControllerCount),
        server::server_flags(0x7f),
        server::ok(0, PacketId::SetClientFlags),
        server::ok(0, PacketId::SetClientName),
        server::controller_data(
            7,
            &testkit::ene_dram(ProtocolVersion::V6, 0),
            ProtocolVersion::V6,
        ),
        server::ok(7, PacketId::RequestControllerData),
    ]
    .concat();
    for byte in script {
        trickled.receive(&[byte]);
    }
    let trickled_frames = sent(&mut trickled);
    let whole_writes = sent(&mut whole);
    assert_eq!(trickled.mirror().phase(), Phase::Ready);
    assert_eq!(whole.device_reports(), trickled.device_reports());
    assert_eq!(ids(&whole_writes), [(PacketId::UpdateMode, 7)]);
    assert_eq!(
        ids(&trickled_frames),
        [
            (PacketId::RequestProtocolVersion, 0),
            (PacketId::RequestControllerCount, 0),
            (PacketId::SetClientFlags, 0),
            (PacketId::SetClientName, 0),
            (PacketId::RequestControllerData, 7),
            (PacketId::UpdateMode, 7)
        ]
    );
    assert_eq!(trickled_frames[5], whole_writes[0]);
}

#[test]
fn ack_bodies_route_by_acknowledged_packet() {
    // A write ACK and an in-order ACK for the same device are told apart by the
    // acknowledged packet id, not by arrival order.
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 2))],
        targets(&[("dram-rgb", target("ENE DRAM", "Static", RED))]),
    );
    sent(&mut session);
    feed(&mut session, &[server::device_list_updated()]);
    sent(&mut session);
    feed(
        &mut session,
        &[
            server::count_v6(&[7]),
            server::ok(7, PacketId::UpdateLeds),
            server::ok(0, PacketId::RequestControllerCount),
        ],
    );
    assert_eq!(
        ids(&sent(&mut session)),
        [(PacketId::RequestControllerData, 7)]
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
}

#[test]
fn detection_notifications_are_reported_without_changing_state() {
    let mut session = v6_ready(
        &[(7, testkit::ene_dram(ProtocolVersion::V6, 0))],
        BTreeMap::new(),
    );
    session.drain_events();
    feed(
        &mut session,
        &[
            server::detection_started(),
            server::detection_progress(40, "ENE SMBus DRAM"),
        ],
    );
    assert_eq!(
        session.drain_events(),
        [
            Event::Mirror(MirrorEvent::DetectionStarted),
            Event::Mirror(MirrorEvent::DetectionProgress {
                percent: 40,
                text: WireString::from("ENE SMBus DRAM")
            })
        ]
    );
    assert_eq!(session.mirror().phase(), Phase::Ready);
    assert!(sent(&mut session).is_empty());
}

#[test]
fn a_mirror_on_its_own_enumerates_and_never_writes() {
    let mut mirror = Mirror::new(config(ProtocolVersion::V6));
    let dram = testkit::ene_dram(ProtocolVersion::V6, 0);
    mirror.receive(
        &[
            server::version_reply(6),
            server::server_name("OpenRGB"),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[7]),
            server::ok(0, PacketId::RequestControllerCount),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
            server::controller_data(7, &dram, ProtocolVersion::V6),
            server::ok(7, PacketId::RequestControllerData),
            server::detection_complete(),
        ]
        .concat(),
    );
    let events = mirror.drain_events();
    assert!(events.contains(&MirrorEvent::ListCommitted { controllers: 1 }));
    assert!(events.contains(&MirrorEvent::DetectionComplete));
    assert_eq!(mirror.phase(), Phase::Ready);

    let bytes = mirror.pending_output().to_vec();
    mirror.consume_output(bytes.len());
    let mut framer = Framer::new();
    framer.push(&bytes);
    while let Some(frame) = framer.next_frame().unwrap() {
        assert!(
            !matches!(frame.pkt_id, PacketId::UpdateLeds | PacketId::UpdateMode),
            "a mirror sends no writes"
        );
    }
    let controller = mirror.controllers().next().unwrap();
    assert_eq!(controller.description, dram);
    assert_eq!(
        controller.raw,
        testkit::controller_data_payload(&dram, ProtocolVersion::V6),
        "the raw reply payload is kept"
    );

    mirror.begin_shutdown();
    assert!(mirror.should_close());
}
