//! Legacy dispatcher → Lua translation for Hyprland's Lua config engine.
//!
//! Under the Lua provider the text after `dispatch ` on the control socket is
//! spliced verbatim into `return hl.dispatch(<text>)` (HyprCtl.cpp), so it
//! must be a single Lua expression that evaluates to a dispatcher object —
//! `hl.dsp.window.close()`, not `killactive`. This module is the pure
//! translation from the schema's legacy action strings (`"movefocus, l"`)
//! to those expressions. The mapping was read out of the Hyprland 0.56.2
//! sources (`src/config/lua/bindings/LuaBindingsDispatchers.cpp`) and
//! verified against nested instances; the wire contract is documented in
//! `docs/hyprland-lua-ipc.md`.
//!
//! Selector grammars carry over from legacy unchanged (workspace `+1`,
//! `previous`, `name:x`, `special:x`, `e+1`; window `address:0x…`,
//! `class:…`; monitor `l`/`DP-1`) — only the call shape changes. Three
//! dispatchers take a bare string instead of a table (`submap`, `layout`,
//! `workspace.toggle_special`); "silent" is spelled `follow = false`; and
//! `fullscreen {n}` must be normalised to `"maximized"`/`"fullscreen"`
//! because the Lua factory rejects other mode numbers the legacy dispatcher
//! silently accepted.

/// Translate a legacy schema action (`"dispatcher, args"`) into the Lua
/// dispatcher expression to send as `dispatch <expr>`.
///
/// Returns `Err` with a precise message for a dispatcher that has no Lua
/// translation here — sending the legacy string would be a guaranteed Lua
/// syntax error, so failing loudly is the honest behaviour.
pub fn action_to_lua(action: &str) -> Result<String, String> {
    let a = action.trim();
    let (disp, args) = match a.split_once(',') {
        Some((d, r)) => (d.trim(), r.trim()),
        None => (a, ""),
    };

    let expr = match disp {
        "exec" => format!("hl.dsp.exec_cmd({})", lua_str(args)),
        "execr" => format!("hl.dsp.exec_raw({})", lua_str(args)),
        "workspace" => format!("hl.dsp.focus({{ workspace = {} }})", lua_str(args)),
        "focusworkspaceoncurrentmonitor" => format!(
            "hl.dsp.focus({{ workspace = {}, on_current_monitor = true }})",
            lua_str(args)
        ),
        "movetoworkspace" => move_to_workspace(args, false),
        "movetoworkspacesilent" => move_to_workspace(args, true),
        "movefocus" => format!("hl.dsp.focus({{ direction = {} }})", lua_str(args)),
        "focusmonitor" => format!("hl.dsp.focus({{ monitor = {} }})", lua_str(args)),
        "focuswindow" => format!("hl.dsp.focus({{ window = {} }})", lua_str(args)),
        "swapwindow" => format!("hl.dsp.window.swap({{ direction = {} }})", lua_str(args)),
        "swapnext" => "hl.dsp.window.swap({ next = true })".to_string(),
        "movewindow" | "movewindoworgroup" => {
            let group_aware = disp == "movewindoworgroup";
            move_window(args, group_aware)?
        }
        "moveactive" => {
            let (dx, dy) = parse_two_ints(disp, args)?;
            format!("hl.dsp.window.move({{ x = {dx}, y = {dy}, relative = true }})")
        }
        "resizeactive" => {
            // Legacy also accepted percentages and `exact`; vogix only ever
            // emits integer pixel deltas, and the Lua factory takes plain
            // numbers — anything else is refused here rather than silently
            // mistranslated.
            let (dx, dy) = parse_two_ints(disp, args)?;
            format!("hl.dsp.window.resize({{ x = {dx}, y = {dy}, relative = true }})")
        }
        // Bare-scalar dispatchers — a table argument would error (submap,
        // layout) or silently degrade to the unnamed special workspace
        // (toggle_special), so these three never go through table building.
        "submap" => format!("hl.dsp.submap({})", lua_str(args)),
        "layoutmsg" => format!("hl.dsp.layout({})", lua_str(args)),
        "togglespecialworkspace" => {
            if args.is_empty() {
                "hl.dsp.workspace.toggle_special()".to_string()
            } else {
                format!("hl.dsp.workspace.toggle_special({})", lua_str(args))
            }
        }
        "killactive" => "hl.dsp.window.close()".to_string(),
        "forcekillactive" => "hl.dsp.window.kill()".to_string(),
        "closewindow" => format!("hl.dsp.window.close({{ window = {} }})", lua_str(args)),
        "togglefloating" => "hl.dsp.window.float()".to_string(),
        "setfloating" => "hl.dsp.window.float({ action = \"enable\" })".to_string(),
        "settiled" => "hl.dsp.window.float({ action = \"disable\" })".to_string(),
        "fullscreen" => match args {
            "" => "hl.dsp.window.fullscreen()".to_string(),
            // Legacy: 1 = maximize, any other number = fullscreen. The Lua
            // factory only accepts "maximized"/"fullscreen" (or "1"/"0"), so
            // normalise instead of forwarding the raw number.
            "1" => "hl.dsp.window.fullscreen({ mode = \"maximized\" })".to_string(),
            _ => "hl.dsp.window.fullscreen({ mode = \"fullscreen\" })".to_string(),
        },
        "pin" => "hl.dsp.window.pin()".to_string(),
        "pseudo" => "hl.dsp.window.pseudo()".to_string(),
        "centerwindow" => "hl.dsp.window.center()".to_string(),
        "cyclenext" => "hl.dsp.window.cycle_next()".to_string(),
        "bringactivetotop" => "hl.dsp.window.bring_to_top()".to_string(),
        "togglegroup" => "hl.dsp.group.toggle()".to_string(),
        "changegroupactive" => {
            // Legacy treats anything other than b/prev as forward.
            if args == "b" || args == "prev" {
                "hl.dsp.group.prev()".to_string()
            } else {
                "hl.dsp.group.next()".to_string()
            }
        }
        "movegroupwindow" => {
            if args == "b" {
                "hl.dsp.group.move_window({ forward = false })".to_string()
            } else {
                "hl.dsp.group.move_window()".to_string()
            }
        }
        "exit" => "hl.dsp.exit()".to_string(),
        other => {
            return Err(format!(
                "no Lua translation for dispatcher '{other}' (action '{action}'); \
                 the legacy string cannot be sent to a Lua-engine Hyprland"
            ));
        }
    };

    debug_assert!(
        expr.contains('('),
        "every Lua dispatcher expression must be a call: {expr}"
    );
    Ok(expr)
}

