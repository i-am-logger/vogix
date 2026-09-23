//! The read side of one OpenRGB SDK connection: protocol negotiation, the
//! controller list, and a cache of every controller's description.
//!
//! A [`Mirror`] is a pure state machine. Bytes read from the socket go in
//! through [`Mirror::receive`] (or [`Mirror::feed`] and
//! [`Mirror::process_next`]); frames to send come out of
//! [`Mirror::pending_output`]; what happened comes out as [`MirrorEvent`]s.
//! It performs no I/O and never waits on time: every transition is driven by a
//! frame from the server.
//!
//! # Correlating replies
//!
//! The server answers one client's requests in order on that client's listen
//! thread (`NetworkServer::ListenThreadFunction`): the reply to a request (if
//! any), then, at protocol 6, an `ACK`. Other frames — `DEVICE_LIST_UPDATED`,
//! detection notifications, `SIGNALUPDATE` and the `ACK`s of queued writes —
//! come from other server threads and may fall between them, but never inside
//! one frame, because every send holds the server's `send_in_progress` lock. So
//! the mirror keeps its outstanding requests in a FIFO and matches each reply
//! and in-order `ACK` against its head:
//!
//! - The handshake pipelines `REQUEST_PROTOCOL_VERSION` and a
//!   `REQUEST_CONTROLLER_COUNT` fence. A count reply ahead of any version reply
//!   means a protocol 0 server, which never answers the version request.
//! - At protocol 6, `REQUEST_CONTROLLER_DATA` for an id the server no longer
//!   lists gets no reply but still an `ACK` with status OK
//!   (`SendReply_ControllerData` sends nothing for an invalid id), so an `ACK`
//!   without a preceding reply means the controller vanished.
//! - At protocol 5 nothing is acknowledged, so enumeration requests the data of
//!   every index and then a trailing count as a fence: a data reply missing
//!   before the fence means the list changed.
//!
//! # Resync
//!
//! One resync runs at a time. Every `DEVICE_LIST_UPDATED` bumps the list
//! generation; one arriving during a resync sets `resync_pending`, so exactly one
//! more resync follows however many arrived. At protocol 6 ids are stable, so a
//! resync diffs them and requests data only for new ids. At protocol 5 indices
//! are not stable: the list is unusable from the moment a `DEVICE_LIST_UPDATED`
//! arrives until a resync that saw no list change during it commits, and the
//! results of a resync that did see one are discarded. Every list change the
//! server makes is followed by a `DEVICE_LIST_UPDATED` (the resource manager
//! signals it after `SetControllers`), so a protocol 5 resync that found the
//! list inconsistent without one waits for that notification instead of
//! repeating itself.
//!
//! # Writes and closing
//!
//! The session layer sends `UPDATEMODE` and `UPDATELEDS` through the mirror,
//! which tracks each one until the server acknowledges it (protocol 6). The
//! server queues a write to the controller's own thread and acknowledges it from
//! there after applying it, so closing the socket with a write outstanding would
//! drop the connection under a queued write. [`Mirror::should_close`] therefore
//! holds the close, after [`Mirror::begin_shutdown`] or a fault, until the
//! output is flushed and — at protocol 6 — every outstanding acknowledgement has
//! arrived. At protocol 5 there are no acknowledgements, so the close waits
//! only for the flush. A stream that can no longer be framed closes at once.

use super::super::codec::{self, ControllerList, SignalUpdateBody};
use super::super::model::{
    AckStatus, ClientFlags, ClientName, ControllerDescription, ProtocolVersion, ServerFlags,
    UnsupportedProtocol, UpdateReason,
};
use super::super::wire::{
    DecodeError, Frame, Framer, FramingError, MAX_PACKET_SIZE, PacketId, WireString,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The most controllers one protocol 6 count reply can list; a protocol 5 count
/// above it is treated as corrupt rather than turned into that many requests.
pub const MAX_CONTROLLERS: u32 = (MAX_PACKET_SIZE - 4) / 4;

/// How the client introduces itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorConfig {
    /// The highest protocol version vogix offers.
    pub max_protocol: ProtocolVersion,
    pub client_name: ClientName,
}

