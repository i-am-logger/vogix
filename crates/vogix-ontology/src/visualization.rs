/// Visualization ontology — formal model of visual encoding and perception.
///
/// Formalizes the foundational visualization literature as ontological structures
/// with axioms that can be verified and composed.
///
/// Sources:
/// - Bertin, "Semiology of Graphics" (1967): 7 visual variables
/// - Cleveland & McGill, "Graphical Perception" (1984): perceptual accuracy ranking
/// - Munzner, "Visualization Analysis and Design" (2014): channel effectiveness
/// - Shneiderman, "The Eyes Have It" (1996): overview-zoom-filter-detail mantra
/// - Wickham, "Layered Grammar of Graphics" (2010): compositional pipeline
use praxis::category::Entity;
use praxis::ontology::{Axiom, Quality};

// ══════════════════════════════════════════════
// Bertin's Visual Variables
// ══════════════════════════════════════════════

/// The 7 visual variables from Bertin's Semiology of Graphics.
///
/// Source: Bertin (1967), Chapter 2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualVariable {
    Position,
    Size,
    Value, // lightness/darkness
    Color, // hue
    Orientation,
    Shape,
    Texture,
}

impl Entity for VisualVariable {
    fn variants() -> Vec<Self> {
        vec![
            Self::Position, Self::Size, Self::Value, Self::Color,
            Self::Orientation, Self::Shape, Self::Texture,
        ]
    }
}

/// Properties of visual variables per Bertin.
///
/// Source: Bertin (1967), systematized by MacEachren (1995)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariableProperties {
    /// Can group similar items (associative)
    pub associative: bool,
    /// Can isolate one category at a glance (selective)
    pub selective: bool,
    /// Can express ordering (ordered)
    pub ordered: bool,
    /// Can express proportional differences (quantitative)
    pub quantitative: bool,
}

/// Quality: Bertin properties for each visual variable.
#[derive(Debug, Clone)]
pub struct BertinProperties;

impl Quality for BertinProperties {
    type Individual = VisualVariable;
    type Value = VariableProperties;

    fn get(&self, var: &VisualVariable) -> Option<VariableProperties> {
        Some(match var {
            VisualVariable::Position => VariableProperties {
                associative: true, selective: true, ordered: true, quantitative: true,
            },
            VisualVariable::Size => VariableProperties {
                associative: false, selective: true, ordered: true, quantitative: true,
            },
            VisualVariable::Value => VariableProperties {
                associative: false, selective: true, ordered: true, quantitative: false,
            },
            VisualVariable::Color => VariableProperties {
                associative: true, selective: true, ordered: false, quantitative: false,
            },
            VisualVariable::Orientation => VariableProperties {
                associative: true, selective: true, ordered: false, quantitative: false,
            },
            VisualVariable::Shape => VariableProperties {
                associative: true, selective: false, ordered: false, quantitative: false,
            },
            VisualVariable::Texture => VariableProperties {
                associative: true, selective: true, ordered: true, quantitative: false,
            },
        })
    }
}

// ══════════════════════════════════════════════
// Cleveland-McGill Perceptual Ranking
// ══════════════════════════════════════════════

/// Visual encoding tasks ranked by perceptual accuracy.
///
/// Source: Cleveland & McGill (1984), confirmed by Heer & Bostock (2010)
/// Rank 1 = most accurate, Rank 6 = least accurate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerceptualTask {
    /// Position along a common aligned scale (most accurate)
    PositionCommonScale,
    /// Position along non-aligned scales
    PositionNonAligned,
    /// Length, direction, angle
    LengthDirectionAngle,
    /// Area
    Area,
    /// Volume, curvature
    VolumeCurvature,
    /// Shading, color saturation (least accurate)
    ShadingColorSaturation,
}

impl Entity for PerceptualTask {
    fn variants() -> Vec<Self> {
        vec![
            Self::PositionCommonScale,
            Self::PositionNonAligned,
            Self::LengthDirectionAngle,
            Self::Area,
            Self::VolumeCurvature,
            Self::ShadingColorSaturation,
        ]
    }
}

/// Quality: accuracy rank (1 = best, 6 = worst).
#[derive(Debug, Clone)]
pub struct AccuracyRank;

impl Quality for AccuracyRank {
    type Individual = PerceptualTask;
    type Value = u8;

    fn get(&self, task: &PerceptualTask) -> Option<u8> {
        Some(match task {
            PerceptualTask::PositionCommonScale => 1,
            PerceptualTask::PositionNonAligned => 2,
            PerceptualTask::LengthDirectionAngle => 3,
            PerceptualTask::Area => 4,
            PerceptualTask::VolumeCurvature => 5,
            PerceptualTask::ShadingColorSaturation => 6,
        })
    }
}

