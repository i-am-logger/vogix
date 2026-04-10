/// Report generation — transforms validation results into structured output.
///
/// Inspired by:
/// - W3C EARL (Evaluation and Report Language): test result vocabulary
/// - Vega-Lite: declarative visualization grammar
///
/// The report is a functor: ValidationResults → OutputFormat
/// JSON, HTML, SVG are different surfaces consuming the same data.
use crate::validate_themes::ThemeResult;

/// Generate a JSON report from validation results.
///
/// EARL-inspired structure:
/// - meta: generator info
/// - summary: aggregate counts
/// - results: per-theme details with luminance traces
pub fn to_json(results: &[ThemeResult], dataset_name: &str) -> String {
    let total = results.len();
    let mono_pass = results.iter().filter(|r| r.luminance_monotone).count();
    let wcag_pass = results.iter().filter(|r| r.wcag_aa).count();
    let dark = results.iter().filter(|r| r.polarity == "dark").count();
    let light = results.iter().filter(|r| r.polarity == "light").count();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"meta\": {\n");
    json.push_str("    \"generator\": \"vogix-ontology\",\n");
    json.push_str(&format!("    \"dataset\": \"{}\",\n", dataset_name));
    json.push_str(&format!(
        "    \"generated\": \"{}\"\n",
        chrono_now()
    ));
    json.push_str("  },\n");

    json.push_str("  \"summary\": {\n");
    json.push_str(&format!("    \"total\": {},\n", total));
    json.push_str(&format!("    \"luminance_monotone\": {},\n", mono_pass));
    json.push_str(&format!("    \"wcag_aa\": {},\n", wcag_pass));
    json.push_str(&format!("    \"dark\": {},\n", dark));
    json.push_str(&format!("    \"light\": {}\n", light));
    json.push_str("  },\n");

    json.push_str("  \"results\": [\n");
    for (i, r) in results.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"theme\": \"{}\",\n", r.theme));
        json.push_str(&format!("      \"variant\": \"{}\",\n", r.variant));
        json.push_str(&format!("      \"polarity\": \"{}\",\n", r.polarity));
        json.push_str(&format!("      \"slots\": {},\n", r.slots_found));
        json.push_str(&format!(
            "      \"luminance_monotone\": {},\n",
            r.luminance_monotone
        ));
        json.push_str(&format!("      \"wcag_aa\": {},\n", r.wcag_aa));
        json.push_str(&format!(
            "      \"contrast_ratio\": {},\n",
            r.contrast_ratio.map(|cr| format!("{:.2}", cr)).unwrap_or_else(|| "null".into())
        ));

        // Luminance ramp trace
        json.push_str("      \"luminance_ramp\": [");
        for (j, (key, lum)) in r.luminance_ramp.iter().enumerate() {
            json.push_str(&format!(
                "{{\"slot\":\"{}\",\"luminance\":{:.4}}}",
                key, lum
            ));
            if j < r.luminance_ramp.len() - 1 {
                json.push(',');
            }
        }
        json.push_str("],\n");

        json.push_str(&format!(
            "      \"mono_break_at\": {}\n",
            r.mono_break_at
                .map(|b| b.to_string())
                .unwrap_or_else(|| "null".into())
        ));

        json.push_str("    }");
        if i < results.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");

    json
}

/// Simple timestamp (no chrono dependency — use compile time or fixed)
fn chrono_now() -> String {
    // In a real build, this would use chrono or std::time
    "2026-04-09T00:00:00Z".to_string()
}