/// One controller as the server last described it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controller {
    /// The `pkt_dev_id` that addresses it: a list index at protocol 5, a unique
    /// id at protocol 6.
    pub dev_id: u32,
    pub description: ControllerDescription,
    /// The payload of the `REQUEST_CONTROLLER_DATA` reply it was decoded from.
    pub raw: Vec<u8>,
}

/// Where the connection stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for the protocol version reply.
    Handshaking,
    /// A resync is running, or the protocol 5 list is stale.
    Enumerating { generation: u64 },
    /// The controller list is current and no resync runs.
    Ready,
    /// The server violated the protocol; nothing more is requested.
    Faulted(Fault),
}

/// A protocol violation vogix detected. These are deterministic: the same
/// server state produces them again, so they are reported rather than retried.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("the OpenRGB byte stream cannot be framed: {0}")]
    Framing(FramingError),
    #[error(
        "OpenRGB answered the controller count without answering the protocol version request, \
         so it speaks SDK protocol 0; vogix speaks protocols 5 and 6"
    )]
    ProtocolZero,
    #[error(transparent)]
    Unsupported(UnsupportedProtocol),
    #[error("malformed {packet} for device {dev_id} from OpenRGB: {error}")]
    Decode {
        packet: PacketId,
        dev_id: u32,
        error: DecodeError,
    },
    #[error("unexpected {packet} for device {dev_id} from OpenRGB: {detail}")]
    Unexpected {
        packet: PacketId,
        dev_id: u32,
        detail: &'static str,
    },
    #[error("OpenRGB acknowledged {acked} for device {dev_id}, which vogix has not sent")]
    UnexpectedAck { dev_id: u32, acked: PacketId },
    #[error("OpenRGB reports {count} controllers, more than the {max} one SDK packet can list")]
    ControllerCountTooLarge { count: u32, max: u32 },
}

impl Fault {
    /// Frames can still be delimited after this fault, so acknowledgements of
    /// outstanding writes can still be matched.
    pub fn stream_intact(&self) -> bool {
        !matches!(self, Self::Framing(_))
    }
}

/// Identifies one read-back request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadBackToken(pub(super) u64);

/// What the mirror observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorEvent {
    /// The protocol version is agreed.
    Negotiated {
        server_max: u32,
        protocol: ProtocolVersion,
    },
    ServerName(WireString),
    ServerFlags(ServerFlags),
    /// A resync finished with no further list change pending.
    ListCommitted {
        controllers: usize,
    },
    /// A protocol 6 controller left the list.
    ControllerRemoved {
        dev_id: u32,
    },
    /// A protocol 5 resync saw the list change; the next
    /// `DEVICE_LIST_UPDATED` starts another.
    ListInconsistent,
    DetectionStarted,
    DetectionProgress {
        percent: u32,
        text: WireString,
    },
    DetectionComplete,
    /// A `SIGNALUPDATE` refreshed the cached description.
    CacheRefreshed {
        dev_id: u32,
        reason: UpdateReason,
    },
    /// A frame vogix does not act on, skipped by its size.
    Skipped {
        packet: PacketId,
        dev_id: u32,
        size: usize,
    },
    /// An in-order request was acknowledged with a non-OK status.
    RequestRejected {
        packet: PacketId,
        dev_id: u32,
        status: AckStatus,
    },
    /// A write sent through [`Mirror::send_write`] was acknowledged.
    WriteAck {
        dev_id: u32,
        packet: PacketId,
        status: AckStatus,
    },
    /// A read-back finished; `found` is false when the server sent no data
    /// because the controller is gone.
    ReadBack {
        dev_id: u32,
        token: ReadBackToken,
        found: bool,
    },
    Faulted(Fault),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CountRole {
    /// Starts a resync.
    Seed,
    /// Ends a protocol 5 resync.
    V5Fence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataRole {
    Enumerate,
    ReadBack(ReadBackToken),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    Version,
    Count(CountRole),
    Data { dev_id: u32, role: DataRole },
    ClientName,
    ClientFlags,
}

impl Request {
    fn packet(self) -> PacketId {
        match self {
            Self::Version => PacketId::RequestProtocolVersion,
            Self::Count(_) => PacketId::RequestControllerCount,
            Self::Data { .. } => PacketId::RequestControllerData,
            Self::ClientName => PacketId::SetClientName,
            Self::ClientFlags => PacketId::SetClientFlags,
        }
    }

    fn dev_id(self) -> u32 {
        match self {
            Self::Data { dev_id, .. } => dev_id,
            _ => 0,
        }
    }

    /// The server always replies to this request before acknowledging it.
    fn reply_required(self) -> bool {
        matches!(self, Self::Version | Self::Count(_))
    }

    /// The reply packet this request can receive.
    fn reply(self) -> Option<PacketId> {
        match self {
            Self::Version | Self::Count(_) | Self::Data { .. } => Some(self.packet()),
            Self::ClientName | Self::ClientFlags => None,
        }
    }
}

/// Whether an `ACK` follows the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AckExpect {
    /// Sent before the protocol version was known.
    Undetermined,
    Awaited,
    None,
}