// ══════════════════════════════════════════════
// Data Types and Encoding Suitability
// ══════════════════════════════════════════════

/// Data measurement levels (Stevens' scale of measurement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataLevel {
    /// Named categories, no order (e.g., scheme type, theme name)
    Nominal,
    /// Ordered categories, no fixed intervals (e.g., severity ranking)
    Ordinal,
    /// Ordered with fixed intervals, no true zero (e.g., temperature in °C)
    Interval,
    /// Ordered with fixed intervals and true zero (e.g., luminance, contrast ratio)
    Ratio,
}

impl Entity for DataLevel {
    fn variants() -> Vec<Self> {
        vec![Self::Nominal, Self::Ordinal, Self::Interval, Self::Ratio]
    }
}

/// Which visual variable is suitable for which data level?
///
/// Source: Bertin (1967), Munzner (2014)
pub fn suitable_encodings(level: DataLevel) -> Vec<VisualVariable> {
    let props = BertinProperties;
    VisualVariable::variants()
        .into_iter()
        .filter(|v| {
            let p = props.get(v).unwrap();
            match level {
                DataLevel::Nominal => p.associative || p.selective,
                DataLevel::Ordinal => p.ordered,
                DataLevel::Interval => p.ordered && p.selective,
                DataLevel::Ratio => p.quantitative,
            }
        })
        .collect()
}

// ══════════════════════════════════════════════
// Geometry Types (Geom layer from Grammar of Graphics)
// ══════════════════════════════════════════════

/// Geometric mark types for data visualization.
///
/// Source: Wickham (2010) — the Geom layer of the Grammar of Graphics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeomType {
    Point,
    Line,
    Bar,
    Area,
    Tile,
    Pie,
    Text,
    Boxplot,
}

impl Entity for GeomType {
    fn variants() -> Vec<Self> {
        vec![
            Self::Point, Self::Line, Self::Bar, Self::Area,
            Self::Tile, Self::Pie, Self::Text, Self::Boxplot,
        ]
    }
}

/// Which geom types suit which data levels?
pub fn suitable_geoms(level: DataLevel) -> Vec<GeomType> {
    match level {
        DataLevel::Nominal => vec![GeomType::Bar, GeomType::Tile, GeomType::Pie, GeomType::Text],
        DataLevel::Ordinal => vec![GeomType::Bar, GeomType::Line, GeomType::Tile, GeomType::Boxplot],
        DataLevel::Interval => vec![GeomType::Line, GeomType::Bar, GeomType::Area, GeomType::Point],
        DataLevel::Ratio => vec![GeomType::Bar, GeomType::Line, GeomType::Point, GeomType::Area, GeomType::Boxplot],
    }
}

/// Quality: primary use case per geom.
#[derive(Debug, Clone)]
pub struct GeomUseCase;

impl Quality for GeomUseCase {
    type Individual = GeomType;
    type Value = &'static str;
    fn get(&self, geom: &GeomType) -> Option<&'static str> {
        Some(match geom {
            GeomType::Point => "individual values, correlations",
            GeomType::Line => "trends, connected sequences",
            GeomType::Bar => "magnitude comparison, ranking",
            GeomType::Area => "cumulative totals, stacked",
            GeomType::Tile => "matrix, heatmap, grid",
            GeomType::Pie => "proportions of whole (area encoding = rank 4, use sparingly)",
            GeomType::Text => "exact values, annotations",
            GeomType::Boxplot => "distribution summary",
        })
    }
}

// ══════════════════════════════════════════════
// Shneiderman's Information Seeking Mantra
// ══════════════════════════════════════════════

/// Interaction levels from the Visual Information Seeking Mantra.
///
/// Source: Shneiderman, "The Eyes Have It" (1996)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionLevel {
    /// See the entire dataset at once — identify patterns
    Overview,
    /// Focus on items of interest — narrow the view
    ZoomAndFilter,
    /// Inspect individual items — see exact values
    DetailsOnDemand,
}

impl Entity for InteractionLevel {
    fn variants() -> Vec<Self> {
        vec![Self::Overview, Self::ZoomAndFilter, Self::DetailsOnDemand]
    }
}

/// Quality: recommended visualization for each interaction level.
#[derive(Debug, Clone)]
pub struct RecommendedVis;

impl Quality for RecommendedVis {
    type Individual = InteractionLevel;
    type Value = &'static str;

