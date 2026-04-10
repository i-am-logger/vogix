/// Ontology explorer — self-referential visualization of reasoning traces.
///
/// The explorer visualizes the ontology using the ontology's own theme.
/// Concept nodes light up as axioms evaluate, colored by the active theme.
///
/// Sources:
/// - Mendez et al., "Evonne" (EuroVis 2023): proof tree visualization
/// - Srisuchinnawong et al., "NeuroVis" (2021): neural activation encoding
/// - Wongsuphasawat et al., "TensorFlow Graph Visualizer" (VAST 2017): dataflow
/// - Beck et al., "Dynamic Graph Visualization" (2017): temporal animation
/// - W3C PROV-O: provenance data model
use praxis::ontology::Axiom;
use std::collections::HashMap;

/// A concept node in the ontology graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConceptNode {
    pub id: String,
    pub label: String,
    pub kind: ConceptKind,
}

/// What kind of ontology concept this node represents.
///
/// Source: OWL 2 structural specification + praxis type system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConceptKind {
    /// A type/entity (e.g., ColorSlot, Mode, SchemeType)
    Entity,
    /// A relationship/morphism (e.g., bright-variant-of, mode transition)
    Relationship,
    /// An axiom/rule (e.g., LuminanceMonotonicity, WcagForegroundContrast)
    AxiomNode,
    /// A quality/property (e.g., Polarity, SemanticRole)
    Quality,
    /// A data value (e.g., a specific Rgb color, a luminance value)
    Value,
}

/// An edge connecting two concept nodes.
#[derive(Debug, Clone)]
pub struct ConceptEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub kind: EdgeKind,
}

/// What kind of relationship the edge represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Taxonomic (is-a)
    IsA,
    /// Mereological (has-a / part-of)
    HasA,
    /// Dependency (axiom depends on concept)
    DependsOn,
    /// Evaluation flow (axiom evaluates concept)
    Evaluates,
    /// Produces (axiom produces result)
    Produces,
}

/// Activation state of a node during reasoning trace playback.
///
/// Source: NeuroVis 4-channel encoding (Srisuchinnawong 2021)
/// Mapped to theme colors from the active vogix palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivationState {
    /// Not yet evaluated — dim/inactive
    Inactive,
    /// Currently being evaluated — pulsing/highlighted
    Evaluating,
    /// Axiom satisfied — success color
    Satisfied,
    /// Axiom violated — danger color
    Violated,
    /// Intermediate result — accent color
    Intermediate,
}

/// Maps activation states to semantic color roles from the theme ontology.
///
/// The explorer uses the active theme's functional colors to render itself.
/// This is self-referential: the theming ontology colors the theming visualization.
pub fn activation_to_theme_role(state: ActivationState) -> &'static str {
    match state {
        ActivationState::Inactive => "foreground-comment",    // base03: muted
        ActivationState::Evaluating => "active",              // base0C: highlighted
        ActivationState::Satisfied => "success",              // base08: green
        ActivationState::Violated => "danger",                // base0B: red
        ActivationState::Intermediate => "link",              // base0D: blue
    }
}

/// A single step in a reasoning trace.
///
/// Source: praxis Engine::Trace concept
#[derive(Debug, Clone)]
pub struct TraceStep {
    pub step: usize,
    /// Nodes activated in this step
    pub activated: Vec<String>,
    /// Their activation state
    pub state: ActivationState,
    /// Description of what happened
    pub description: String,
}

/// A complete reasoning trace — sequence of steps from question to answer.
#[derive(Debug, Clone)]
pub struct ReasoningTrace {
    pub question: String,
    pub steps: Vec<TraceStep>,
    pub result: ActivationState,
}

