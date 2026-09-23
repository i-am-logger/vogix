//! Hyprland IPC dispatch over the control socket (`.socket.sock`).
//!
//! The schema's actions are Hyprland config-bind strings (`"movefocus, l"`,
//! `"exec, $TERMINAL"`, `"killactive,"`). To run one we translate it to the
//! socket command form (`dispatch movefocus l`) and write it to Hyprland's
//! request socket — the same thing `hyprctl dispatch` does, but without spawning
//! a process per keystroke.

use std::cell::OnceCell;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Which config engine the connected compositor runs. Hyprland ≥ 0.55 has two:
/// the legacy hyprlang parser and the Lua engine (mandatory from 0.57). The
/// wire dialect differs on every WRITE — `keyword` is rejected under Lua
/// ("keyword can't work with non-legacy parsers. Use eval.") and a legacy
/// `dispatch <name> <args>` line is parsed as a Lua expression — while reads
/// (`j/…` queries, the `.socket2.sock` event stream) are identical. Reported
/// by `j/status` as `configProvider`; an instance without that endpoint
/// predates the Lua engine and is therefore hyprlang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigProvider {
    #[default]
    Hyprlang,
    Lua,
}

/// Translate a schema action (`"dispatcher, args"`) into a socket command
/// (`"dispatch dispatcher args"`).
///
/// The action's first comma separates the dispatcher from its arguments;
/// Hyprland's socket protocol wants them space-separated after `dispatch`.
pub fn action_to_command(action: &str) -> String {
    let a = action.trim();
    match a.split_once(',') {
        Some((disp, args)) => {
            let args = args.trim();
            if args.is_empty() {
                format!("dispatch {}", disp.trim())
            } else {
                format!("dispatch {} {}", disp.trim(), args)
            }
        }
        None => format!("dispatch {a}"),
    }
}

/// A handle to Hyprland's control socket.
#[derive(Debug, Clone)]
pub struct Hypr {
    socket: PathBuf,
    /// The config engine behind this socket, as the instance itself stated
    /// it. Unset while the instance has not answered: a compositor that is
    /// still starting accepts the connection and answers later, so a missing
    /// answer says nothing about the engine. It is asked again before each
    /// write until it answers, and then fixed for the handle's life. A
    /// compositor restart re-enters through [`Hypr::discover`] (the callers
    /// drop the handle on a rejected write), so a provider change — e.g. the
    /// hyprlang→Lua migration flip — is picked up with the new socket.
    provider: OnceCell<ConfigProvider>,
}

impl Hypr {
    /// Locate a **live** Hyprland control socket.
    ///
    /// Candidates, in priority order: the instance named by
    /// `$HYPRLAND_INSTANCE_SIGNATURE` (the compositor that launched us), then
    /// every other instance socket newest-modified first. Each candidate is
    /// connection-tested ([`is_live`]) and dead ones are skipped — `$HIS` is
    /// NOT trusted blindly. After a Hyprland crash/restart the dead instance's
    /// `.socket.sock` lingers on disk and the stale signature still in our
    /// environment would otherwise pin the engine to that dead compositor
    /// forever (the "keybindings stop after Hyprland restarts" bug); falling
    /// through to the newest *live* socket re-attaches to the restarted
    /// compositor with no service restart. `$HIS` is also commonly absent for a
    /// systemd *user* unit, where the newest-live scan is the only path —
    /// `XDG_RUNTIME_DIR` is always set, so this needs no env propagation.
    pub fn discover() -> Option<Self> {
        Self::candidate_sockets()
            .into_iter()
            .find(|sock| Self::is_live(sock))
            .map(Self::attach)
    }

    /// A handle on `socket`, asking the instance once which config engine it
    /// runs. An unanswered question leaves the provider unset for the writes
    /// to ask again ([`Self::write_provider`]).
    fn attach(socket: PathBuf) -> Self {
        let provider = OnceCell::new();
        if let Some(p) = Self::query_provider(&socket) {
            let _ = provider.set(p);
        }
        Self { socket, provider }
    }