    fn get(&self, level: &InteractionLevel) -> Option<&'static str> {
        Some(match level {
            InteractionLevel::Overview => "heatmap or small-multiple grid",
            InteractionLevel::ZoomAndFilter => "sortable/filterable list with sparklines",
            InteractionLevel::DetailsOnDemand => "expanded card with charts and exact values",
        })
    }
}

// ══════════════════════════════════════════════
// Wickham's Grammar of Graphics Layers
// ══════════════════════════════════════════════

/// The 7 layers of the Grammar of Graphics.
///
/// Source: Wickham, "Layered Grammar of Graphics" (2010), extending Wilkinson (2005)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrammarLayer {
    /// Raw dataset
    Data,
    /// Mapping data variables to visual channels
    Aesthetics,
    /// Geometric mark type (point, line, bar, area)
    Geom,
    /// Statistical transformation (identity, count, bin, smooth)
    Stat,
    /// Mapping data range to aesthetic range
    Scale,
    /// Coordinate system (cartesian, polar, geographic)
    Coord,
    /// Conditioning/paneling into subplots
    Facet,
}

impl Entity for GrammarLayer {
    fn variants() -> Vec<Self> {
        vec![
            Self::Data, Self::Aesthetics, Self::Geom, Self::Stat,
            Self::Scale, Self::Coord, Self::Facet,
        ]
    }
}

/// Quality: pipeline order (Data=0, Facet=6).
#[derive(Debug, Clone)]
pub struct PipelineOrder;

impl Quality for PipelineOrder {
    type Individual = GrammarLayer;
    type Value = u8;

    fn get(&self, layer: &GrammarLayer) -> Option<u8> {
        Some(match layer {
            GrammarLayer::Data => 0,
            GrammarLayer::Aesthetics => 1,
            GrammarLayer::Geom => 2,
            GrammarLayer::Stat => 3,
            GrammarLayer::Scale => 4,
            GrammarLayer::Coord => 5,
            GrammarLayer::Facet => 6,
        })
    }
}

// ══════════════════════════════════════════════
// Munzner's Channel Effectiveness
// ══════════════════════════════════════════════

/// Channel types from Munzner's effectiveness ranking.
///
/// Source: Munzner, "Visualization Analysis and Design" (2014), Chapter 5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// For encoding magnitude (quantitative/ordered data)
    Magnitude,
    /// For encoding identity (categorical data)
    Identity,
}

// ══════════════════════════════════════════════
// Axioms
// ══════════════════════════════════════════════

/// Position is the only visual variable that is both associative AND quantitative.
///
/// Source: Bertin (1967) — position is uniquely powerful among visual variables.
pub struct PositionUnique;

impl Axiom for PositionUnique {
    fn description(&self) -> &str {
        "position is the only visual variable that is both associative and quantitative (Bertin 1967)"
    }
    fn holds(&self) -> bool {
        let props = BertinProperties;
        let both: Vec<_> = VisualVariable::variants()
            .into_iter()
            .filter(|v| {
                let p = props.get(v).unwrap();
                p.associative && p.quantitative
            })
            .collect();
        both.len() == 1 && both[0] == VisualVariable::Position
    }
}

/// Cleveland-McGill ranking is strictly ordered (no ties between levels).
///
/// Source: Cleveland & McGill (1984), Table 1
pub struct RankingStrictlyOrdered;

impl Axiom for RankingStrictlyOrdered {
    fn description(&self) -> &str {
        "Cleveland-McGill perceptual ranking is strictly ordered (1984)"
    }
    fn holds(&self) -> bool {
        let rank = AccuracyRank;
        let tasks = PerceptualTask::variants();
        for i in 0..tasks.len() {
            for j in (i + 1)..tasks.len() {
                if rank.get(&tasks[i]) == rank.get(&tasks[j]) {
                    return false;
                }
            }
        }
        true
    }
}

/// Position encoding is the most accurate perceptual task.
///
/// Source: Cleveland & McGill (1984), confirmed by Heer & Bostock (2010)
pub struct PositionMostAccurate;

impl Axiom for PositionMostAccurate {
    fn description(&self) -> &str {
        "position on common scale is the most accurate encoding (Cleveland & McGill 1984)"
    }
    fn holds(&self) -> bool {
        AccuracyRank.get(&PerceptualTask::PositionCommonScale) == Some(1)
    }
}

/// Color/shading is the least accurate for quantitative data.
///
/// Source: Cleveland & McGill (1984)
pub struct ColorLeastAccurate;

impl Axiom for ColorLeastAccurate {
    fn description(&self) -> &str {
        "shading/color saturation is the least accurate for quantitative data (Cleveland & McGill 1984)"
    }
    fn holds(&self) -> bool {
        AccuracyRank.get(&PerceptualTask::ShadingColorSaturation) == Some(6)
    }
}

