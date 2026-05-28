//! Inspect submap-mode telemetry captured by the daemon.
//!
//! The daemon writes transitions to ~/.local/state/vogix/modes.log as:
//!
//!     2026-05-09T22:01:14.123Z  app -> desktop  (app: 4523ms)
//!
//! These commands parse that log and surface ergonomics signals:
//!   - `recent`     — last N transitions, raw
//!   - `stats`      — per-mode dwell histogram + counts
//!   - `confusion`  — re-entries within a threshold (likely accidental)
//!
//! Parsing is permissive: malformed lines are skipped (with a warn count
//! summarised at the end) so a corrupted tail doesn't kill analysis.
use crate::errors::{Result, VogixError};
use crate::state::State;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// One parsed transition row.
#[derive(Debug, Clone, PartialEq)]
struct Transition {
    timestamp: String,
    prev: String,
    next: String,
    dwell_ms: u64,
}

fn modes_log_path() -> Result<PathBuf> {
    Ok(State::state_dir()?.join("modes.log"))
}

/// Parse one log line. Returns None if the line doesn't match the expected
/// format — caller decides whether to count or report.
fn parse_line(line: &str) -> Option<Transition> {
    // Format: "{ts}  {prev} -> {next}  ({prev}: {dwell}ms)"
    // We split on "  " (double space) to get the three fields without
    // depending on a regex crate.
    let mut parts = line.splitn(3, "  ");
    let timestamp = parts.next()?.trim().to_string();
    let arrow = parts.next()?.trim();
    let dwell_field = parts.next()?.trim();

    // arrow: "prev -> next"
    let mut arrow_parts = arrow.splitn(2, " -> ");
    let prev = arrow_parts.next()?.trim().to_string();
    let next = arrow_parts.next()?.trim().to_string();

    // dwell_field: "(prev: 4523ms)"
    let dwell_inner = dwell_field
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))?;
    // "prev: 4523ms"
    let (_label, ms_part) = dwell_inner.split_once(':')?;
    let ms_str = ms_part.trim().strip_suffix("ms")?;
    let dwell_ms: u64 = ms_str.trim().parse().ok()?;

    Some(Transition {
        timestamp,
        prev,
        next,
        dwell_ms,
    })
}

/// Read and parse the entire modes.log. Returns (transitions, malformed_count).
fn load_log() -> Result<(Vec<Transition>, usize)> {
    let path = modes_log_path()?;
    if !path.exists() {
        return Err(VogixError::Config(format!(
            "no modes.log at {} — daemon not running, or no submap activity yet",
            path.display()
        )));
    }
    let raw = fs::read_to_string(&path)?;
    let mut transitions = Vec::new();
    let mut malformed = 0usize;
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(line) {
            Some(t) => transitions.push(t),
            None => malformed += 1,
        }
    }
    Ok((transitions, malformed))
}

pub fn handle_modes_recent(count: usize) -> Result<()> {
    let (transitions, malformed) = load_log()?;
    let total = transitions.len();
    let start = total.saturating_sub(count);
    for t in &transitions[start..] {
        println!(
            "{}  {} -> {}  ({}: {}ms)",
            t.timestamp, t.prev, t.next, t.prev, t.dwell_ms
        );
    }
    if malformed > 0 {
        eprintln!("(skipped {} malformed line(s))", malformed);
    }
    Ok(())
}

