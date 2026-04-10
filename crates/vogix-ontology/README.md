# vogix-ontology

Formal ontology for the Vogix UX subsystem — color theming, modal interaction, and surface rendering. Built on [praxis](https://github.com/i-am-logger/praxis) category theory primitives.

Will move to praxis once the API stabilizes.

## Modules

- **`modes`** — Modal interaction ontology. User-configurable mode graphs with axioms (no dead states, root reachable). Default: App↔Desktop↔Arrange/Theme, Console overlay.
- **`surfaces`** — Surface functor ontology. Abstract rendering targets (terminals, window borders, LEDs, shaders). Functors map palettes to surface-specific configs. Theme change proven as natural transformation.
- **`validate_themes`** — Empirical evaluation against real theme datasets.

## Theme Validation Results

Evaluated against 464 themes from three scheme types:

| Dataset | Themes | Dark | Light | Luminance Monotonicity | WCAG AA Contrast |
|---------|--------|------|-------|----------------------|-----------------|
| Base16 | 243 | 170 | 67 | 168 (69%) | 221 (91%) |
| Base24 | 171 | 151 | 20 | 89 (52%) | 147 (86%) |
| Vogix16 | 50 | 25 | 25 | 45 (90%) | 46 (92%) |
| **Total** | **464** | **346** | **112** | **302 (65%)** | **414 (89%)** |

Key findings:
- **35% of themes violate luminance monotonicity** — the base00→base07 ramp is not strictly ordered
- **11% fail WCAG AA** — insufficient contrast between foreground and background
- **Base24 has lower compliance than Base16** (52% vs 69% monotonicity)
- **Vogix16 has highest compliance** (90% monotonicity, 92% WCAG AA) — curated design system

## Tests

```
cargo test -p vogix-ontology
```

48 tests including property-based testing (proptest):
- Mode graph axiom verification (valid and invalid graphs)
- Surface functor determinism and information preservation
- Natural transformation across arbitrary palettes
- Theme validation against real datasets

## Sources

Color science: ITU-R BT.709-6, IEC 61966-2-1, WCAG 2.1, ECMA-48, Porter & Duff 1984

Theming: tinted-theming Base16/Base24 specs, Vogix16 design system

Modal interaction: Harel Statecharts 1987, Thimbleby Matrix Algebra 2004, Beaudouin-Lafon Instrumental Interaction 2000

Category theory: Mac Lane "Categories for the Working Mathematician" 1971