/// Shneiderman's mantra has exactly 3 levels in order.
pub struct ManthaThreeLevels;

impl Axiom for ManthaThreeLevels {
    fn description(&self) -> &str {
        "Shneiderman's mantra has 3 levels: overview, zoom/filter, details-on-demand (1996)"
    }
    fn holds(&self) -> bool {
        InteractionLevel::variants().len() == 3
    }
}

/// Grammar of Graphics pipeline has 7 layers in strict order.
///
/// Source: Wickham (2010), extending Wilkinson (2005)
pub struct GrammarSevenLayers;

impl Axiom for GrammarSevenLayers {
    fn description(&self) -> &str {
        "Grammar of Graphics has 7 layers in strict pipeline order (Wickham 2010)"
    }
    fn holds(&self) -> bool {
        let order = PipelineOrder;
        let layers = GrammarLayer::variants();
        layers.len() == 7 && layers.windows(2).all(|w| order.get(&w[0]).unwrap() < order.get(&w[1]).unwrap())
    }
}

/// Ratio data should use quantitative encodings (position or size).
///
/// Source: Bertin (1967) — only position and size are quantitative.
pub struct RatioNeedsQuantitative;

impl Axiom for RatioNeedsQuantitative {
    fn description(&self) -> &str {
        "ratio data requires quantitative visual variables (position or size) per Bertin 1967"
    }
    fn holds(&self) -> bool {
        let suitable = suitable_encodings(DataLevel::Ratio);
        // Must include position, must not include shape
        suitable.contains(&VisualVariable::Position)
            && !suitable.contains(&VisualVariable::Shape)
    }
}

/// Nominal data can use color (hue) and shape (associative).
///
/// Source: Bertin (1967) — color is selective, shape is associative. Both work for nominal.
pub struct NominalUsesColorAndShape;