    /// Which config engine this handle talks to, asking the instance if it
    /// has not said yet. `None` while it has not answered.
    pub fn provider(&self) -> Option<ConfigProvider> {
        if let Some(p) = self.provider.get() {
            return Some(*p);
        }
        let p = Self::query_provider(&self.socket)?;
        Some(*self.provider.get_or_init(|| p))
    }

    /// The dialect a write must be sent in. A write in a guessed dialect is
    /// wrong on the other engine (`keyword` under Lua is rejected; a Lua
    /// expression under hyprlang is an unknown dispatcher), so an instance
    /// that has not answered fails the write instead, and the caller's
    /// re-discovery or retry asks again.
    fn write_provider(&self) -> std::io::Result<ConfigProvider> {
        self.provider().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "hyprland has not answered which config engine it runs (j/status); \
                 not writing in a guessed dialect",
            )
        })
    }

    /// Ask the instance which config engine it runs (`j/status` →
    /// `"configProvider": "lua" | "hyprlang"`), or `None` when it gave no
    /// complete answer (see [`provider_from_reply`]).
    fn query_provider(socket: &Path) -> Option<ConfigProvider> {
        let mut stream = UnixStream::connect(socket).ok()?;
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok()?;
        stream
            .set_write_timeout(Some(Duration::from_millis(200)))
            .ok()?;
        stream.write_all(b"j/status").ok()?;
        let mut buf = String::new();
        let complete = stream.read_to_string(&mut buf).is_ok();
        provider_from_reply(&buf, complete)
    }

    /// Candidate control-socket paths in priority order (see [`Self::discover`]).
    fn candidate_sockets() -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        if let Ok(his) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            for base in Self::socket_bases() {
                out.push(base.join(&his).join(".socket.sock"));
            }
        }
        // Every instance socket, newest-modified first — covers a restarted
        // compositor whose new signature isn't in our environment yet.
        let mut dated: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        for base in Self::socket_bases() {
            let Ok(entries) = std::fs::read_dir(&base) else {
                continue;
            };
            for entry in entries.flatten() {
                let sock = entry.path().join(".socket.sock");
                if !sock.exists() || out.contains(&sock) {
                    continue;
                }
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                dated.push((mtime, sock));
            }
        }
        dated.sort_by(|a, b| b.0.cmp(&a.0));
        out.extend(dated.into_iter().map(|(_, sock)| sock));
        out
    }

    /// True when a Hyprland is actually listening on `socket`. A lingering
    /// socket file left by a crashed instance still `exists()` but refuses the
    /// connection (`ECONNREFUSED`), so this is what distinguishes a live
    /// compositor from a dead one's leftover node.
    fn is_live(socket: &Path) -> bool {
        UnixStream::connect(socket).is_ok()
    }

    /// Directories that hold per-instance Hyprland socket folders, preferred
    /// first.
    fn socket_bases() -> Vec<PathBuf> {
        let mut bases = Vec::new();
        if let Ok(x) = std::env::var("XDG_RUNTIME_DIR") {
            bases.push(PathBuf::from(x).join("hypr"));
        }
        bases.push(PathBuf::from("/tmp/hypr"));
        bases
    }

    /// The resolved socket path.
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket
    }

    /// Run a schema action by dispatching it over the socket. Under the Lua
    /// engine the legacy string is translated to a `hl.dsp.*` expression
    /// first ([`super::hypr_lua::action_to_lua`]); a dispatcher with no Lua
    /// translation fails loudly rather than sending a guaranteed Lua syntax
    /// error.
    pub fn dispatch(&self, action: &str) -> std::io::Result<()> {
        match self.write_provider()? {
            ConfigProvider::Hyprlang => self.send(&action_to_command(action)),
            ConfigProvider::Lua => {
                let expr = super::hypr_lua::action_to_lua(action).map_err(std::io::Error::other)?;
                self.send(&format!("dispatch {expr}"))
            }
        }
    }

    /// Set a Hyprland config keyword at runtime (e.g. a per-mode border colour),
    /// as `hyprctl keyword <key> <value>` does but over the socket directly.
    /// Under the Lua engine `keyword` no longer exists; the same change is an
    /// incremental `eval hl.config({…})`.
    pub fn set_keyword(&self, key: &str, value: &str) -> std::io::Result<()> {
        match self.write_provider()? {
            ConfigProvider::Hyprlang => self.send(&format!("keyword {key} {value}")),
            ConfigProvider::Lua => {
                self.send(&format!("eval {}", keywords_to_eval(&[(key, value)])))
            }
        }
    }

    /// Set several keywords in one round trip. Hyprlang takes a `[[BATCH]]` of
    /// `keyword` commands; the Lua engine takes a single `eval hl.config({…})`
    /// with the key paths merged into one table (both border colours share
    /// `general.col`, so one eval covers what two keywords did).
    pub fn set_keywords(&self, pairs: &[(&str, &str)]) -> std::io::Result<()> {
        if pairs.is_empty() {
            return Ok(());
        }
        match self.write_provider()? {
            ConfigProvider::Hyprlang => {
                let body = pairs
                    .iter()
                    .map(|(k, v)| format!("keyword {k} {v}"))
                    .collect::<Vec<_>>()
                    .join(";");
                self.send(&format!("[[BATCH]]{body}"))
            }
            ConfigProvider::Lua => self.send(&format!("eval {}", keywords_to_eval(pairs))),
        }
    }

    /// Path of this instance's EVENT socket (`.socket2.sock`), derived from the
    /// request socket. Hyprland streams newline-delimited `EVENT>>DATA` here —
    /// we read `activewindow>>class,title` to track the focused window's class
    /// for the context-aware Super→Ctrl remap.
    pub fn event_socket_path(&self) -> PathBuf {
        self.socket.with_file_name(".socket2.sock")
    }

    /// Connect to the event stream (non-blocking) for active-window tracking.
    pub fn connect_events(&self) -> std::io::Result<UnixStream> {
        let stream = UnixStream::connect(self.event_socket_path())?;
        stream.set_nonblocking(true)?;
        Ok(stream)
    }

    /// Query the currently-focused window's class via `j/activewindow` (used to
    /// SEED the class on startup; the event stream only delivers later changes).
    /// `None` when nothing is focused or the query fails.
    pub fn query_active_class(&self) -> Option<String> {
        let mut stream = UnixStream::connect(&self.socket).ok()?;
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok()?;
        stream
            .set_write_timeout(Some(Duration::from_millis(200)))
            .ok()?;
        stream.write_all(b"j/activewindow").ok()?;
        let mut buf = String::new();
        let _ = stream.read_to_string(&mut buf);
        parse_active_class_json(&buf)
    }

    /// Write a raw command to the control socket and check the reply.
    fn send(&self, command: &str) -> std::io::Result<()> {
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(Duration::from_millis(200)))?;
        stream.set_write_timeout(Some(Duration::from_millis(200)))?;
        stream.write_all(command.as_bytes())?;
        // Hyprland replies "ok" on success, or an error string. Treat a
        // non-ok reply as a failure: a stale socket left by a *restarted*
        // compositor frequently still `connect()`s and accepts the write but
        // rejects the dispatch — and without inspecting the reply that silent
        // drop looks like success. That is the "keybindings stopped working
        // after Hyprland restarted" symptom: the engine keeps dispatching into
        // a dead instance. Returning Err here lets the caller drop the stale
        // handle and re-discover the live socket.
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);
        let reply = String::from_utf8_lossy(&buf);
        if !reply_is_ok(&reply) {
            return Err(std::io::Error::other(format!(
                "hyprland rejected '{command}': {}",
                reply.trim()
            )));
        }
        Ok(())
    }
}

