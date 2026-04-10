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
        json.push_str(&format!("      \"scheme\": \"{}\",\n", r.scheme));
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

/// Generate an ontology-driven HTML report.
///
/// The visualization is determined by the ReportSpec:
/// - Encodings from Cleveland-McGill ranking
/// - Geoms from Grammar of Graphics
/// - 3 Shneiderman levels: overview heatmap → sparkline grid → detail cards
/// - Tufte principles: high data-ink ratio, sparklines, small multiples
pub fn to_html(results: &[ThemeResult], dataset_name: &str) -> String {
    let json_data = to_json(results, dataset_name);

    // The report spec drives the visualization
    // Luminance: position (rank 1) + line geom (sparkline)
    // Contrast: position (rank 1) + bar geom (bullet graph)
    // Pass/fail: hue (rank 6 but acceptable for binary nominal)
    // Scheme type: spatial region (facet)

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Vogix Theme Validation Report</title>
<style>
:root {{
  --bg: #0d1117; --fg: #c9d1d9; --fg2: #8b949e; --border: #30363d;
  --pass: #3fb950; --fail: #f85149; --warn: #d29922;
  --accent: #58a6ff; --card: #161b22; --hover: #1c2128;
}}
* {{ margin:0; padding:0; box-sizing:border-box; }}
body {{ font-family: 'Inter', -apple-system, system-ui, sans-serif; background:var(--bg); color:var(--fg); line-height:1.5; }}

/* Tufte: maximize data-ink ratio — minimal chrome */
.container {{ max-width:1400px; margin:0 auto; padding:2rem; }}
header {{ margin-bottom:2rem; border-bottom:1px solid var(--border); padding-bottom:1rem; }}
header h1 {{ font-size:1.3rem; font-weight:600; letter-spacing:-0.02em; }}
header p {{ color:var(--fg2); font-size:0.85rem; }}

/* Level 1: Overview — Shneiderman "overview first" */
.overview {{ display:grid; grid-template-columns:repeat(auto-fit, minmax(120px,1fr)); gap:0.75rem; margin-bottom:2rem; }}
.stat {{ background:var(--card); border:1px solid var(--border); border-radius:4px; padding:0.75rem 1rem; }}
.stat .n {{ font-size:1.8rem; font-weight:700; font-variant-numeric:tabular-nums; }}
.stat .label {{ font-size:0.7rem; color:var(--fg2); text-transform:uppercase; letter-spacing:0.05em; }}
.stat.ok .n {{ color:var(--pass); }}
.stat.bad .n {{ color:var(--fail); }}

/* Heatmap — overview of all themes × axioms */
.heatmap {{ margin-bottom:2rem; }}
.heatmap-grid {{ display:flex; flex-wrap:wrap; gap:1px; background:var(--border); padding:1px; }}
.heatmap-cell {{ width:3px; height:12px; }}
.heatmap-cell.pass {{ background:var(--pass); }}
.heatmap-cell.fail {{ background:var(--fail); }}
.heatmap-cell.partial {{ background:var(--warn); }}
.heatmap-label {{ font-size:0.7rem; color:var(--fg2); margin-bottom:0.25rem; }}

/* Level 2: Explore — Shneiderman "zoom and filter" */
.controls {{ display:flex; gap:0.5rem; margin-bottom:1rem; flex-wrap:wrap; align-items:center; }}
.controls input {{ background:var(--card); border:1px solid var(--border); color:var(--fg); padding:0.35rem 0.7rem; border-radius:3px; font-size:0.8rem; }}
.controls button {{ background:var(--card); border:1px solid var(--border); color:var(--fg); padding:0.35rem 0.7rem; border-radius:3px; cursor:pointer; font-size:0.75rem; }}
.controls button.active {{ background:var(--accent); color:var(--bg); border-color:var(--accent); }}
.controls .geom-switch {{ margin-left:auto; display:flex; gap:0.25rem; }}
.controls .geom-switch button {{ font-size:0.7rem; padding:0.25rem 0.5rem; }}

/* Sparkline grid — Tufte "small multiples" */
.sparkline-grid {{ display:grid; grid-template-columns:repeat(auto-fill, minmax(280px,1fr)); gap:0.5rem; margin-bottom:2rem; }}
.theme-card {{ background:var(--card); border:1px solid var(--border); border-radius:4px; padding:0.6rem 0.8rem; cursor:pointer; transition:border-color 0.15s; }}
.theme-card:hover {{ border-color:var(--accent); }}
.theme-card.fail {{ border-left:3px solid var(--fail); }}
.theme-card.pass {{ border-left:3px solid var(--pass); }}
.theme-card .name {{ font-size:0.8rem; font-weight:600; margin-bottom:0.25rem; display:flex; justify-content:space-between; }}
.theme-card .badges {{ display:flex; gap:0.3rem; }}
.theme-card .badge {{ font-size:0.65rem; padding:0.1rem 0.3rem; border-radius:2px; }}
.badge.ok {{ background:rgba(63,185,80,0.15); color:var(--pass); }}
.badge.err {{ background:rgba(248,81,73,0.15); color:var(--fail); }}
.badge.info {{ background:rgba(88,166,255,0.1); color:var(--accent); }}

/* Sparkline — Cleveland rank 1 (position on common scale) */
.sparkline {{ height:32px; display:flex; align-items:flex-end; gap:1px; margin:0.3rem 0; }}
.spark-bar {{ flex:1; border-radius:1px 1px 0 0; min-width:2px; transition:background 0.15s; }}
.spark-bar.ok {{ background:var(--accent); }}
.spark-bar.brk {{ background:var(--fail); }}

/* Bullet graph — Few: contrast ratio vs thresholds */
.bullet {{ height:8px; background:var(--border); border-radius:1px; position:relative; margin:0.25rem 0; }}
.bullet-fill {{ height:100%; border-radius:1px; position:absolute; left:0; top:0; }}
.bullet-mark {{ position:absolute; top:-2px; width:1px; height:12px; }}
.bullet-mark.aa {{ background:var(--warn); }}
.bullet-mark.aaa {{ background:var(--pass); }}

/* Level 3: Detail — Shneiderman "details on demand" */
.detail {{ display:none; background:var(--bg); border:1px solid var(--border); border-radius:4px; padding:1rem; margin-top:0.5rem; }}
.detail.open {{ display:block; }}
.detail-trace {{ font-family:'JetBrains Mono', monospace; font-size:0.75rem; line-height:1.8; }}
.detail-trace .slot {{ display:inline-block; width:50px; color:var(--fg2); }}
.detail-trace .val {{ font-weight:600; }}
.detail-trace .brk {{ color:var(--fail); }}

/* Table view (alternative geom) */
.table-view {{ display:none; }}
.table-view.active {{ display:block; }}
.sparkline-grid.hidden {{ display:none; }}
table {{ width:100%; border-collapse:collapse; font-size:0.8rem; }}
th {{ text-align:left; padding:0.4rem 0.6rem; border-bottom:1px solid var(--border); cursor:pointer; font-size:0.7rem; text-transform:uppercase; letter-spacing:0.05em; color:var(--fg2); }}
th:hover {{ color:var(--accent); }}
td {{ padding:0.4rem 0.6rem; border-bottom:1px solid var(--border); }}
tr:hover {{ background:var(--hover); }}
</style>
</head>
<body>
<div class="container">
<header>
  <h1>Theme Validation Report</h1>
  <p>Ontology-driven axiom evaluation — <span id="ds"></span> · <span id="n"></span> themes · Encoding: position (Cleveland rank 1) + sparkline (Tufte)</p>
</header>

<!-- Level 1: Overview (Shneiderman) -->
<div class="overview" id="stats"></div>

<div class="heatmap">
  <div class="heatmap-label">All themes — each cell = one theme (green=pass both, red=fail any, yellow=partial)</div>
  <div class="heatmap-grid" id="heatmap"></div>
</div>

<!-- Level 2: Explore (Shneiderman) -->
<div class="controls">
  <input type="text" id="q" placeholder="Search..." oninput="render()">
  <button class="active" onclick="filt('all',this)">All</button>
  <button onclick="filt('pass',this)">Pass</button>
  <button onclick="filt('fail',this)">Fail</button>
  <button onclick="filt('dark',this)">Dark</button>
  <button onclick="filt('light',this)">Light</button>
  <div class="geom-switch">
    <button class="active" onclick="setGeom('card',this)">Cards</button>
    <button onclick="setGeom('table',this)">Table</button>
  </div>
</div>

<div class="sparkline-grid" id="grid"></div>
<div class="table-view" id="tbl"><table><thead><tr>
  <th onclick="sort('theme')">Theme</th>
  <th onclick="sort('variant')">Variant</th>
  <th onclick="sort('polarity')">Polarity</th>
  <th onclick="sort('luminance_monotone')">Mono</th>
  <th onclick="sort('wcag_aa')">WCAG</th>
  <th onclick="sort('contrast_ratio')">CR</th>
  <th>Ramp</th>
</tr></thead><tbody id="tbody"></tbody></table></div>

<script>
const D = {json_data};
let f='all', s='theme', asc=true, geom='card';

function render() {{
  const q = document.getElementById('q').value.toLowerCase();
  let r = [...D.results].filter(x => {{
    if (q && !`${{x.theme}}/${{x.variant}}`.toLowerCase().includes(q)) return false;
    if (f==='pass') return x.luminance_monotone && x.wcag_aa;
    if (f==='fail') return !x.luminance_monotone || !x.wcag_aa;
    if (f==='dark') return x.polarity==='dark';
    if (f==='light') return x.polarity==='light';
    return true;
  }});
  r.sort((a,b) => {{
    let va=a[s], vb=b[s];
    if (typeof va==='boolean') {{ va=va?1:0; vb=vb?1:0; }}
    if (va==null) va=-1; if (vb==null) vb=-1;
    return asc ? (va>vb?1:-1) : (va<vb?1:-1);
  }});

  // Cards (sparkline geom)
  document.getElementById('grid').innerHTML = r.map(x => {{
    const ok = x.luminance_monotone && x.wcag_aa;
    const cls = ok ? 'pass' : 'fail';
    const mx = Math.max(...x.luminance_ramp.map(v=>v.luminance), 0.01);
    const bars = x.luminance_ramp.map((v,j) => {{
      const h = Math.max(2, (v.luminance/mx)*32);
      const c = x.mono_break_at!=null && j>=x.mono_break_at ? 'brk' : 'ok';
      return `<div class="spark-bar ${{c}}" style="height:${{h}}px" title="${{v.slot}}: ${{v.luminance.toFixed(3)}}"></div>`;
    }}).join('');
    const cr = x.contrast_ratio != null ? x.contrast_ratio : 0;
    const crPct = Math.min(cr/21*100, 100);
    const crCol = cr >= 7 ? 'var(--pass)' : cr >= 4.5 ? 'var(--warn)' : 'var(--fail)';
    const brk = x.mono_break_at!=null ? `<span class="badge err">break@${{x.luminance_ramp[x.mono_break_at]?.slot||'?'}}</span>` : '';
    const trace = x.luminance_ramp.map(v =>
      `<span class="slot">${{v.slot}}</span><span class="val${{x.mono_break_at!=null && x.luminance_ramp.indexOf(v)>=x.mono_break_at ? ' brk' : ''}}">${{v.luminance.toFixed(4)}}</span>`
    ).join('<br>');

    return `<div class="theme-card ${{cls}}" onclick="this.querySelector('.detail').classList.toggle('open')">
      <div class="name"><span>${{x.theme}}/${{x.variant}}</span><div class="badges">
        <span class="badge ${{x.luminance_monotone?'ok':'err'}}">mono</span>
        <span class="badge ${{x.wcag_aa?'ok':'err'}}">wcag</span>
        <span class="badge info">${{x.polarity}}</span>
        ${{brk}}
      </div></div>
      <div class="sparkline">${{bars}}</div>
      <div class="bullet"><div class="bullet-fill" style="width:${{crPct}}%;background:${{crCol}}"></div>
        <div class="bullet-mark aa" style="left:${{4.5/21*100}}%" title="AA 4.5:1"></div>
        <div class="bullet-mark aaa" style="left:${{7/21*100}}%" title="AAA 7:1"></div>
      </div>
      <div style="font-size:0.7rem;color:var(--fg2)">CR: ${{cr.toFixed(1)}}:1</div>
      <div class="detail"><div class="detail-trace">${{trace}}</div></div>
    </div>`;
  }}).join('');

  // Table
  document.getElementById('tbody').innerHTML = r.map(x => {{
    const mono = x.luminance_monotone ? '✓' : '✗';
    const wcag = x.wcag_aa ? '✓' : '✗';
    const cr = x.contrast_ratio!=null ? x.contrast_ratio.toFixed(1)+':1' : '—';
    const mx = Math.max(...x.luminance_ramp.map(v=>v.luminance), 0.01);
    const bars = x.luminance_ramp.map((v,j) => {{
      const h = Math.max(1, (v.luminance/mx)*20);
      const c = x.mono_break_at!=null && j>=x.mono_break_at ? 'var(--fail)' : 'var(--accent)';
      return `<div style="display:inline-block;width:8px;height:${{h}}px;background:${{c}};border-radius:1px 1px 0 0;vertical-align:bottom"></div>`;
    }}).join('');
    return `<tr>
      <td>${{x.theme}}</td><td>${{x.variant}}</td><td>${{x.polarity}}</td>
      <td style="color:${{x.luminance_monotone?'var(--pass)':'var(--fail)'}}">${{mono}}</td>
      <td style="color:${{x.wcag_aa?'var(--pass)':'var(--fail)'}}">${{wcag}}</td>
      <td>${{cr}}</td><td style="line-height:0">${{bars}}</td>
    </tr>`;
  }}).join('');
}}

function init() {{
  document.getElementById('ds').textContent = D.meta.dataset;
  document.getElementById('n').textContent = D.summary.total;
  // Stats
  const t = D.summary.total;
  document.getElementById('stats').innerHTML = `
    <div class="stat"><div class="n">${{t}}</div><div class="label">Themes</div></div>
    <div class="stat ok"><div class="n">${{D.summary.luminance_monotone}}</div><div class="label">Monotone (${{(D.summary.luminance_monotone/t*100).toFixed(0)}}%)</div></div>
    <div class="stat ok"><div class="n">${{D.summary.wcag_aa}}</div><div class="label">WCAG AA (${{(D.summary.wcag_aa/t*100).toFixed(0)}}%)</div></div>
    <div class="stat"><div class="n">${{D.summary.dark}}</div><div class="label">Dark</div></div>
    <div class="stat"><div class="n">${{D.summary.light}}</div><div class="label">Light</div></div>
    <div class="stat bad"><div class="n">${{t-D.summary.luminance_monotone}}</div><div class="label">Mono Fail</div></div>
    <div class="stat bad"><div class="n">${{t-D.summary.wcag_aa}}</div><div class="label">WCAG Fail</div></div>
  `;
  // Heatmap — one cell per theme
  document.getElementById('heatmap').innerHTML = D.results.map(x => {{
    const c = x.luminance_monotone && x.wcag_aa ? 'pass' : (!x.luminance_monotone && !x.wcag_aa ? 'fail' : 'partial');
    return `<div class="heatmap-cell ${{c}}" title="${{x.theme}}/${{x.variant}}"></div>`;
  }}).join('');
  render();
}}

function filt(v,el) {{ f=v; document.querySelectorAll('.controls>button').forEach(b=>b.classList.remove('active')); el.classList.add('active'); render(); }}
function sort(c) {{ if(s===c) asc=!asc; else {{ s=c; asc=true; }} render(); }}
function setGeom(g,el) {{
  geom=g;
  document.querySelectorAll('.geom-switch button').forEach(b=>b.classList.remove('active'));
  el.classList.add('active');
  document.getElementById('grid').classList.toggle('hidden', g==='table');
  document.getElementById('tbl').classList.toggle('active', g==='table');
}}

init();
</script>
</div>
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
                scheme: "base16".into(),
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
                scheme: "base16".into(),
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
                scheme: "base16".into(),
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