pub fn handle_modes_stats() -> Result<()> {
    let (transitions, malformed) = load_log()?;
    if transitions.is_empty() {
        println!("No transitions recorded yet.");
        return Ok(());
    }

    // Aggregate dwell time per mode (using `prev` since dwell is "time spent
    // in prev before transitioning"). `next` of the final entry is the mode
    // currently active and isn't yet closed out.
    let mut totals: BTreeMap<String, (u64, u64, u64, u64)> = BTreeMap::new();
    // (sum_ms, count, min_ms, max_ms)
    for t in &transitions {
        let entry = totals.entry(t.prev.clone()).or_insert((0, 0, u64::MAX, 0));
        entry.0 += t.dwell_ms;
        entry.1 += 1;
        entry.2 = entry.2.min(t.dwell_ms);
        entry.3 = entry.3.max(t.dwell_ms);
    }

    println!(
        "{:<10}  {:>6}  {:>10}  {:>10}  {:>10}  {:>10}",
        "mode", "count", "total(ms)", "avg(ms)", "min(ms)", "max(ms)"
    );
    println!("{}", "-".repeat(64));
    for (mode, (sum, count, min, max)) in &totals {
        let avg = sum / count.max(&1);
        println!(
            "{:<10}  {:>6}  {:>10}  {:>10}  {:>10}  {:>10}",
            mode, count, sum, avg, min, max
        );
    }
    println!();
    println!("Total transitions: {}", transitions.len());
    if malformed > 0 {
        eprintln!("(skipped {} malformed line(s))", malformed);
    }
    Ok(())
}

pub fn handle_modes_confusion(threshold_ms: u64) -> Result<()> {
    let (transitions, malformed) = load_log()?;

    // A "confusion" pattern: enter mode M, leave M within threshold, re-enter M
    // within threshold of leaving. Indicates an accidental tap or canceled
    // intention.
    //
    // Concretely we walk consecutive triples (a, b, c) where:
    //   a = X -> M       (entered M)
    //   b = M -> Y       with b.dwell_ms < threshold  (left M quickly)
    //   c = Y -> M       with c.dwell_ms < threshold  (came back quickly)
    let mut hits: Vec<(&Transition, &Transition, &Transition)> = Vec::new();
    for window in transitions.windows(3) {
        let (a, b, c) = (&window[0], &window[1], &window[2]);
        if a.next != b.prev {
            continue;
        }
        if b.dwell_ms >= threshold_ms {
            continue;
        }
        if c.dwell_ms >= threshold_ms {
            continue;
        }
        if c.next != a.next {
            continue;
        }
        hits.push((a, b, c));
    }

    if hits.is_empty() {
        println!(
            "No confusion patterns under {}ms across {} transitions.",
            threshold_ms,
            transitions.len()
        );
    } else {
        println!(
            "{} confusion pattern(s) under {}ms (entered, left fast, re-entered fast):",
            hits.len(),
            threshold_ms
        );
        for (a, b, c) in &hits {
            println!(
                "  {} → {}  then {}ms → {}  then {}ms → {}",
                a.timestamp, a.next, b.dwell_ms, b.next, c.dwell_ms, c.next
            );
        }
    }
    if malformed > 0 {
        eprintln!("(skipped {} malformed line(s))", malformed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_basic() {
        let line = "2026-05-10T04:03:21.666Z  app -> desktop  (app: 3516ms)";
        let t = parse_line(line).expect("should parse");
        assert_eq!(t.timestamp, "2026-05-10T04:03:21.666Z");
        assert_eq!(t.prev, "app");
        assert_eq!(t.next, "desktop");
        assert_eq!(t.dwell_ms, 3516);
    }

    #[test]
    fn test_parse_line_zero_dwell() {
        let line = "2026-05-10T04:03:23.988Z  desktop -> app  (desktop: 0ms)";
        let t = parse_line(line).expect("should parse");
        assert_eq!(t.dwell_ms, 0);
    }

    #[test]
    fn test_parse_line_rejects_garbage() {
        assert!(parse_line("not a log line").is_none());
        assert!(parse_line("").is_none());
        assert!(parse_line("a -> b").is_none());
    }

    #[test]
    fn test_parse_line_rejects_missing_dwell_unit() {
        // missing "ms" suffix
        let line = "2026-05-10T04:03:21.666Z  app -> desktop  (app: 3516)";
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn test_parse_line_rejects_non_numeric_dwell() {
        let line = "2026-05-10T04:03:21.666Z  app -> desktop  (app: fastms)";
        assert!(parse_line(line).is_none());
    }
}