/// Is a control-socket reply a success? A single request answers exactly
/// `ok`; a `[[BATCH]]` answers one `ok` per command joined by blank lines —
/// `ok\n\n\nok` for a two-command batch (probed live on 0.56.2, hyprlang
/// provider). Success = every non-empty line is `ok`. An empty / timed-out
/// reply is tolerated as ok so a merely slow compositor doesn't churn
/// re-discovery.
fn reply_is_ok(reply: &str) -> bool {
    reply
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .all(|line| line == "ok")
}

/// Extract the window class from a Hyprland `activewindow>>class,title` event
/// line. The title may itself contain commas, so the class is the first field
/// after `>>`. Returns `None` for non-activewindow lines or an empty class
/// (e.g. `activewindow>>,` when focus is lost → fail-safe to non-terminal).
pub fn parse_activewindow_event(line: &str) -> Option<String> {
    let rest = line
        .trim_end_matches(['\n', '\r'])
        .strip_prefix("activewindow>>")?;
    let class = rest.split(',').next().unwrap_or("").trim();
    (!class.is_empty()).then(|| class.to_string())
}

/// Extract `"class"` from a `j/activewindow` JSON reply with a minimal field
/// scan (no serde dependency here). An empty reply (`{}` / no focus) → None.
fn parse_active_class_json(json: &str) -> Option<String> {
    let key = "\"class\":";
    let after = json[json.find(key)? + key.len()..].trim_start();
    let after = after.strip_prefix('"')?;
    let class = &after[..after.find('"')?];
    (!class.is_empty()).then(|| class.to_string())
}