impl Axiom for NominalUsesColorAndShape {
    fn description(&self) -> &str {
        "nominal data can use color hue (selective) and shape (associative) per Bertin 1967"
    }
    fn holds(&self) -> bool {
        let suitable = suitable_encodings(DataLevel::Nominal);
        suitable.contains(&VisualVariable::Color)
            && suitable.contains(&VisualVariable::Shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Entity tests ──

    #[test]
    fn test_7_visual_variables() {
        assert_eq!(VisualVariable::variants().len(), 7);
    }

    #[test]
    fn test_6_perceptual_tasks() {
        assert_eq!(PerceptualTask::variants().len(), 6);
    }

    #[test]
    fn test_4_data_levels() {
        assert_eq!(DataLevel::variants().len(), 4);
    }

    #[test]
    fn test_8_geom_types() {
        assert_eq!(GeomType::variants().len(), 8);
    }

    #[test]
    fn test_ratio_supports_bar_and_line() {
        let geoms = suitable_geoms(DataLevel::Ratio);
        assert!(geoms.contains(&GeomType::Bar));
        assert!(geoms.contains(&GeomType::Line));
    }

    #[test]
    fn test_nominal_supports_pie() {
        let geoms = suitable_geoms(DataLevel::Nominal);
        assert!(geoms.contains(&GeomType::Pie));
    }

    #[test]
    fn test_ratio_no_pie() {
        // Pie charts are for proportions (nominal), not magnitudes (ratio)
        let geoms = suitable_geoms(DataLevel::Ratio);
        assert!(!geoms.contains(&GeomType::Pie));
    }

    #[test]
    fn test_every_geom_has_use_case() {
        let uc = GeomUseCase;
        for geom in GeomType::variants() {
            assert!(uc.get(&geom).is_some());
        }
    }

    #[test]
    fn test_3_interaction_levels() {
        assert_eq!(InteractionLevel::variants().len(), 3);
    }

    #[test]
    fn test_7_grammar_layers() {
        assert_eq!(GrammarLayer::variants().len(), 7);
    }

    // ── Quality tests ──

    #[test]
    fn test_position_is_quantitative() {
        let p = BertinProperties.get(&VisualVariable::Position).unwrap();
        assert!(p.quantitative);
        assert!(p.associative);
        assert!(p.selective);
        assert!(p.ordered);
    }

    #[test]
    fn test_color_not_ordered() {
        let p = BertinProperties.get(&VisualVariable::Color).unwrap();
        assert!(!p.ordered);
        assert!(!p.quantitative);
        assert!(p.selective); // can isolate categories
    }

    #[test]
    fn test_shape_only_associative() {
        let p = BertinProperties.get(&VisualVariable::Shape).unwrap();
        assert!(p.associative);
        assert!(!p.selective);
        assert!(!p.ordered);
        assert!(!p.quantitative);
    }

    #[test]
    fn test_position_rank_1() {
        assert_eq!(AccuracyRank.get(&PerceptualTask::PositionCommonScale), Some(1));
    }

    #[test]
    fn test_shading_rank_6() {
        assert_eq!(AccuracyRank.get(&PerceptualTask::ShadingColorSaturation), Some(6));
    }

    #[test]
    fn test_grammar_pipeline_order() {
        let order = PipelineOrder;
        assert!(order.get(&GrammarLayer::Data).unwrap() < order.get(&GrammarLayer::Aesthetics).unwrap());
        assert!(order.get(&GrammarLayer::Aesthetics).unwrap() < order.get(&GrammarLayer::Geom).unwrap());
        assert!(order.get(&GrammarLayer::Coord).unwrap() < order.get(&GrammarLayer::Facet).unwrap());
    }

    // ── Encoding suitability tests ──

    #[test]
    fn test_ratio_includes_position() {
        let suitable = suitable_encodings(DataLevel::Ratio);
        assert!(suitable.contains(&VisualVariable::Position));
        assert!(suitable.contains(&VisualVariable::Size));
    }

    #[test]
    fn test_nominal_includes_color_and_shape() {
        let suitable = suitable_encodings(DataLevel::Nominal);
        assert!(suitable.contains(&VisualVariable::Color));
        assert!(suitable.contains(&VisualVariable::Shape));
    }

    #[test]
    fn test_ordinal_includes_value_and_position() {
        let suitable = suitable_encodings(DataLevel::Ordinal);
        assert!(suitable.contains(&VisualVariable::Position));
        assert!(suitable.contains(&VisualVariable::Value));
    }

    // ── Axiom tests ──

    #[test]
    fn test_position_unique() {
        assert!(PositionUnique.holds());
    }

    #[test]
    fn test_ranking_strictly_ordered() {
        assert!(RankingStrictlyOrdered.holds());
    }

    #[test]
    fn test_position_most_accurate() {
        assert!(PositionMostAccurate.holds());
    }

    #[test]
    fn test_color_least_accurate() {
        assert!(ColorLeastAccurate.holds());
    }

    #[test]
    fn test_mantra_three_levels() {
        assert!(ManthaThreeLevels.holds());
    }

    #[test]
    fn test_grammar_seven_layers() {
        assert!(GrammarSevenLayers.holds());
    }

    #[test]
    fn test_ratio_needs_quantitative() {
        assert!(RatioNeedsQuantitative.holds());
    }

    #[test]
    fn test_nominal_uses_color_and_shape() {
        assert!(NominalUsesColorAndShape.holds());
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_every_variable_has_properties(idx in 0usize..7) {
            let var = &VisualVariable::variants()[idx];
            prop_assert!(BertinProperties.get(var).is_some());
        }

        #[test]
        fn prop_every_task_has_rank(idx in 0usize..6) {
            let task = &PerceptualTask::variants()[idx];
            let rank = AccuracyRank.get(task).unwrap();
            prop_assert!(rank >= 1 && rank <= 6);
        }

        #[test]
        fn prop_ranks_unique(a in 0usize..6, b in 0usize..6) {
            if a != b {
                let tasks = PerceptualTask::variants();
                let rank = AccuracyRank;
                prop_assert_ne!(rank.get(&tasks[a]), rank.get(&tasks[b]));
            }
        }

        #[test]
        fn prop_every_data_level_has_encodings(idx in 0usize..4) {
            let level = DataLevel::variants()[idx];
            let suitable = suitable_encodings(level);
            prop_assert!(!suitable.is_empty(), "every data level should have at least one suitable encoding");
        }

        #[test]
        fn prop_higher_data_level_fewer_encodings(_dummy in 0u8..1) {
            // Ratio (most constrained) should have fewer suitable encodings than Nominal (least)
            let nominal = suitable_encodings(DataLevel::Nominal).len();
            let ratio = suitable_encodings(DataLevel::Ratio).len();
            prop_assert!(ratio <= nominal, "ratio should have fewer encodings than nominal");
        }

        #[test]
        fn prop_grammar_layers_strictly_increasing(idx in 0usize..6) {
            let layers = GrammarLayer::variants();
            let order = PipelineOrder;
            let a = order.get(&layers[idx]).unwrap();
            let b = order.get(&layers[idx + 1]).unwrap();
            prop_assert!(a < b, "pipeline layers must be strictly ordered");
        }
    }
}
