# Hyprland IPC under the two config engines

Hyprland ≥ 0.55 ships two config engines: the legacy **hyprlang** parser and the
**Lua** engine (the only one from 0.57, which also removes the legacy `keyword`
and non-Lua `dispatch` IPC — hyprwm/Hyprland#15539). vogix talks to the
compositor over the raw control socket for keybind dispatch, border colours,
the screen shader, session restore and mode switching, so every *write* must
speak the dialect of the engine actually running. Reads (`j/…` queries, the
`.socket2.sock` event stream) are identical under both.

Everything below was verified against nested Hyprland **0.56.2** instances, one
per engine, over the raw `.socket.sock`.

## Detecting the engine

`j/status` returns:

```json
{ "configProvider": "hyprlang" | "lua", "backend": "wayland" }
```

A pre-Lua Hyprland has no such endpoint, which is itself the answer: legacy is
the only dialect it speaks. `Hypr::discover()` performs this detection once per
connection; a compositor restart re-enters discovery, so an engine change (the
hyprlang→Lua migration flip) is picked up with the new socket.

## Writes, by engine

| Operation | hyprlang | Lua |
|---|---|---|
| set one option | `keyword general:border_size 5` → `ok` | `eval hl.config({ general = { border_size = 5 } })` → `ok` |
| set several | `[[BATCH]]keyword k v;keyword k v` → `ok\n\n\nok` | one `eval hl.config({ merged nested table })` (atomic) — `[[BATCH]]eval …;eval …` also works |
| clear a string option | `keyword decoration:screen_shader [[EMPTY]]` | `screen_shader = ""` (reads back as a cleanly empty `str: ""`) |
| dispatch | `dispatch movefocus l` | `dispatch <Lua expression>` — the line is wrapped as `return hl.dispatch(<text>)`, so it must be a Lua dispatcher call, e.g. `dispatch hl.dsp.focus({ workspace = "2" })`; legacy strings are Lua syntax errors |
| exit | `dispatch exit` | `dispatch hl.dsp.exit()` |

Wrong-dialect writes fail with stable text:

- `keyword` under Lua → `keyword can't work with non-legacy parsers. Use eval.`
- `eval` under hyprlang → `eval is only supported with the lua config manager`

## Reply contract

Success is `ok` — for a **single** request, exactly that. A `[[BATCH]]`
concatenates one reply per command joined by blank lines, so a two-command
keyword batch answers `ok\n\n\nok` (probed live on 0.56.2, hyprlang provider).
Errors are non-`ok` text; Lua-side failures are prefixed `error:` and include
the Lua traceback position, e.g.

```
error: return hl.config({ general = { definitely_not_a_key = 1 } });:1: unknown config key 'general.definitely_not_a_key'
```

vogix's socket layer accepts a reply whose every non-empty line is `ok` and
treats anything else as a rejected write (`reply_is_ok` in
`src/input/hypr.rs`), which also covers wrong-dialect replies: the caller
drops the handle and re-discovers, and discovery re-detects the engine.

## Values in `hl.config`

Integers, floats and booleans are bare Lua values; everything else is a quoted
Lua string (escape `\`, `"`, and line breaks — nothing else). Colours keep
hyprlang spelling as strings (`"rgb(3366aa)"`); reading such an option back via
`j/getoption` reports type `gradient` under Lua where hyprlang said `custom`.

## Unchanged under Lua

- `j/…` queries (`j/getoption`, `j/clients`, `j/activewindow`, …).
- The `.socket2.sock` event stream, including `configreloaded>>` (emitted for
  both `reload` and `reload config-only`).
- `reload` and `reload config-only` themselves.