/// Generate an HTML report with embedded JSON for interactive visualization.
///
/// Single self-contained HTML file with:
/// - Summary cards (total, pass/fail counts)
/// - Sortable/filterable table of all themes
/// - Expandable luminance ramp traces
/// - Visual ramp bars showing where monotonicity breaks
pub fn to_html(results: &[ThemeResult], dataset_name: &str) -> String {
    let json_data = to_json(results, dataset_name);

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Vogix Theme Validation Report</title>
<style>
  :root {{
    --bg: #0d1117; --fg: #c9d1d9; --border: #30363d;
    --green: #3fb950; --red: #f85149; --yellow: #d29922;
    --blue: #58a6ff; --card-bg: #161b22;
  }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace; background: var(--bg); color: var(--fg); padding: 2rem; }}
  h1 {{ font-size: 1.5rem; margin-bottom: 0.5rem; }}
  .subtitle {{ color: #8b949e; margin-bottom: 2rem; }}
  .cards {{ display: flex; gap: 1rem; margin-bottom: 2rem; flex-wrap: wrap; }}
  .card {{ background: var(--card-bg); border: 1px solid var(--border); border-radius: 6px; padding: 1rem 1.5rem; min-width: 150px; }}
  .card .value {{ font-size: 2rem; font-weight: bold; }}
  .card .label {{ font-size: 0.8rem; color: #8b949e; }}
  .card.pass .value {{ color: var(--green); }}
  .card.fail .value {{ color: var(--red); }}
  .card.info .value {{ color: var(--blue); }}
  .controls {{ margin-bottom: 1rem; display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }}
  .controls input {{ background: var(--card-bg); border: 1px solid var(--border); color: var(--fg); padding: 0.4rem 0.8rem; border-radius: 4px; }}
  .controls button {{ background: var(--card-bg); border: 1px solid var(--border); color: var(--fg); padding: 0.4rem 0.8rem; border-radius: 4px; cursor: pointer; }}
  .controls button.active {{ background: var(--blue); color: var(--bg); }}
  table {{ width: 100%; border-collapse: collapse; font-size: 0.85rem; }}
  th {{ text-align: left; padding: 0.5rem; border-bottom: 2px solid var(--border); cursor: pointer; user-select: none; }}
  th:hover {{ color: var(--blue); }}
  td {{ padding: 0.5rem; border-bottom: 1px solid var(--border); }}
  tr:hover {{ background: var(--card-bg); }}
  .pass-badge {{ color: var(--green); }}
  .fail-badge {{ color: var(--red); }}
  .ramp {{ display: flex; gap: 2px; align-items: flex-end; height: 40px; }}
  .ramp-bar {{ width: 30px; background: var(--blue); border-radius: 2px 2px 0 0; transition: background 0.2s; }}
  .ramp-bar.break {{ background: var(--red); }}
  .trace {{ display: none; background: var(--card-bg); padding: 0.5rem; border-radius: 4px; margin-top: 0.25rem; font-size: 0.75rem; font-family: monospace; }}
  .trace.open {{ display: block; }}
  .expandable {{ cursor: pointer; }}
  .expandable:hover {{ color: var(--blue); }}
</style>
</head>
<body>
<h1>Vogix Theme Validation Report</h1>
<p class="subtitle">Formal ontology axiom evaluation — <span id="dataset"></span> (<span id="count"></span> themes)</p>

<div class="cards" id="cards"></div>

<div class="controls">
  <input type="text" id="search" placeholder="Search themes..." oninput="filterTable()">
  <button class="active" onclick="setFilter('all')">All</button>
  <button onclick="setFilter('pass')">Pass</button>
  <button onclick="setFilter('fail')">Fail</button>
  <button onclick="setFilter('dark')">Dark</button>
  <button onclick="setFilter('light')">Light</button>
</div>

<table>
  <thead>
    <tr>
      <th onclick="sortBy('theme')">Theme</th>
      <th onclick="sortBy('variant')">Variant</th>
      <th onclick="sortBy('polarity')">Polarity</th>
      <th onclick="sortBy('luminance_monotone')">Monotonicity</th>
      <th onclick="sortBy('wcag_aa')">WCAG AA</th>
      <th onclick="sortBy('contrast_ratio')">Contrast</th>
      <th>Luminance Ramp</th>
    </tr>
  </thead>
  <tbody id="tbody"></tbody>
</table>

<script>
const DATA = {json_data};
let currentFilter = 'all';
let currentSort = 'theme';
let sortAsc = true;

function init() {{
  document.getElementById('dataset').textContent = DATA.meta.dataset;
  document.getElementById('count').textContent = DATA.summary.total;

  const cards = document.getElementById('cards');
  cards.innerHTML = `
    <div class="card info"><div class="value">${{DATA.summary.total}}</div><div class="label">Total Themes</div></div>
    <div class="card pass"><div class="value">${{DATA.summary.luminance_monotone}}</div><div class="label">Monotone (${{(DATA.summary.luminance_monotone/DATA.summary.total*100).toFixed(0)}}%)</div></div>
    <div class="card pass"><div class="value">${{DATA.summary.wcag_aa}}</div><div class="label">WCAG AA (${{(DATA.summary.wcag_aa/DATA.summary.total*100).toFixed(0)}}%)</div></div>
    <div class="card info"><div class="value">${{DATA.summary.dark}}</div><div class="label">Dark</div></div>
    <div class="card info"><div class="value">${{DATA.summary.light}}</div><div class="label">Light</div></div>
    <div class="card fail"><div class="value">${{DATA.summary.total - DATA.summary.luminance_monotone}}</div><div class="label">Mono Failures</div></div>
    <div class="card fail"><div class="value">${{DATA.summary.total - DATA.summary.wcag_aa}}</div><div class="label">WCAG Failures</div></div>
  `;
  renderTable();
}}

function renderTable() {{
  const tbody = document.getElementById('tbody');
  let rows = [...DATA.results];

  // Filter
  const search = document.getElementById('search').value.toLowerCase();
  rows = rows.filter(r => {{
    if (search && !`${{r.theme}}/${{r.variant}}`.toLowerCase().includes(search)) return false;
    if (currentFilter === 'pass') return r.luminance_monotone && r.wcag_aa;
    if (currentFilter === 'fail') return !r.luminance_monotone || !r.wcag_aa;
    if (currentFilter === 'dark') return r.polarity === 'dark';
    if (currentFilter === 'light') return r.polarity === 'light';
    return true;
  }});

  // Sort
  rows.sort((a, b) => {{
    let va = a[currentSort], vb = b[currentSort];
    if (typeof va === 'boolean') {{ va = va ? 1 : 0; vb = vb ? 1 : 0; }}
    if (va == null) va = -1;
    if (vb == null) vb = -1;
    return sortAsc ? (va > vb ? 1 : -1) : (va < vb ? 1 : -1);
  }});

  tbody.innerHTML = rows.map((r, i) => {{
    const mono = r.luminance_monotone ? '<span class="pass-badge">✓</span>' : '<span class="fail-badge">✗</span>';
    const wcag = r.wcag_aa ? '<span class="pass-badge">✓</span>' : '<span class="fail-badge">✗</span>';
    const cr = r.contrast_ratio != null ? r.contrast_ratio.toFixed(1) + ':1' : '—';
    const maxLum = Math.max(...r.luminance_ramp.map(x => x.luminance), 0.01);
    const ramp = r.luminance_ramp.map((x, j) => {{
      const h = Math.max(2, (x.luminance / maxLum) * 40);
      const cls = r.mono_break_at != null && j >= r.mono_break_at ? 'ramp-bar break' : 'ramp-bar';
      return `<div class="${{cls}}" style="height:${{h}}px" title="${{x.slot}}: ${{x.luminance.toFixed(3)}}"></div>`;
    }}).join('');
    const trace = r.luminance_ramp.map(x => `${{x.slot}}: ${{x.luminance.toFixed(4)}}`).join('  ');
    const breakInfo = r.mono_break_at != null ? ` — breaks at ${{r.luminance_ramp[r.mono_break_at]?.slot || '?'}}` : '';

    return `<tr>
      <td>${{r.theme}}</td>
      <td>${{r.variant}}</td>
      <td>${{r.polarity}}</td>
      <td>${{mono}}</td>
      <td>${{wcag}}</td>
      <td>${{cr}}</td>
      <td>
        <div class="ramp">${{ramp}}</div>
        <div class="expandable" onclick="this.nextElementSibling.classList.toggle('open')">▸ trace${{breakInfo}}</div>
        <div class="trace">${{trace}}</div>
      </td>
    </tr>`;
  }}).join('');
}}

function sortBy(col) {{
  if (currentSort === col) sortAsc = !sortAsc;
  else {{ currentSort = col; sortAsc = true; }}
  renderTable();
}}

function setFilter(f) {{
  currentFilter = f;
  document.querySelectorAll('.controls button').forEach(b => b.classList.remove('active'));
  event.target.classList.add('active');
  renderTable();
}}

function filterTable() {{ renderTable(); }}

init();
</script>
</body>
</html>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_results() -> Vec<ThemeResult> {
        vec![
            ThemeResult {
                theme: "test-dark".into(),
                variant: "default".into(),
                slots_found: 16,
                luminance_monotone: true,
                wcag_aa: true,
                contrast_ratio: Some(12.5),
                polarity: "dark".into(),
                luminance_ramp: vec![
                    ("base00".into(), 0.03),
                    ("base01".into(), 0.05),
                    ("base05".into(), 0.72),
                    ("base07".into(), 0.88),
                ],
                mono_break_at: None,
            },
            ThemeResult {
                theme: "test-broken".into(),
                variant: "default".into(),
                slots_found: 16,
                luminance_monotone: false,
                wcag_aa: false,
                contrast_ratio: Some(2.1),
                polarity: "dark".into(),
                luminance_ramp: vec![
                    ("base00".into(), 0.03),
                    ("base01".into(), 0.05),
                    ("base05".into(), 0.72),
                    ("base06".into(), 0.60),
                ],
                mono_break_at: Some(3),
            },
        ]
    }

    #[test]
    fn test_json_output() {
        let json = to_json(&sample_results(), "test");
        assert!(json.contains("\"total\": 2"));
        assert!(json.contains("\"luminance_monotone\": 1"));
        assert!(json.contains("test-dark"));
        assert!(json.contains("test-broken"));
        assert!(json.contains("\"mono_break_at\": 3"));
        assert!(json.contains("\"luminance\":"));
    }

    #[test]
    fn test_html_output() {
        let html = to_html(&sample_results(), "test");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Vogix Theme Validation Report"));
        assert!(html.contains("test-dark"));
        assert!(html.contains("test-broken"));
        assert!(html.contains("luminance_ramp"));
    }

    #[test]
    fn test_html_is_self_contained() {
        let html = to_html(&sample_results(), "test");
        assert!(!html.contains("href=\"http"));
        assert!(!html.contains("src=\"http"));
        assert!(html.contains("<style>"));
        assert!(html.contains("<script>"));
    }

    #[test]
    fn test_generate_real_report() {
        use crate::validate_themes::scan_themes;

        let home = std::path::Path::new(env!("HOME"));
        let datasets = [
            ("base16", home.join("Code/github/logger/tinted-schemes/base16")),
            ("base24", home.join("Code/github/logger/tinted-schemes/base24")),
            ("vogix16", home.join("Code/github/logger/vogix16-themes")),
        ];

        let mut all_results = Vec::new();
        for (_, dir) in &datasets {
            if dir.exists() {
                all_results.extend(scan_themes(dir));
            }
        }

        if all_results.is_empty() {
            return; // skip if no datasets
        }

        let html = to_html(&all_results, "all (base16 + base24 + vogix16)");
        let json = to_json(&all_results, "all");

        // Write to docs/ for viewing
        let docs_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
        std::fs::create_dir_all(&docs_dir).unwrap();
        std::fs::write(docs_dir.join("report.html"), &html).unwrap();
        std::fs::write(docs_dir.join("report.json"), &json).unwrap();

        println!(
            "\n  Report generated: {}/report.html ({} themes, {} bytes)\n",
            docs_dir.display(),
            all_results.len(),
            html.len()
        );

        assert!(html.len() > 1000);
        assert!(json.len() > 1000);
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    fn arb_theme_result() -> impl Strategy<Value = ThemeResult> {
        (
            "[a-z]{3,10}",  // theme
            "[a-z]{3,10}",  // variant
            prop::bool::ANY, // monotone
            prop::bool::ANY, // wcag
            0.5f64..21.0,    // contrast
        )
            .prop_map(|(theme, variant, mono, wcag, cr)| ThemeResult {
                theme,
                variant,
                slots_found: 16,
                luminance_monotone: mono,
                wcag_aa: wcag,
                contrast_ratio: Some(cr),
                polarity: if mono { "dark" } else { "light" }.into(),
                luminance_ramp: vec![
                    ("base00".into(), 0.03),
                    ("base05".into(), 0.72),
                ],
                mono_break_at: if mono { None } else { Some(1) },
            })
    }

    proptest! {
        #[test]
        fn prop_json_contains_all_themes(results in proptest::collection::vec(arb_theme_result(), 1..10)) {
            let json = to_json(&results, "test");
            for r in &results {
                prop_assert!(json.contains(&r.theme), "missing theme in JSON");
            }
        }

        #[test]
        fn prop_json_total_matches(results in proptest::collection::vec(arb_theme_result(), 1..10)) {
            let json = to_json(&results, "test");
            let expected = format!("\"total\": {}", results.len());
            prop_assert!(json.contains(&expected), "total mismatch");
        }

        #[test]
        fn prop_html_contains_all_themes(results in proptest::collection::vec(arb_theme_result(), 1..10)) {
            let html = to_html(&results, "test");
            for r in &results {
                prop_assert!(html.contains(&r.theme), "missing theme in HTML");
            }
        }

        #[test]
        fn prop_html_is_valid_structure(results in proptest::collection::vec(arb_theme_result(), 1..5)) {
            let html = to_html(&results, "test");
            prop_assert!(html.starts_with("<!DOCTYPE html>"));
            prop_assert!(html.contains("</html>"));
            prop_assert!(html.contains("<style>"));
            prop_assert!(html.contains("<script>"));
        }

        #[test]
        fn prop_json_monotone_count_correct(results in proptest::collection::vec(arb_theme_result(), 1..10)) {
            let json = to_json(&results, "test");
            let expected_mono = results.iter().filter(|r| r.luminance_monotone).count();
            let expected = format!("\"luminance_monotone\": {}", expected_mono);
            prop_assert!(json.contains(&expected), "monotone count mismatch");
        }
    }
}