/// `movetoworkspace[silent] <ws>[,<window-selector>]` — the window selector is
/// how session restore targets a specific client (`1,address:0x…`). Silence is
/// spelled `follow = false`; there is no `silent` field, and a stray one would
/// be ignored into a non-silent move.
fn move_to_workspace(args: &str, silent: bool) -> String {
    let (ws, window) = match args.split_once(',') {
        Some((w, sel)) => (w.trim(), Some(sel.trim())),
        None => (args, None),
    };
    let mut fields = format!("workspace = {}", lua_str(ws));
    if let Some(sel) = window {
        fields.push_str(&format!(", window = {}", lua_str(sel)));
    }
    if silent {
        fields.push_str(", follow = false");
    }
    format!("hl.dsp.window.move({{ {fields} }})")
}

/// `movewindow <dir>` or `movewindow mon:<sel>[ silent]`. The Lua monitor
/// selector grammar has no `mon:` prefix — it must be stripped here.
fn move_window(args: &str, group_aware: bool) -> Result<String, String> {
    let group = if group_aware {
        ", group_aware = true"
    } else {
        ""
    };
    if let Some(rest) = args.strip_prefix("mon:") {
        let (sel, silent) = match rest.strip_suffix(" silent") {
            Some(s) => (s.trim(), true),
            None => (rest.trim(), false),
        };
        let follow = if silent { ", follow = false" } else { "" };
        return Ok(format!(
            "hl.dsp.window.move({{ monitor = {}{follow}{group} }})",
            lua_str(sel)
        ));
    }
    if args.is_empty() {
        // A bare `hl.dsp.window.move()` would start an interactive mouse
        // drag (legacy `mouse movewindow`), which is never what a keybind
        // action means — refuse instead.
        return Err("movewindow needs a direction or mon:<selector>".to_string());
    }
    Ok(format!(
        "hl.dsp.window.move({{ direction = {}{group} }})",
        lua_str(args)
    ))
}

fn parse_two_ints(disp: &str, args: &str) -> Result<(i64, i64), String> {
    let mut it = args.split_whitespace();
    let parse = |s: Option<&str>| -> Result<i64, String> {
        s.ok_or_else(|| format!("{disp} needs two integer arguments, got '{args}'"))?
            .parse::<i64>()
            .map_err(|_| {
                format!(
                    "{disp} takes plain pixel integers under the Lua engine \
                     (legacy percentages/`exact` have no Lua form), got '{args}'"
                )
            })
    };
    let dx = parse(it.next())?;
    let dy = parse(it.next())?;
    Ok((dx, dy))
}