impl AckExpect {
    fn for_protocol(protocol: ProtocolVersion) -> Self {
        match protocol {
            ProtocolVersion::V5 => Self::None,
            ProtocolVersion::V6 => Self::Awaited,
        }
    }
}

#[derive(Debug)]
struct InOrder {
    request: Request,
    replied: bool,
    ack: AckExpect,
}

#[derive(Debug)]
enum Resync {
    Idle,
    /// The seed count is outstanding.
    Seeding {
        started: u64,
    },
    /// Protocol 6: data requests for new ids are outstanding.
    V6 {
        outstanding: usize,
    },
    /// Protocol 5: data for every index and the trailing fence are outstanding.
    V5 {
        started: u64,
        seed: u32,
        received: BTreeMap<u32, Controller>,
        missing: bool,
    },
}

/// Bytes queued for the socket.
#[derive(Debug, Default)]
struct Outbox {
    bytes: Vec<u8>,
    sent: usize,
}

impl Outbox {
    fn push(&mut self, frame: &[u8]) {
        self.bytes.extend_from_slice(frame);
    }

    fn pending(&self) -> &[u8] {
        &self.bytes[self.sent..]
    }

    fn is_empty(&self) -> bool {
        self.sent == self.bytes.len()
    }

    fn consume(&mut self, written: usize) {
        self.sent += written.min(self.bytes.len() - self.sent);
        if self.sent == self.bytes.len() {
            self.bytes.clear();
            self.sent = 0;
        } else if self.sent >= self.bytes.len() / 2 {
            self.bytes.drain(..self.sent);
            self.sent = 0;
        }
    }
}

/// The client's mirror of the server's controller list.
#[derive(Debug)]
pub struct Mirror {
    config: MirrorConfig,
    framer: Framer,
    outbox: Outbox,
    protocol: Option<ProtocolVersion>,
    server_name: Option<WireString>,
    inorder: VecDeque<InOrder>,
    /// Addresses in server list order; at protocol 6 this can hold ids whose
    /// data is still being requested.
    order: Vec<u32>,
    controllers: BTreeMap<u32, Controller>,
    /// The cached list can be addressed: false before the first commit and, at
    /// protocol 5, from a list change until the next commit.
    committed: bool,
    ever_committed: bool,
    generation: u64,
    resync: Resync,
    resync_pending: bool,
    /// Outstanding writes by (device, packet id), protocol 6 only.
    writes: BTreeMap<(u32, u32), usize>,
    fault: Option<Fault>,
    shutting_down: bool,
    events: VecDeque<MirrorEvent>,
}

impl Mirror {
    /// A mirror whose output starts with the handshake:
    /// `REQUEST_PROTOCOL_VERSION` then a `REQUEST_CONTROLLER_COUNT` fence.
    pub fn new(config: MirrorConfig) -> Self {
        let mut mirror = Self {
            config,
            framer: Framer::new(),
            outbox: Outbox::default(),
            protocol: None,
            server_name: None,
            inorder: VecDeque::new(),
            order: Vec::new(),
            controllers: BTreeMap::new(),
            committed: false,
            ever_committed: false,
            generation: 0,
            resync: Resync::Seeding { started: 0 },
            resync_pending: false,
            writes: BTreeMap::new(),
            fault: None,
            shutting_down: false,
            events: VecDeque::new(),
        };
        mirror
            .outbox
            .push(&codec::request_protocol_version(mirror.config.max_protocol));
        mirror.inorder.push_back(InOrder {
            request: Request::Version,
            replied: false,
            ack: AckExpect::Undetermined,
        });
        mirror.outbox.push(&codec::request_controller_count());
        mirror.inorder.push_back(InOrder {
            request: Request::Count(CountRole::Seed),
            replied: false,
            ack: AckExpect::Undetermined,
        });
        mirror
    }

