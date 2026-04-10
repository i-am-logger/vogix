# Theme Validation Report — Design Specification

## Sources

- Tufte, "Visual Display of Quantitative Information" (1983): data-ink ratio, small multiples, sparklines
- Bertin, "Semiology of Graphics" (1967): 7 visual variables, data type suitability
- Cleveland & McGill, "Graphical Perception" (1984): perceptual accuracy ranking
- Munzner, "Visualization Analysis and Design" (2014): channel effectiveness, nested model
- Few, "Information Dashboard Design" (2006): preattentive attributes, bullet graphs
- Shneiderman, "The Eyes Have It" (1996): overview → zoom/filter → details-on-demand
- Wickham, "Layered Grammar of Graphics" (2010): data → aesthetics → geom → stat → scale → coord → facet

## Design Principles

1. **Overview first** (Shneiderman): heatmap of 464 themes × axioms, sorted by failure count
2. **Position encodes quantity** (Cleveland rank 1): luminance as Y-position, not color
3. **Hue for categories only** (Bertin): pass=green, fail=red, scheme type as facet
4. **Small multiples** (Tufte): sparkline ramps in a grid, same layout repeated
5. **Bullet graphs for ratios** (Few): contrast ratio vs 4.5:1 and 7:1 thresholds
6. **Details on demand** (Shneiderman): click to expand per-theme trace

## Three-Level Hierarchy

### Level 1: Dashboard Overview
- Header: total themes, pass/fail counts, % compliant
- Heatmap: themes (rows) × axioms (columns), colored by outcome
- Grouped by scheme type (base16 / base24 / vogix16)
- Sorted by failure count descending

### Level 2: Pattern Exploration  
- Sparkline grid: one luminance ramp per theme, aligned Y-axis
- 3D option: luminance landscape (X=slot, Y=luminance, Z=theme)
- Contrast ratio dot plot with threshold lines
- Filter/sort controls

### Level 3: Theme Detail
- Full luminance ramp chart with break point highlighted
- Color swatches showing actual theme colors
- Bullet graphs for each contrast pair
- Axiom checklist with pass/fail + exact values

## Channel Assignment

| Data | Type | Encoding | Cleveland Rank |
|------|------|----------|---------------|
| Luminance (L*) | Quantitative | Y-position | 1 |
| Color role | Ordered | X-position | 1 |
| Pass/fail | Categorical | Hue (green/red) | 6 (acceptable for binary) |
| Scheme type | Categorical | Spatial region (facet) | 1 |
| Contrast ratio | Quantitative | Bar length (bullet) | 2 |
| Theme color | Literal data | Color swatch | N/A (data IS the encoding) |
| Failure severity | Ordered | Sort order | 1 |