/// Quote a Rust string as a Lua 5.5 short string literal. Escapes the
/// backslash, the double quote, and the control bytes a short literal cannot
/// contain raw; everything else — `'`, `$`, `;`, `%`, braces, UTF-8 — passes
/// through, so shell commands and store paths survive verbatim. Shell quoting
/// is a separate concern: the string reaches `/bin/sh -c` untouched.
pub fn lua_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // The quick-reference table from docs/hyprland-lua-ipc.md, verbatim.
    #[test]
    fn quick_reference_translations() {
        let cases = [
            ("exec, walker", r#"hl.dsp.exec_cmd("walker")"#),
            ("workspace, 2", r#"hl.dsp.focus({ workspace = "2" })"#),
            ("workspace, +1", r#"hl.dsp.focus({ workspace = "+1" })"#),
            (
                "workspace, previous",
                r#"hl.dsp.focus({ workspace = "previous" })"#,
            ),
            (
                "movetoworkspace, special:hidden",
                r#"hl.dsp.window.move({ workspace = "special:hidden" })"#,
            ),
            (
                "movetoworkspacesilent, 3",
                r#"hl.dsp.window.move({ workspace = "3", follow = false })"#,
            ),
            ("movefocus, l", r#"hl.dsp.focus({ direction = "l" })"#),
            (
                "swapwindow, u",
                r#"hl.dsp.window.swap({ direction = "u" })"#,
            ),
            (
                "resizeactive, -30 0",
                "hl.dsp.window.resize({ x = -30, y = 0, relative = true })",
            ),
            ("submap, reset", r#"hl.dsp.submap("reset")"#),
            ("submap, resize", r#"hl.dsp.submap("resize")"#),
            ("killactive,", "hl.dsp.window.close()"),
            ("togglefloating,", "hl.dsp.window.float()"),
            ("fullscreen", "hl.dsp.window.fullscreen()"),
            (
                "fullscreen, 1",
                r#"hl.dsp.window.fullscreen({ mode = "maximized" })"#,
            ),
            (
                "fullscreen, 2",
                r#"hl.dsp.window.fullscreen({ mode = "fullscreen" })"#,
            ),
            ("pin,", "hl.dsp.window.pin()"),
            ("togglegroup,", "hl.dsp.group.toggle()"),
            ("changegroupactive, f", "hl.dsp.group.next()"),
            ("changegroupactive, b", "hl.dsp.group.prev()"),
            ("pseudo,", "hl.dsp.window.pseudo()"),
            ("layoutmsg, togglesplit", r#"hl.dsp.layout("togglesplit")"#),
            ("focusmonitor, l", r#"hl.dsp.focus({ monitor = "l" })"#),
            (
                "movewindow, l",
                r#"hl.dsp.window.move({ direction = "l" })"#,
            ),
            (
                "movewindow, mon:DP-1",
                r#"hl.dsp.window.move({ monitor = "DP-1" })"#,
            ),
            (
                "movewindow, mon:r silent",
                r#"hl.dsp.window.move({ monitor = "r", follow = false })"#,
            ),
            (
                "togglespecialworkspace, magic",
                r#"hl.dsp.workspace.toggle_special("magic")"#,
            ),
            (
                "togglespecialworkspace",
                "hl.dsp.workspace.toggle_special()",
            ),
            ("exit", "hl.dsp.exit()"),
        ];
        for (legacy, lua) in cases {
            assert_eq!(action_to_lua(legacy).as_deref(), Ok(lua), "for {legacy:?}");
        }
    }

    #[test]
    fn session_restore_window_selector_carries_over() {
        // Session restore targets a client by address; the selector string is
        // legacy-compatible and rides in the `window` field.
        assert_eq!(
            action_to_lua("movetoworkspacesilent, 3,address:0x55e1").as_deref(),
            Ok(
                r#"hl.dsp.window.move({ workspace = "3", window = "address:0x55e1", follow = false })"#
            )
        );
    }

    #[test]
    fn exec_keeps_shell_text_verbatim() {
        assert_eq!(
            action_to_lua("exec, grimblast save area - | swappy -f -").as_deref(),
            Ok(r#"hl.dsp.exec_cmd("grimblast save area - | swappy -f -")"#)
        );
        // Legacy exec rules blocks still parse inside the command string.
        assert_eq!(
            action_to_lua("exec, [float;size 800 600] kitty").as_deref(),
            Ok(r#"hl.dsp.exec_cmd("[float;size 800 600] kitty")"#)
        );
        // A quote in the command must not break the literal.
        assert_eq!(
            action_to_lua(r#"exec, notify-send "hi there""#).as_deref(),
            Ok(r#"hl.dsp.exec_cmd("notify-send \"hi there\"")"#)
        );
    }

    #[test]
    fn untranslatable_dispatchers_fail_loudly() {
        let err = action_to_lua("dpms, off").unwrap_err();
        assert!(err.contains("dpms"), "{err}");
        let err = action_to_lua("resizeactive, 10% 0").unwrap_err();
        assert!(err.contains("percentages"), "{err}");
        let err = action_to_lua("movewindow,").unwrap_err();
        assert!(err.contains("direction"), "{err}");
    }

    #[test]
    fn lua_str_escaping() {
        assert_eq!(lua_str("plain $VAR; %s"), r#""plain $VAR; %s""#);
        assert_eq!(lua_str("a\tb"), r#""a\tb""#);
        assert_eq!(lua_str("\x01"), r#""\x01""#);
        assert_eq!(lua_str("nixos ❄"), "\"nixos ❄\"");
    }
}