    /// Bytes to write to the socket, oldest first.
    pub fn pending_output(&self) -> &[u8] {
        self.outbox.pending()
    }

    /// Record that the first `written` bytes of [`Mirror::pending_output`]
    /// reached the socket.
    pub fn consume_output(&mut self, written: usize) {
        self.outbox.consume(written);
    }

    /// Feed bytes read from the socket and process every complete frame.
    pub fn receive(&mut self, bytes: &[u8]) {
        self.feed(bytes);
        while self.process_next() {}
    }

    /// Append bytes read from the socket without processing them.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.framer.push(bytes);
    }

    /// Process one complete frame; false when none is buffered or the stream
    /// can no longer be framed.
    pub fn process_next(&mut self) -> bool {
        if matches!(self.fault, Some(Fault::Framing(_))) {
            return false;
        }
        match self.framer.next_frame() {
            Ok(Some(frame)) => {
                self.handle(frame);
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.fail(Fault::Framing(error));
                false
            }
        }
    }

    /// The next event, oldest first.
    pub fn pop_event(&mut self) -> Option<MirrorEvent> {
        self.events.pop_front()
    }

    /// Every pending event, oldest first.
    #[cfg(test)]
    pub fn drain_events(&mut self) -> Vec<MirrorEvent> {
        self.events.drain(..).collect()
    }

    pub fn phase(&self) -> Phase {
        if let Some(fault) = &self.fault {
            return Phase::Faulted(fault.clone());
        }
        if self.protocol.is_none() {
            return Phase::Handshaking;
        }
        if self.is_settled() {
            Phase::Ready
        } else {
            Phase::Enumerating {
                generation: self.generation,
            }
        }
    }

    /// The controller list is current, addressable, and no resync runs.
    pub fn is_settled(&self) -> bool {
        self.fault.is_none() && self.committed && matches!(self.resync, Resync::Idle)
    }

    /// A controller list has been committed at least once.
    pub fn has_committed(&self) -> bool {
        self.ever_committed
    }

    pub fn protocol(&self) -> Option<ProtocolVersion> {
        self.protocol
    }

    pub fn server_name(&self) -> Option<&WireString> {
        self.server_name.as_ref()
    }

    pub fn fault(&self) -> Option<&Fault> {
        self.fault.as_ref()
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down
    }

    /// The current list generation, bumped by every `DEVICE_LIST_UPDATED`.
    #[cfg(test)]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Described controllers in server list order.
    pub fn controllers(&self) -> impl Iterator<Item = &Controller> {
        self.order
            .iter()
            .filter_map(|dev_id| self.controllers.get(dev_id))
    }

    pub fn controller(&self, dev_id: u32) -> Option<&Controller> {
        self.controllers.get(&dev_id)
    }

    /// Stop requesting anything; [`Mirror::should_close`] reports when the
    /// socket can close.
    pub fn begin_shutdown(&mut self) {
        self.shutting_down = true;
    }

    /// The socket can close now: after [`Mirror::begin_shutdown`] or a fault,
    /// once the output is flushed and no acknowledgement is outstanding (every
    /// in-order one when shutting down, the writes' after a fault); at once when
    /// the stream can no longer be framed.
    pub fn should_close(&self) -> bool {
        match &self.fault {
            Some(fault) if !fault.stream_intact() => true,
            Some(_) => self.outbox.is_empty() && self.writes.is_empty(),
            None => {
                self.shutting_down
                    && self.outbox.is_empty()
                    && self.writes.is_empty()
                    && !self
                        .inorder
                        .iter()
                        .any(|entry| entry.ack == AckExpect::Awaited)
            }
        }
    }

    /// Queue a write (`UPDATEMODE` or `UPDATELEDS`) the session encoded; at
    /// protocol 6 it stays outstanding until acknowledged.
    pub(super) fn send_write(&mut self, dev_id: u32, packet: PacketId, frame: &[u8]) {
        self.outbox.push(frame);
        if self.protocol == Some(ProtocolVersion::V6) {
            *self.writes.entry((dev_id, packet.to_wire())).or_default() += 1;
        }
    }

    /// Request `dev_id`'s description after the session's writes to it were
    /// acknowledged (protocol 6); a [`MirrorEvent::ReadBack`] reports the
    /// result and the cache holds the new description.
    pub(super) fn request_readback(&mut self, dev_id: u32, token: ReadBackToken) {
        if let Some(protocol) = self.protocol {
            self.request(
                Request::Data {
                    dev_id,
                    role: DataRole::ReadBack(token),
                },
                protocol,
            );
        }
    }

    fn emit(&mut self, event: MirrorEvent) {
        self.events.push_back(event);
    }

    fn fail(&mut self, fault: Fault) {
        if self.fault.is_none() {
            self.fault = Some(fault.clone());
            self.emit(MirrorEvent::Faulted(fault));
        }
    }

    fn decode_fault(frame: &Frame, error: DecodeError) -> Fault {
        Fault::Decode {
            packet: frame.pkt_id,
            dev_id: frame.dev_id,
            error,
        }
    }

    /// Send a request whose reply or acknowledgement comes back in order.
    fn request(&mut self, request: Request, protocol: ProtocolVersion) {
        let bytes = match request {
            Request::Version => codec::request_protocol_version(self.config.max_protocol),
            Request::Count(_) => codec::request_controller_count(),
            Request::Data { dev_id, .. } => codec::request_controller_data(dev_id, protocol),
            Request::ClientName => codec::set_client_name(&self.config.client_name),
            Request::ClientFlags => codec::set_client_flags(ClientFlags::SUPPORTS_RGBCONTROLLER),
        };
        self.outbox.push(&bytes);
        let ack = AckExpect::for_protocol(protocol);
        // At protocol 5 a request with neither reply nor acknowledgement has
        // nothing to correlate.
        if request.reply().is_some() || ack == AckExpect::Awaited {
            self.inorder.push_back(InOrder {
                request,
                replied: false,
                ack,
            });
        }
    }

    fn handle(&mut self, frame: Frame) {
        if self.fault.is_some() {
            // Only acknowledgements of outstanding writes still matter: they
            // release the close.
            if frame.pkt_id == PacketId::Ack
                && let Ok(ack) = codec::decode_ack(&frame.payload)
                && is_write(ack.acked)
            {
                self.write_ack(frame.dev_id, ack.acked, ack.status);
            }
            return;
        }
        match frame.pkt_id {
            PacketId::RequestProtocolVersion
            | PacketId::RequestControllerCount
            | PacketId::RequestControllerData => {
                self.in_order_reply(frame);
            }
            PacketId::Ack => match codec::decode_ack(&frame.payload) {
                Ok(ack) if is_write(ack.acked) => {
                    self.write_ack(frame.dev_id, ack.acked, ack.status)
                }
                Ok(ack) => self.in_order_ack(frame.dev_id, ack.acked, ack.status),
                Err(error) => self.fail(Self::decode_fault(&frame, error)),
            },
            PacketId::SetServerName => match codec::decode_server_name(&frame.payload) {
                Ok(name) => {
                    self.server_name = Some(name.clone());
                    self.emit(MirrorEvent::ServerName(name));
                }
                Err(error) => self.fail(Self::decode_fault(&frame, error)),
            },
            PacketId::SetServerFlags => match codec::decode_server_flags(&frame.payload) {
                Ok(flags) => self.emit(MirrorEvent::ServerFlags(flags)),
                Err(error) => self.fail(Self::decode_fault(&frame, error)),
            },
            PacketId::DeviceListUpdated => self.list_updated(),
            PacketId::DetectionStarted => self.emit(MirrorEvent::DetectionStarted),
            PacketId::DetectionProgressChanged => {
                match codec::decode_detection_progress(&frame.payload) {
                    Ok(progress) => self.emit(MirrorEvent::DetectionProgress {
                        percent: progress.percent,
                        text: progress.text,
                    }),
                    Err(error) => self.fail(Self::decode_fault(&frame, error)),
                }
            }
            PacketId::DetectionComplete => self.emit(MirrorEvent::DetectionComplete),
            PacketId::SignalUpdate if self.protocol == Some(ProtocolVersion::V6) => {
                self.signal_update(frame)
            }
            packet => self.emit(MirrorEvent::Skipped {
                packet,
                dev_id: frame.dev_id,
                size: frame.payload.len(),
            }),
        }
    }

    fn in_order_reply(&mut self, frame: Frame) {
        loop {
            let Some(head) = self.inorder.front_mut() else {
                self.fail(Fault::Unexpected {
                    packet: frame.pkt_id,
                    dev_id: frame.dev_id,
                    detail: "no request is outstanding",
                });
                return;
            };
            let request = head.request;
            let matches_head = !head.replied
                && request.reply() == Some(frame.pkt_id)
                && (frame.pkt_id != PacketId::RequestControllerData
                    || request.dev_id() == frame.dev_id);
            if matches_head {
                head.replied = true;
                self.on_reply(request, &frame);
                if self.fault.is_none()
                    && let Some(head) = self.inorder.front()
                    && head.replied
                    && head.ack == AckExpect::None
                {
                    self.pop_and_complete(None);
                }
                return;
            }
            if request == Request::Version && frame.pkt_id == PacketId::RequestControllerCount {
                self.fail(Fault::ProtocolZero);
                return;
            }
            // Protocol 5: a data request the server skipped (its index no longer
            // exists) is followed directly by the next reply.
            if matches!(request, Request::Data { .. })
                && !head.replied
                && head.ack == AckExpect::None
            {
                self.pop_and_complete(None);
                continue;
            }
            self.fail(Fault::Unexpected {
                packet: frame.pkt_id,
                dev_id: frame.dev_id,
                detail: "it does not answer the oldest outstanding request",
            });
            return;
        }
    }

    fn in_order_ack(&mut self, dev_id: u32, acked: PacketId, status: AckStatus) {
        let Some(head) = self.inorder.front() else {
            self.fail(Fault::UnexpectedAck { dev_id, acked });
            return;
        };
        if head.ack != AckExpect::Awaited
            || head.request.packet() != acked
            || head.request.dev_id() != dev_id
        {
            self.fail(Fault::UnexpectedAck { dev_id, acked });
            return;
        }
        if head.request.reply_required() && !head.replied {
            self.fail(Fault::Unexpected {
                packet: PacketId::Ack,
                dev_id,
                detail: "the acknowledgement precedes the reply it follows",
            });
            return;
        }
        self.pop_and_complete(Some(status));
    }

    fn write_ack(&mut self, dev_id: u32, packet: PacketId, status: AckStatus) {
        let key = (dev_id, packet.to_wire());
        match self.writes.get_mut(&key) {
            Some(outstanding) => {
                *outstanding -= 1;
                if *outstanding == 0 {
                    self.writes.remove(&key);
                }
                if self.fault.is_none() {
                    self.emit(MirrorEvent::WriteAck {
                        dev_id,
                        packet,
                        status,
                    });
                }
            }
            None => self.fail(Fault::UnexpectedAck {
                dev_id,
                acked: packet,
            }),
        }
    }

    /// Remove the head request and act on its completion.
    fn pop_and_complete(&mut self, status: Option<AckStatus>) {
        let Some(entry) = self.inorder.pop_front() else {
            return;
        };
        if let Some(status) = status
            && status != AckStatus::Ok
        {
            self.emit(MirrorEvent::RequestRejected {
                packet: entry.request.packet(),
                dev_id: entry.request.dev_id(),
                status,
            });
        }
        let Request::Data { dev_id, role } = entry.request else {
            return;
        };
        match role {
            DataRole::ReadBack(token) => self.emit(MirrorEvent::ReadBack {
                dev_id,
                token,
                found: entry.replied,
            }),
            DataRole::Enumerate => match &mut self.resync {
                Resync::V6 { outstanding, .. } => {
                    if !entry.replied {
                        // Listed by the count, gone before its data was sent.
                        self.order.retain(|id| *id != dev_id);
                    }
                    *outstanding = outstanding.saturating_sub(1);
                    if *outstanding == 0 {
                        self.finish_resync();
                    }
                }
                Resync::V5 { missing, .. } => {
                    if !entry.replied {
                        *missing = true;
                    }
                }
                Resync::Idle | Resync::Seeding { .. } => {}
            },
        }
    }

    fn on_reply(&mut self, request: Request, frame: &Frame) {
        match request {
            Request::Version => self.on_version(frame),
            Request::Count(role) => {
                let Some(protocol) = self.protocol else {
                    self.fail(Fault::ProtocolZero);
                    return;
                };
                match codec::decode_controller_count(&frame.payload, protocol) {
                    Ok(list) => match role {
                        CountRole::Seed => self.on_seed(list),
                        CountRole::V5Fence => self.on_fence(list),
                    },
                    Err(error) => self.fail(Self::decode_fault(frame, error)),
                }
            }
            Request::Data { dev_id, role } => {
                let Some(protocol) = self.protocol else {
                    return;
                };
                match codec::decode_controller_data(&frame.payload, protocol) {
                    Ok(description) => {
                        self.on_data(dev_id, role, description, frame.payload.clone())
                    }
                    Err(error) => self.fail(Self::decode_fault(frame, error)),
                }
            }
            Request::ClientName | Request::ClientFlags => {}
        }
    }

    fn on_version(&mut self, frame: &Frame) {
        let server_max = match codec::decode_protocol_version(&frame.payload) {
            Ok(version) => version,
            Err(error) => {
                self.fail(Self::decode_fault(frame, error));
                return;
            }
        };
        let protocol = match ProtocolVersion::negotiate(server_max, self.config.max_protocol) {
            Ok(protocol) => protocol,
            Err(unsupported) => {
                self.fail(Fault::Unsupported(unsupported));
                return;
            }
        };
        self.protocol = Some(protocol);
        for entry in &mut self.inorder {
            if entry.ack == AckExpect::Undetermined {
                entry.ack = AckExpect::for_protocol(protocol);
            }
        }
        self.emit(MirrorEvent::Negotiated {
            server_max,
            protocol,
        });
        if self.shutting_down {
            return;
        }
        if protocol == ProtocolVersion::V6 {
            // The server leaves a client's flags uninitialised until this packet
            // sets them, and gates profile notifications that wait for client
            // acknowledgements on them.
            self.request(Request::ClientFlags, protocol);
        }
        self.request(Request::ClientName, protocol);
    }

    fn on_data(
        &mut self,
        dev_id: u32,
        role: DataRole,
        description: ControllerDescription,
        raw: Vec<u8>,
    ) {
        let controller = Controller {
            dev_id,
            description,
            raw,
        };
        match (role, &mut self.resync) {
            (DataRole::Enumerate, Resync::V5 { received, .. }) => {
                received.insert(dev_id, controller);
            }
            (DataRole::Enumerate, _) => {
                self.controllers.insert(dev_id, controller);
            }
            (DataRole::ReadBack(_), _) => {
                // Ids are never reused, so data for an id the list no longer
                // holds is not re-inserted.
                if let Some(cached) = self.controllers.get_mut(&dev_id) {
                    *cached = controller;
                }
            }
        }
    }

    fn start_resync(&mut self) {
        let Some(protocol) = self.protocol else {
            return;
        };
        self.resync_pending = false;
        self.resync = Resync::Seeding {
            started: self.generation,
        };
        self.request(Request::Count(CountRole::Seed), protocol);
    }

    fn on_seed(&mut self, list: ControllerList) {
        let Resync::Seeding { started } = self.resync else {
            return;
        };
        let Some(protocol) = self.protocol else {
            return;
        };
        match list {
            ControllerList::Ids(ids) => {
                let listed: BTreeSet<u32> = ids.iter().copied().collect();
                let gone: Vec<u32> = self
                    .controllers
                    .keys()
                    .copied()
                    .filter(|id| !listed.contains(id))
                    .collect();
                for dev_id in gone {
                    self.controllers.remove(&dev_id);
                    self.emit(MirrorEvent::ControllerRemoved { dev_id });
                }
                let added: Vec<u32> = ids
                    .iter()
                    .copied()
                    .filter(|id| !self.controllers.contains_key(id))
                    .collect();
                self.order = ids;
                if self.shutting_down {
                    self.resync = Resync::Idle;
                    return;
                }
                self.resync = Resync::V6 {
                    outstanding: added.len(),
                };
                for dev_id in &added {
                    self.request(
                        Request::Data {
                            dev_id: *dev_id,
                            role: DataRole::Enumerate,
                        },
                        protocol,
                    );
                }
                if added.is_empty() {
                    self.finish_resync();
                }
            }
            ControllerList::Count(count) => {
                if count > MAX_CONTROLLERS {
                    self.fail(Fault::ControllerCountTooLarge {
                        count,
                        max: MAX_CONTROLLERS,
                    });
                    return;
                }
                if self.shutting_down {
                    self.resync = Resync::Idle;
                    return;
                }
                self.resync = Resync::V5 {
                    started,
                    seed: count,
                    received: BTreeMap::new(),
                    missing: false,
                };
                for index in 0..count {
                    self.request(
                        Request::Data {
                            dev_id: index,
                            role: DataRole::Enumerate,
                        },
                        protocol,
                    );
                }
                self.request(Request::Count(CountRole::V5Fence), protocol);
            }
        }
    }

    fn on_fence(&mut self, list: ControllerList) {
        let ControllerList::Count(fence) = list else {
            return;
        };
        let Resync::V5 {
            started,
            seed,
            received,
            missing,
        } = std::mem::replace(&mut self.resync, Resync::Idle)
        else {
            return;
        };
        if self.shutting_down {
            return;
        }
        if started != self.generation {
            // A DEVICE_LIST_UPDATED arrived during this resync: its results may
            // mix two lists. The fresh count is sent after that notification was
            // read, so it reflects the change.
            self.start_resync();
        } else if !missing && fence == seed && received.len() == seed as usize {
            self.order = (0..seed).collect();
            self.controllers = received;
            self.finish_resync();
        } else {
            self.emit(MirrorEvent::ListInconsistent);
        }
    }

    fn finish_resync(&mut self) {
        self.resync = Resync::Idle;
        if self.resync_pending && !self.shutting_down {
            self.start_resync();
            return;
        }
        self.committed = true;
        self.ever_committed = true;
        let controllers = self
            .order
            .iter()
            .filter(|id| self.controllers.contains_key(id))
            .count();
        self.emit(MirrorEvent::ListCommitted { controllers });
    }

    fn list_updated(&mut self) {
        self.generation += 1;
        if self.protocol == Some(ProtocolVersion::V5) {
            // Indices may now name different controllers.
            self.committed = false;
        }
        if self.shutting_down {
            return;
        }
        match self.resync {
            Resync::Idle if self.protocol.is_some() => self.start_resync(),
            _ => self.resync_pending = true,
        }
    }

    fn signal_update(&mut self, frame: Frame) {
        let update = match codec::decode_signal_update(&frame.payload, ProtocolVersion::V6) {
            Ok(update) => update,
            Err(error) => {
                self.fail(Self::decode_fault(&frame, error));
                return;
            }
        };
        let Some(controller) = self.controllers.get_mut(&frame.dev_id) else {
            self.emit(MirrorEvent::Skipped {
                packet: frame.pkt_id,
                dev_id: frame.dev_id,
                size: frame.payload.len(),
            });
            return;
        };
        match update.body {
            SignalUpdateBody::Colours(colours) => controller.description.colors = colours,
            SignalUpdateBody::Device(description) => controller.description = *description,
        }
        self.emit(MirrorEvent::CacheRefreshed {
            dev_id: frame.dev_id,
            reason: update.reason,
        });
    }
}

fn is_write(packet: PacketId) -> bool {
    matches!(packet, PacketId::UpdateLeds | PacketId::UpdateMode)
}