impl ReasoningTrace {
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            steps: Vec::new(),
            result: ActivationState::Inactive,
        }
    }

    pub fn add_step(&mut self, activated: Vec<String>, state: ActivationState, desc: impl Into<String>) {
        self.steps.push(TraceStep {
            step: self.steps.len(),
            activated,
            state,
            description: desc.into(),
        });
    }

    pub fn conclude(&mut self, result: ActivationState) {
        self.result = result;
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// The ontology graph — nodes and edges forming the knowledge structure.
#[derive(Debug, Clone)]
pub struct OntologyGraph {
    pub nodes: Vec<ConceptNode>,
    pub edges: Vec<ConceptEdge>,
}

impl OntologyGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: impl Into<String>, label: impl Into<String>, kind: ConceptKind) {
        self.nodes.push(ConceptNode {
            id: id.into(),
            label: label.into(),
            kind,
        });
    }

    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>, label: impl Into<String>, kind: EdgeKind) {
        self.edges.push(ConceptEdge {
            from: from.into(),
            to: to.into(),
            label: label.into(),
            kind,
        });
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Build the theming ontology graph — all concepts and relationships.
pub fn theming_ontology_graph() -> OntologyGraph {
    let mut g = OntologyGraph::new();

    // Entities
    g.add_node("color_slot", "ColorSlot", ConceptKind::Entity);
    g.add_node("scheme_type", "SchemeType", ConceptKind::Entity);
    g.add_node("polarity", "Polarity", ConceptKind::Entity);
    g.add_node("palette", "Palette", ConceptKind::Entity);
    g.add_node("semantic_role", "SemanticRole", ConceptKind::Entity);
    g.add_node("vogix16", "Vogix16Semantic", ConceptKind::Entity);
    g.add_node("ansi16", "Ansi16Color", ConceptKind::Entity);

    // Values
    for i in 0..=7 {
        g.add_node(format!("base0{}", i), format!("base0{}", i), ConceptKind::Value);
    }
    for c in "89ABCDEF".chars() {
        g.add_node(format!("base0{}", c), format!("base0{}", c), ConceptKind::Value);
    }

    // Axioms
    g.add_node("ax_mono", "LuminanceMonotonicity", ConceptKind::AxiomNode);
    g.add_node("ax_wcag", "WcagForegroundContrast", ConceptKind::AxiomNode);
    g.add_node("ax_bright", "BrightVariantBrighter", ConceptKind::AxiomNode);
    g.add_node("ax_bijection_v", "Vogix16Bijection", ConceptKind::AxiomNode);
    g.add_node("ax_bijection_a", "Ansi16Bijection", ConceptKind::AxiomNode);

    // Qualities
    g.add_node("q_luminance", "RelativeLuminance", ConceptKind::Quality);
    g.add_node("q_contrast", "ContrastRatio", ConceptKind::Quality);

    // Relationships
    g.add_edge("palette", "color_slot", "contains", EdgeKind::HasA);
    g.add_edge("palette", "polarity", "has polarity", EdgeKind::HasA);
    g.add_edge("scheme_type", "color_slot", "defines slots", EdgeKind::HasA);
    g.add_edge("color_slot", "semantic_role", "has role", EdgeKind::HasA);
    g.add_edge("vogix16", "color_slot", "maps to", EdgeKind::IsA);
    g.add_edge("ansi16", "color_slot", "maps to", EdgeKind::IsA);

    // Axiom dependencies
    g.add_edge("ax_mono", "q_luminance", "uses", EdgeKind::DependsOn);
    for i in 0..=7 {
        g.add_edge("ax_mono", format!("base0{}", i), "evaluates", EdgeKind::Evaluates);
    }
    g.add_edge("ax_wcag", "q_contrast", "uses", EdgeKind::DependsOn);
    g.add_edge("ax_wcag", "base00", "evaluates bg", EdgeKind::Evaluates);
    g.add_edge("ax_wcag", "base05", "evaluates fg", EdgeKind::Evaluates);

    // Bright variant axiom evaluates accent slots
    for c in "89ABCDEF".chars() {
        g.add_edge("ax_bright", format!("base0{}", c), "evaluates", EdgeKind::Evaluates);
    }

    // Bijection axioms connect schemes to slots
    g.add_edge("ax_bijection_v", "vogix16", "maps", EdgeKind::Evaluates);
    g.add_edge("ax_bijection_v", "color_slot", "to", EdgeKind::Evaluates);
    g.add_edge("ax_bijection_a", "ansi16", "maps", EdgeKind::Evaluates);
    g.add_edge("ax_bijection_a", "color_slot", "to", EdgeKind::Evaluates);

    g
}

/// Build a sample reasoning trace for monotonicity evaluation.
pub fn monotonicity_trace(palette_name: &str, passes: bool) -> ReasoningTrace {
    let mut t = ReasoningTrace::new(format!("Does {} satisfy luminance monotonicity?", palette_name));

    t.add_step(vec!["palette".into()], ActivationState::Evaluating, "Load palette");
    t.add_step(
        (0..=7).map(|i| format!("base0{}", i)).collect(),
        ActivationState::Evaluating,
        "Extract base00-base07 ramp slots",
    );
    t.add_step(vec!["q_luminance".into()], ActivationState::Evaluating, "Compute relative luminance per slot (WCAG 2.1)");
    t.add_step(vec!["ax_mono".into()], ActivationState::Evaluating, "Check luminance ordering");

    if passes {
        t.add_step(vec!["ax_mono".into()], ActivationState::Satisfied, "Monotonicity satisfied ✓");
        t.conclude(ActivationState::Satisfied);
    } else {
        t.add_step(vec!["ax_mono".into()], ActivationState::Violated, "Monotonicity violated ✗ — break detected");
        t.conclude(ActivationState::Violated);
    }

    t
}

// ── Axioms ──

/// Every concept kind maps to a theme color role.
pub struct ActivationThemeMapped;

impl Axiom for ActivationThemeMapped {
    fn description(&self) -> &str {
        "every activation state maps to a theme color role (self-referential theming)"
    }
    fn holds(&self) -> bool {
        let states = [
            ActivationState::Inactive,
            ActivationState::Evaluating,
            ActivationState::Satisfied,
            ActivationState::Violated,
            ActivationState::Intermediate,
        ];
        states.iter().all(|s| !activation_to_theme_role(*s).is_empty())
    }
}

/// Theming ontology graph is connected (no isolated nodes).
pub struct GraphConnected;

impl Axiom for GraphConnected {
    fn description(&self) -> &str {
        "theming ontology graph has no isolated nodes"
    }
    fn holds(&self) -> bool {
        let g = theming_ontology_graph();
        let connected: std::collections::HashSet<&str> = g
            .edges
            .iter()
            .flat_map(|e| [e.from.as_str(), e.to.as_str()])
            .collect();
        // All nodes should be referenced by at least one edge
        g.nodes.iter().all(|n| connected.contains(n.id.as_str()))
    }
}

/// A reasoning trace has at least 2 steps (start + conclusion).
pub struct TraceMinimalSteps;

impl Axiom for TraceMinimalSteps {
    fn description(&self) -> &str {
        "reasoning trace has at least 2 steps"
    }
    fn holds(&self) -> bool {
        let t = monotonicity_trace("test", true);
        t.step_count() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_5_activation_states() {
        let states = [
            ActivationState::Inactive,
            ActivationState::Evaluating,
            ActivationState::Satisfied,
            ActivationState::Violated,
            ActivationState::Intermediate,
        ];
        assert_eq!(states.len(), 5);
    }

    #[test]
    fn test_activation_theme_mapped() {
        assert!(ActivationThemeMapped.holds());
    }

    #[test]
    fn test_theming_graph_has_nodes() {
        let g = theming_ontology_graph();
        assert!(g.node_count() > 20);
        assert!(g.edge_count() > 15);
    }

    #[test]
    fn test_graph_connected() {
        assert!(GraphConnected.holds());
    }

    #[test]
    fn test_monotonicity_trace_pass() {
        let t = monotonicity_trace("test-dark", true);
        assert!(t.step_count() >= 4);
        assert_eq!(t.result, ActivationState::Satisfied);
    }

    #[test]
    fn test_monotonicity_trace_fail() {
        let t = monotonicity_trace("catppuccin-mocha", false);
        assert_eq!(t.result, ActivationState::Violated);
    }

    #[test]
    fn test_trace_minimal_steps() {
        assert!(TraceMinimalSteps.holds());
    }

    #[test]
    fn test_activation_roles_are_distinct() {
        let states = [
            ActivationState::Inactive,
            ActivationState::Evaluating,
            ActivationState::Satisfied,
            ActivationState::Violated,
            ActivationState::Intermediate,
        ];
        let roles: Vec<_> = states.iter().map(|s| activation_to_theme_role(*s)).collect();
        let unique: std::collections::HashSet<_> = roles.iter().collect();
        assert_eq!(roles.len(), unique.len(), "activation states must map to distinct theme roles");
    }

    #[test]
    fn test_concept_kinds() {
        let g = theming_ontology_graph();
        let entity_count = g.nodes.iter().filter(|n| n.kind == ConceptKind::Entity).count();
        let axiom_count = g.nodes.iter().filter(|n| n.kind == ConceptKind::AxiomNode).count();
        let value_count = g.nodes.iter().filter(|n| n.kind == ConceptKind::Value).count();
        assert!(entity_count >= 5);
        assert!(axiom_count >= 3);
        assert!(value_count >= 8); // base00-base07 at minimum
    }

    #[test]
    fn test_edge_kinds() {
        let g = theming_ontology_graph();
        let has_a = g.edges.iter().filter(|e| matches!(e.kind, EdgeKind::HasA)).count();
        let evaluates = g.edges.iter().filter(|e| matches!(e.kind, EdgeKind::Evaluates)).count();
        assert!(has_a >= 4);
        assert!(evaluates >= 8); // mono evaluates base00-base07
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_trace_always_has_conclusion(passes in prop::bool::ANY) {
            let t = monotonicity_trace("test", passes);
            prop_assert_ne!(t.result, ActivationState::Inactive, "trace must conclude");
        }

        #[test]
        fn prop_activation_roles_non_empty(idx in 0usize..5) {
            let states = [
                ActivationState::Inactive,
                ActivationState::Evaluating,
                ActivationState::Satisfied,
                ActivationState::Violated,
                ActivationState::Intermediate,
            ];
            let role = activation_to_theme_role(states[idx]);
            prop_assert!(!role.is_empty());
        }
    }
}