/// Extract the config engine from a `j/status` reply (same minimal field scan
/// as [`parse_active_class_json`]). `None` when the field is absent — a
/// pre-Lua Hyprland, whose reply to the unknown request carries no such key.
fn parse_config_provider_json(json: &str) -> Option<ConfigProvider> {
    let key = "\"configProvider\":";
    let after = json[json.find(key)? + key.len()..].trim_start();
    let after = after.strip_prefix('"')?;
    let value = &after[..after.find('"')?];
    match value {
        "lua" => Some(ConfigProvider::Lua),
        "hyprlang" => Some(ConfigProvider::Hyprlang),
        _ => None,
    }
}

/// What a `j/status` reply says about the config engine. `complete` is
/// whether the instance closed the connection after replying (Hyprland
/// replies, then closes) rather than the read timing out.
///
/// - A reply naming the provider is the answer, even if the read then timed
///   out.
/// - A complete, non-empty reply without the field comes from an instance
///   that predates the Lua engine ("unknown request"), which is hyprlang.
/// - Anything else is no answer (`None`). That includes an empty reply and a
///   timed-out read: a compositor still starting up accepts the connection
///   and answers later.
fn provider_from_reply(reply: &str, complete: bool) -> Option<ConfigProvider> {
    match parse_config_provider_json(reply) {
        Some(provider) => Some(provider),
        None if complete && !reply.trim().is_empty() => Some(ConfigProvider::Hyprlang),
        None => None,
    }
}

