pragma ComponentBehavior: Bound
// AI-agent usage: one row per Claude Code account (the fleet's account
// layout — ~/.claude is the default, ~/.claude-accounts/<alias> the rest),
// with today's session count and output tokens summed from each account's
// projects/*.jsonl transcripts. Read-only; refreshed on open.
import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    property var rows: []

    Component.onCompleted: proc.running = true

    spacing: 10

    PanelLabel {
        text: "Agents"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    PanelLabel {
        visible: root.rows.length === 0
        text: "no Claude accounts found"
        color: Tokens.color("popup", "muted")
    }

    Repeater {
        model: root.rows

        RowLayout {
            id: row

            required property var modelData

            Layout.fillWidth: true

            PanelLabel {
                Layout.fillWidth: true
                text: "󱚝 " + row.modelData.name
            }

            PanelLabel {
                text: row.modelData.sessions + " today"
                color: Tokens.color("popup", "muted")
            }

            PanelLabel {
                text: row.modelData.tokens + " tok"
                color: Tokens.color("popup", "accent")
            }
        }
    }

    Process {
        id: proc
        // One subprocess per open: for each account dir, count today's
        // transcript files and sum their output tokens. Tab-separated rows.
        command: ["sh", "-c", `
            for dir in "$HOME/.claude" "$HOME"/.claude-accounts/*/; do
              [ -d "$dir/projects" ] || continue
              name=$(basename "$dir")
              [ "$name" = ".claude" ] && name=default
              files=$(find "$dir/projects" -name '*.jsonl' -newermt "today" 2>/dev/null)
              [ -n "$files" ] || { printf '%s\\t0\\t0\\n' "$name"; continue; }
              sessions=$(printf '%s\\n' "$files" | wc -l)
              tokens=$(printf '%s\\n' "$files" | xargs grep -ho '"output_tokens":[0-9]*' 2>/dev/null \\
                | cut -d: -f2 | awk '{s+=$1} END{print s+0}')
              printf '%s\\t%s\\t%s\\n' "$name" "$sessions" "$tokens"
            done
        `]
        stdout: StdioCollector {
            onStreamFinished: {
                root.rows = text.split("\n").filter(l => l !== "").map(l => {
                    const p = l.split("\t");
                    return { name: p[0], sessions: p[1] ?? "0", tokens: p[2] ?? "0" };
                });
            }
        }
    }
}