/// Render one or more `section:sub.key = value` keyword pairs as a single
/// incremental `hl.config({…})` expression for the Lua engine's `eval`, in
/// the flat bracketed-key form:
///
/// ```text
/// hl.config({ ["general.col.active_border"] = "rgb(3366aa)" })
/// ```
///
/// The Lua manager registers every hyprlang name with `:` → `.` and `-` → `_`
/// (`luaConfigValueName`), and `hl.config` resolves a flat dotted key in one
/// lookup — no nesting logic, no ambiguity about where a path segment ends.
/// Values keep hyprlang's spelling: integers, floats and booleans go bare,
/// everything else becomes a Lua string literal. The legacy `[[EMPTY]]`
/// sentinel (hyprlang's "unset a string option") maps to `""` — verified on
/// 0.56.2: clearing `decoration:screen_shader` with `""` reads back cleanly
/// empty. A multi-pair call is one atomic eval and one refresh schedule.
pub fn keywords_to_eval(pairs: &[(&str, &str)]) -> String {
    let body = pairs
        .iter()
        .map(|(key, value)| {
            let lua_key = key.replace(':', ".").replace('-', "_");
            format!(
                "[{}] = {}",
                super::hypr_lua::lua_str(&lua_key),
                lua_value(value)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("hl.config({{ {body} }})")
}

/// Render a hyprlang keyword value as a Lua expression: numbers and booleans
/// bare, everything else a quoted string. `[[EMPTY]]` is hyprlang's
/// unset-a-string sentinel and becomes the empty string.
fn lua_value(value: &str) -> String {
    let v = value.trim();
    if v == "[[EMPTY]]" {
        return "\"\"".to_string();
    }
    if v == "true" || v == "false" {
        return v.to_string();
    }
    if v.parse::<i64>().is_ok() || v.parse::<f64>().is_ok() {
        return v.to_string();
    }
    super::hypr_lua::lua_str(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_action_becomes_dispatch() {
        assert_eq!(action_to_command("movefocus, l"), "dispatch movefocus l");
    }

    #[test]
    fn no_arg_action_drops_trailing_comma() {
        assert_eq!(action_to_command("killactive,"), "dispatch killactive");
        assert_eq!(action_to_command("fullscreen"), "dispatch fullscreen");
    }

    #[test]
    fn exec_and_workspace_and_resize() {
        assert_eq!(
            action_to_command("exec, $TERMINAL"),
            "dispatch exec $TERMINAL"
        );
        assert_eq!(action_to_command("workspace, 1"), "dispatch workspace 1");
        assert_eq!(
            action_to_command("resizeactive, -40 0"),
            "dispatch resizeactive -40 0"
        );
    }

    #[test]
    fn whitespace_is_normalized() {
        assert_eq!(
            action_to_command("  movewindow ,  l "),
            "dispatch movewindow l"
        );
    }

    #[test]
    fn activewindow_event_extracts_class() {
        // Class is the first field; the title may contain commas.
        assert_eq!(
            parse_activewindow_event("activewindow>>kitty,~/work, notes"),
            Some("kitty".into())
        );
        assert_eq!(
            parse_activewindow_event("activewindow>>firefox,Mozilla\n"),
            Some("firefox".into())
        );
        // Focus lost (empty class) → None → fail-safe to non-terminal.
        assert_eq!(parse_activewindow_event("activewindow>>,"), None);
        // Unrelated events are ignored.
        assert_eq!(parse_activewindow_event("workspace>>2"), None);
    }

    #[test]
    fn config_provider_json_is_scanned() {
        // The exact reply shape observed on 0.56.2 (nested probe).
        assert_eq!(
            parse_config_provider_json(
                "\n{\n    \"configProvider\": \"lua\",\n    \"backend\": \"wayland\"\n}"
            ),
            Some(ConfigProvider::Lua)
        );
        assert_eq!(
            parse_config_provider_json(
                "{\"configProvider\": \"hyprlang\", \"backend\": \"wayland\"}"
            ),
            Some(ConfigProvider::Hyprlang)
        );
        // A pre-Lua Hyprland answers the unknown request with no such field.
        assert_eq!(parse_config_provider_json("unknown request"), None);
        assert_eq!(parse_config_provider_json("{}"), None);
    }

    #[test]
    fn only_an_answer_settles_the_provider() {
        let lua = "\n{\n    \"configProvider\": \"lua\",\n    \"backend\": \"drm\"\n}\n";
        assert_eq!(provider_from_reply(lua, true), Some(ConfigProvider::Lua));
        // Named before the read timed out: still the answer.
        assert_eq!(provider_from_reply(lua, false), Some(ConfigProvider::Lua));
        assert_eq!(
            provider_from_reply(r#"{"configProvider": "hyprlang"}"#, true),
            Some(ConfigProvider::Hyprlang)
        );
        // A pre-Lua instance answers the unknown request, and closes.
        assert_eq!(
            provider_from_reply("unknown request", true),
            Some(ConfigProvider::Hyprlang)
        );
        // No answer yet: a compositor still starting, or one that closed
        // without a reply. Neither says which engine it runs.
        assert_eq!(provider_from_reply("", false), None);
        assert_eq!(provider_from_reply("", true), None);
        assert_eq!(provider_from_reply("\n{\n", false), None);
    }

    /// A stand-in control socket. `status` answers the n-th (0-based)
    /// `j/status` request, or `None` to leave it unanswered (the connection
    /// stays open, as a compositor's does while it is busy). Every other
    /// request is recorded and answered `ok`.
    struct FakeInstance {
        dir: PathBuf,
        socket: PathBuf,
        writes: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl FakeInstance {
        fn start(name: &str, status: fn(usize) -> Option<(Duration, &'static str)>) -> Self {
            use std::os::unix::net::UnixListener;
            use std::sync::{Arc, Mutex};
            // Short: a Unix socket path is at most 107 bytes, and TMPDIR can
            // be long under a dev shell.
            let dir = std::env::temp_dir().join(format!(".vh-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("fake instance dir");
            let socket = dir.join(".socket.sock");
            let listener = UnixListener::bind(&socket).expect("bind fake instance");
            let writes = Arc::new(Mutex::new(Vec::new()));
            let seen = writes.clone();
            let queries = Arc::new(Mutex::new(0usize));
            std::thread::spawn(move || {
                for conn in listener.incoming() {
                    let Ok(mut conn) = conn else { return };
                    let (seen, queries) = (seen.clone(), queries.clone());
                    std::thread::spawn(move || {
                        let mut buf = [0u8; 4096];
                        let n = conn.read(&mut buf).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buf[..n]).to_string();
                        if request.is_empty() {
                            return;
                        }
                        if request != "j/status" {
                            seen.lock().unwrap().push(request);
                            let _ = conn.write_all(b"ok");
                            return;
                        }
                        let nth = {
                            let mut q = queries.lock().unwrap();
                            *q += 1;
                            *q - 1
                        };
                        match status(nth) {
                            Some((delay, reply)) => {
                                std::thread::sleep(delay);
                                let _ = conn.write_all(reply.as_bytes());
                            }
                            None => std::thread::sleep(Duration::from_secs(2)),
                        }
                    });
                }
            });
            Self {
                dir,
                socket,
                writes,
            }
        }

        fn writes(&self) -> Vec<String> {
            self.writes.lock().unwrap().clone()
        }
    }

    impl Drop for FakeInstance {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    const LUA_STATUS: &str = r#"{"configProvider": "lua", "backend": "drm"}"#;

    #[test]
    fn a_late_status_answer_is_asked_again_not_guessed() {
        // The first j/status is answered after the handle stopped waiting,
        // as a compositor still starting its session does; later ones at
        // once. The write must go out in the Lua dialect.
        let fake = FakeInstance::start("late", |nth| {
            Some(if nth == 0 {
                (Duration::from_millis(600), LUA_STATUS)
            } else {
                (Duration::ZERO, LUA_STATUS)
            })
        });
        let hypr = Hypr::attach(fake.socket.clone());
        hypr.set_keyword("general:col.active_border", "rgb(6c5d52)")
            .expect("the write goes out once the instance answers");
        assert_eq!(hypr.provider(), Some(ConfigProvider::Lua));
        assert_eq!(
            fake.writes(),
            vec![r#"eval hl.config({ ["general.col.active_border"] = "rgb(6c5d52)" })"#]
        );
    }

    #[test]
    fn an_instance_that_never_answers_gets_no_write() {
        let fake = FakeInstance::start("silent", |_| None);
        let hypr = Hypr::attach(fake.socket.clone());
        let err = hypr
            .set_keyword("general:col.active_border", "rgb(6c5d52)")
            .expect_err("no dialect, no write");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        assert!(hypr.dispatch("workspace, 2").is_err());
        assert_eq!(hypr.provider(), None);
        assert!(fake.writes().is_empty(), "wrote {:?}", fake.writes());
    }

    #[test]
    fn a_pre_lua_instance_is_hyprlang() {
        let fake = FakeInstance::start("prelua", |_| Some((Duration::ZERO, "unknown request")));
        let hypr = Hypr::attach(fake.socket.clone());
        hypr.set_keyword("general:col.active_border", "rgb(6c5d52)")
            .expect("hyprlang write");
        assert_eq!(
            fake.writes(),
            vec!["keyword general:col.active_border rgb(6c5d52)"]
        );
    }

    #[test]
    fn reply_parsing_accepts_batch_ok() {
        // Single request: bare `ok`. `[[BATCH]]`: one `ok` per command joined
        // by blank lines — the exact shape probed live on 0.56.2.
        assert!(reply_is_ok("ok"));
        assert!(reply_is_ok("ok\n\n\nok"));
        // Empty / timed-out reads are tolerated (slow compositor ≠ stale).
        assert!(reply_is_ok(""));
        assert!(reply_is_ok("\n"));
        // Any error text fails, alone or inside a batch reply.
        assert!(!reply_is_ok("error: invalid keyword"));
        assert!(!reply_is_ok("ok\n\n\nerror: invalid keyword"));
        assert!(!reply_is_ok("unknown request"));
    }

    #[test]
    fn keyword_renders_as_flat_bracketed_config() {
        assert_eq!(
            keywords_to_eval(&[("general:col.active_border", "rgb(3366aa)")]),
            r#"hl.config({ ["general.col.active_border"] = "rgb(3366aa)" })"#
        );
    }

    #[test]
    fn keyword_pairs_share_one_atomic_eval() {
        // The daemon's border pair must become ONE eval → one refresh.
        assert_eq!(
            keywords_to_eval(&[
                ("general:col.active_border", "rgb(89b4fa)"),
                ("general:col.inactive_border", "rgb(313244)"),
            ]),
            r#"hl.config({ ["general.col.active_border"] = "rgb(89b4fa)", ["general.col.inactive_border"] = "rgb(313244)" })"#
        );
    }

    #[test]
    fn keyword_values_keep_their_types() {
        assert_eq!(
            keywords_to_eval(&[("general:border_size", "5")]),
            r#"hl.config({ ["general.border_size"] = 5 })"#
        );
        assert_eq!(
            keywords_to_eval(&[("decoration:dim_strength", "0.3")]),
            r#"hl.config({ ["decoration.dim_strength"] = 0.3 })"#
        );
        assert_eq!(
            keywords_to_eval(&[("misc:disable_autoreload", "true")]),
            r#"hl.config({ ["misc.disable_autoreload"] = true })"#
        );
    }

    #[test]
    fn keyword_names_map_like_lua_config_value_name() {
        // luaConfigValueName: ':' → '.' and '-' → '_'.
        assert_eq!(
            keywords_to_eval(&[("some:dashed-name", "1")]),
            r#"hl.config({ ["some.dashed_name"] = 1 })"#
        );
    }

    #[test]
    fn legacy_empty_sentinel_becomes_empty_string() {
        // hyprlang clears a string option with [[EMPTY]]; the Lua engine
        // clears with "" (verified on 0.56.2: screen_shader reads back "").
        assert_eq!(
            keywords_to_eval(&[("decoration:screen_shader", "[[EMPTY]]")]),
            r#"hl.config({ ["decoration.screen_shader"] = "" })"#
        );
    }

    #[test]
    fn active_class_json_is_scanned() {
        assert_eq!(
            parse_active_class_json(r#"{"address":"0x5","class":"kitty","title":"x"}"#),
            Some("kitty".into())
        );
        assert_eq!(parse_active_class_json("{}"), None); // no focus
        assert_eq!(parse_active_class_json(r#"{"class":""}"#), None); // empty class
    }

    // Regression: a Hyprland crash/restart leaves the dead instance's
    // `.socket.sock` file on disk. `discover()` must treat that lingering node
    // as NOT live, otherwise a stale $HYPRLAND_INSTANCE_SIGNATURE pins the
    // engine to the dead compositor and keybindings stay dead until the service
    // is restarted by hand.
    #[test]
    fn is_live_rejects_lingering_dead_socket() {
        use std::os::unix::net::UnixListener;
        let sock =
            std::env::temp_dir().join(format!(".vogix-hypr-islive-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock);

        // No socket node at all → not live.
        assert!(!Hypr::is_live(&sock));

        // A listening (live) compositor → live.
        let listener = UnixListener::bind(&sock).expect("bind test socket");
        assert!(Hypr::is_live(&sock));

        // Listener gone but the file lingers (the crashed-instance case) → the
        // connection is refused, so it must read as not live.
        //
        // Closing the listener does not end the socket instantly in a process
        // that forks: any concurrently spawned child holds copies of every fd
        // until its exec, and those copies keep the socket connectable
        // (O_CLOEXEC clears at exec, not at fork). Other tests in this binary
        // spawn processes, so assert the EVENTUAL state with a deadline
        // instead of the instant after drop — the production concern is a
        // compositor that died seconds-to-minutes ago, not a microsecond
        // fork window.
        drop(listener);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match std::os::unix::net::UnixStream::connect(&sock) {
                Err(e) => {
                    assert_eq!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionRefused,
                        "a lingering dead socket must refuse, got: {e:?}"
                    );
                    assert!(!Hypr::is_live(&sock));
                    break;
                }
                Ok(_) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Ok(_) => panic!("socket still connectable 5s after the listener closed"),
            }
        }

        let _ = std::fs::remove_file(&sock);
    }
}
