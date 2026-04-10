/// Desktop mode ontology — defines WHAT modes and transitions ARE,
/// not which specific modes exist. The user configures the mode graph.
///
/// The ontology provides:
/// - ModeGraph: a validated set of modes + transitions
/// - Axioms: no dead states, root reachable, passthrough consistency
/// - Qualities: depth, parent, catchall behavior
///
/// The default vogix modes (App, Desktop, Arrange, Theme, Console) are
/// provided as a convenience but are not the only valid configuration.
///
/// Sources:
/// - vim modal editing (Normal/Insert/Visual/Command)
/// - Hyprland submaps (https://wiki.hypr.land/Configuring/Binds/#submaps)
/// - macOS Mission Control (Desktop mode inspiration)
use praxis::ontology::Axiom;
use std::collections::{HashMap, HashSet};
/// A mode definition — part of a user-configured mode graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModeId(pub String);

impl ModeId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}
/// Properties of a mode.
#[derive(Debug, Clone)]
pub struct ModeProperties {
    /// Does this mode swallow unbound keys? (false = passthrough to apps/terminal)
    pub catchall: bool,
    /// Parent mode (where Escape returns to). None = root mode.
    pub parent: Option<ModeId>,
    /// Depth in the hierarchy (root = 0).
    pub depth: u8,
}
/// A directed transition between two modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Transition {
    pub from: ModeId,
    pub to: ModeId,
}
/// A validated mode graph — the set of all modes and valid transitions.
///
/// Built from user configuration, validated against axioms.
#[derive(Debug, Clone)]
pub struct ModeGraph {
    pub root: ModeId,
    pub modes: HashMap<ModeId, ModeProperties>,
    pub transitions: HashSet<Transition>,
}

impl ModeGraph {
    /// Create a new mode graph with a root mode.
    pub fn new(root: ModeId) -> Self {
        let mut modes = HashMap::new();
        modes.insert(
            root.clone(),
            ModeProperties {
                catchall: false,
                parent: None,
                depth: 0,
            },
        );
        Self {
            root,
            modes,
            transitions: HashSet::new(),
        }
    }

    /// Add a mode to the graph.
    pub fn add_mode(&mut self, id: ModeId, props: ModeProperties) {
        self.modes.insert(id, props);
    }

    /// Add a transition between two modes.
    pub fn add_transition(&mut self, from: ModeId, to: ModeId) {
        self.transitions.insert(Transition { from, to });
    }

    /// Is a transition valid in this graph?
    pub fn is_valid_transition(&self, from: &ModeId, to: &ModeId) -> bool {
        self.transitions.contains(&Transition {
            from: from.clone(),
            to: to.clone(),
        })
    }

    /// Get modes reachable from a given mode (BFS).
    pub fn reachable_from(&self, start: &ModeId) -> HashSet<ModeId> {
        let mut visited = HashSet::new();
        let mut frontier = vec![start.clone()];
        while let Some(current) = frontier.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            for t in &self.transitions {
                if t.from == current && !visited.contains(&t.to) {
                    frontier.push(t.to.clone());
                }
            }
        }
        visited
    }

    /// Validate this graph against all axioms.
    pub fn validate(&self) -> Vec<String> {
        let mut failures = Vec::new();

        let axioms: Vec<Box<dyn Axiom>> = vec![
            Box::new(NoDeadStates {
                graph: self.clone(),
            }),
            Box::new(RootReachable {
                graph: self.clone(),
            }),
            Box::new(RootNoParent {
                graph: self.clone(),
            }),
        ];

        for axiom in &axioms {
            if !axiom.holds() {
                failures.push(axiom.description().to_string());
            }
        }

        failures
    }
}

// ── Default vogix mode graph ──
/// Build the default vogix mode graph.
///
/// App (root) ↔ Desktop ↔ Arrange
///                       ↔ Theme
/// Any → Console, Console → App
pub fn default_mode_graph() -> ModeGraph {
    let mut g = ModeGraph::new(ModeId::new("app"));

    g.add_mode(
        ModeId::new("desktop"),
        ModeProperties {
            catchall: true,
            parent: Some(ModeId::new("app")),
            depth: 1,
        },
    );
    g.add_mode(
        ModeId::new("arrange"),
        ModeProperties {
            catchall: true,
            parent: Some(ModeId::new("desktop")),
            depth: 2,
        },
    );
    g.add_mode(
        ModeId::new("theme"),
        ModeProperties {
            catchall: true,
            parent: Some(ModeId::new("desktop")),
            depth: 2,
        },
    );
    g.add_mode(
        ModeId::new("console"),
        ModeProperties {
            catchall: false, // passthrough to tmux
            parent: Some(ModeId::new("app")),
            depth: 1,
        },
    );

    // App ↔ Desktop
    g.add_transition(ModeId::new("app"), ModeId::new("desktop"));
    g.add_transition(ModeId::new("desktop"), ModeId::new("app"));
    // Desktop → sub-modes
    g.add_transition(ModeId::new("desktop"), ModeId::new("arrange"));
    g.add_transition(ModeId::new("desktop"), ModeId::new("theme"));
    // Sub-modes → Desktop
    g.add_transition(ModeId::new("arrange"), ModeId::new("desktop"));
    g.add_transition(ModeId::new("theme"), ModeId::new("desktop"));
    // Any → Console
    for mode in ["app", "desktop", "arrange", "theme"] {
        g.add_transition(ModeId::new(mode), ModeId::new("console"));
    }
    // Console → App
    g.add_transition(ModeId::new("console"), ModeId::new("app"));

    g
}

// ── Axioms ──
/// Every mode can reach the root (no dead states).
pub struct NoDeadStates {
    pub graph: ModeGraph,
}

impl Axiom for NoDeadStates {
    fn description(&self) -> &str {
        "every mode can reach root (no dead states)"
    }
    fn holds(&self) -> bool {
        for mode_id in self.graph.modes.keys() {
            if *mode_id == self.graph.root {
                continue;
            }
            // Walk parent chain to root
            let mut current = mode_id.clone();
            let mut steps = 0;
            loop {
                if current == self.graph.root {
                    break;
                }
                if steps > 10 {
                    return false; // cycle or too deep
                }
                match self.graph.modes.get(&current) {
                    Some(props) => match &props.parent {
                        Some(parent) => {
                            current = parent.clone();
                            steps += 1;
                        }
                        None => return false, // non-root with no parent
                    },
                    None => return false, // mode not in graph
                }
            }
        }
        true
    }
}
/// Root mode is reachable from every mode via transitions.
pub struct RootReachable {
    pub graph: ModeGraph,
}

impl Axiom for RootReachable {
    fn description(&self) -> &str {
        "root is reachable from every mode via transitions"
    }
    fn holds(&self) -> bool {
        for mode_id in self.graph.modes.keys() {
            let reachable = self.graph.reachable_from(mode_id);
            if !reachable.contains(&self.graph.root) {
                return false;
            }
        }
        true
    }
}
/// Root mode has no parent.
pub struct RootNoParent {
    pub graph: ModeGraph,
}

impl Axiom for RootNoParent {
    fn description(&self) -> &str {
        "root mode has no parent"
    }
    fn holds(&self) -> bool {
        self.graph
            .modes
            .get(&self.graph.root)
            .map(|p| p.parent.is_none())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_graph() -> ModeGraph {
        default_mode_graph()
    }

    // ── Structure tests ──

    #[test]
    fn test_default_has_5_modes() {
        assert_eq!(default_graph().modes.len(), 5);
    }

    #[test]
    fn test_default_root_is_app() {
        assert_eq!(default_graph().root, ModeId::new("app"));
    }

    #[test]
    fn test_valid_transitions() {
        let g = default_graph();
        assert!(g.is_valid_transition(&ModeId::new("app"), &ModeId::new("desktop")));
        assert!(g.is_valid_transition(&ModeId::new("desktop"), &ModeId::new("arrange")));
        assert!(g.is_valid_transition(&ModeId::new("app"), &ModeId::new("console")));
    }

    #[test]
    fn test_invalid_transitions() {
        let g = default_graph();
        // Can't skip Desktop
        assert!(!g.is_valid_transition(&ModeId::new("app"), &ModeId::new("arrange")));
        assert!(!g.is_valid_transition(&ModeId::new("app"), &ModeId::new("theme")));
        // Can't go between sub-modes directly
        assert!(!g.is_valid_transition(&ModeId::new("arrange"), &ModeId::new("theme")));
    }

    #[test]
    fn test_console_reachable_from_all() {
        let g = default_graph();
        for mode in ["app", "desktop", "arrange", "theme"] {
            assert!(
                g.is_valid_transition(&ModeId::new(mode), &ModeId::new("console")),
                "Console not reachable from {}",
                mode
            );
        }
    }

    #[test]
    fn test_all_modes_can_reach_app() {
        let g = default_graph();
        for mode_id in g.modes.keys() {
            let reachable = g.reachable_from(mode_id);
            assert!(
                reachable.contains(&ModeId::new("app")),
                "{:?} cannot reach app",
                mode_id
            );
        }
    }

    // ── Quality tests ──

    #[test]
    fn test_catchall() {
        let g = default_graph();
        assert!(!g.modes[&ModeId::new("app")].catchall);
        assert!(g.modes[&ModeId::new("desktop")].catchall);
        assert!(g.modes[&ModeId::new("arrange")].catchall);
        assert!(g.modes[&ModeId::new("theme")].catchall);
        assert!(!g.modes[&ModeId::new("console")].catchall);
    }

    #[test]
    fn test_depth() {
        let g = default_graph();
        assert_eq!(g.modes[&ModeId::new("app")].depth, 0);
        assert_eq!(g.modes[&ModeId::new("desktop")].depth, 1);
        assert_eq!(g.modes[&ModeId::new("console")].depth, 1);
        assert_eq!(g.modes[&ModeId::new("arrange")].depth, 2);
        assert_eq!(g.modes[&ModeId::new("theme")].depth, 2);
    }

    #[test]
    fn test_parent() {
        let g = default_graph();
        assert_eq!(g.modes[&ModeId::new("app")].parent, None);
        assert_eq!(
            g.modes[&ModeId::new("desktop")].parent,
            Some(ModeId::new("app"))
        );
        assert_eq!(
            g.modes[&ModeId::new("arrange")].parent,
            Some(ModeId::new("desktop"))
        );
    }

    // ── Axiom tests ──

    #[test]
    fn test_no_dead_states() {
        let g = default_graph();
        assert!(NoDeadStates { graph: g }.holds());
    }

    #[test]
    fn test_root_reachable() {
        let g = default_graph();
        assert!(RootReachable { graph: g }.holds());
    }

    #[test]
    fn test_root_no_parent() {
        let g = default_graph();
        assert!(RootNoParent { graph: g }.holds());
    }

    #[test]
    fn test_validate_default_passes() {
        let g = default_graph();
        let failures = g.validate();
        assert!(failures.is_empty(), "failures: {:?}", failures);
    }

    #[test]
    fn test_dead_state_detected() {
        let mut g = ModeGraph::new(ModeId::new("root"));
        g.add_mode(
            ModeId::new("orphan"),
            ModeProperties {
                catchall: false,
                parent: None, // no parent!
                depth: 1,
            },
        );
        assert!(!NoDeadStates { graph: g }.holds());
    }

    #[test]
    fn test_unreachable_detected() {
        let mut g = ModeGraph::new(ModeId::new("root"));
        g.add_mode(
            ModeId::new("island"),
            ModeProperties {
                catchall: false,
                parent: Some(ModeId::new("root")),
                depth: 1,
            },
        );
        // No transitions to/from island
        assert!(!RootReachable { graph: g }.holds());
    }

    // ── Custom mode graph test ──

    #[test]
    fn test_custom_graph_validates() {
        let mut g = ModeGraph::new(ModeId::new("normal"));
        g.add_mode(
            ModeId::new("insert"),
            ModeProperties {
                catchall: false,
                parent: Some(ModeId::new("normal")),
                depth: 1,
            },
        );
        g.add_mode(
            ModeId::new("visual"),
            ModeProperties {
                catchall: true,
                parent: Some(ModeId::new("normal")),
                depth: 1,
            },
        );
        g.add_transition(ModeId::new("normal"), ModeId::new("insert"));
        g.add_transition(ModeId::new("insert"), ModeId::new("normal"));
        g.add_transition(ModeId::new("normal"), ModeId::new("visual"));
        g.add_transition(ModeId::new("visual"), ModeId::new("normal"));

        let failures = g.validate();
        assert!(failures.is_empty(), "failures: {:?}", failures);
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_root_always_in_reachable(mode_name in "[a-z]{3,8}") {
            // Any graph with bidirectional transitions to root validates
            let mut g = ModeGraph::new(ModeId::new("root"));
            g.add_mode(ModeId::new(mode_name.clone()), ModeProperties {
                catchall: false,
                parent: Some(ModeId::new("root")),
                depth: 1,
            });
            g.add_transition(ModeId::new("root"), ModeId::new(mode_name.clone()));
            g.add_transition(ModeId::new(mode_name), ModeId::new("root"));
            prop_assert!(g.validate().is_empty());
        }

        #[test]
        fn prop_default_graph_all_modes_reach_app(idx in 0usize..5) {
            let g = default_mode_graph();
            let modes: Vec<_> = g.modes.keys().cloned().collect();
            let mode = &modes[idx];
            let reachable = g.reachable_from(mode);
            prop_assert!(reachable.contains(&ModeId::new("app")));
        }

        #[test]
        fn prop_orphan_node_fails_validation(name in "[a-z]{3,8}") {
            // A node with no parent (and not root) should fail NoDeadStates
            let mut g = ModeGraph::new(ModeId::new("root"));
            g.add_mode(ModeId::new(name.clone()), ModeProperties {
                catchall: false,
                parent: None, // orphan
                depth: 1,
            });
            g.add_transition(ModeId::new("root"), ModeId::new(name.clone()));
            g.add_transition(ModeId::new(name), ModeId::new("root"));
            let failures = g.validate();
            prop_assert!(!failures.is_empty(), "orphan should fail validation");
        }

        #[test]
        fn prop_island_node_fails_reachability(name in "[a-z]{3,8}") {
            // A node with parent but no transitions should fail RootReachable
            let mut g = ModeGraph::new(ModeId::new("root"));
            g.add_mode(ModeId::new(name.clone()), ModeProperties {
                catchall: false,
                parent: Some(ModeId::new("root")),
                depth: 1,
            });
            // No transitions added — island
            let axiom = RootReachable { graph: g };
            prop_assert!(!axiom.holds());
        }

        #[test]
        fn prop_chain_graph_validates(n in 1usize..5) {
            // A chain: root → a → b → ... with back-edges validates
            let mut g = ModeGraph::new(ModeId::new("root"));
            let mut prev = "root".to_string();
            for i in 0..n {
                let name = format!("m{}", i);
                g.add_mode(ModeId::new(name.clone()), ModeProperties {
                    catchall: true,
                    parent: Some(ModeId::new(prev.clone())),
                    depth: (i + 1) as u8,
                });
                g.add_transition(ModeId::new(prev.clone()), ModeId::new(name.clone()));
                g.add_transition(ModeId::new(name.clone()), ModeId::new(prev.clone()));
                prev = name;
            }
            prop_assert!(g.validate().is_empty());
        }

        #[test]
        fn prop_reachable_set_includes_self(idx in 0usize..5) {
            // reachable_from(x) always includes x
            let g = default_mode_graph();
            let modes: Vec<_> = g.modes.keys().cloned().collect();
            let mode = &modes[idx];
            let reachable = g.reachable_from(mode);
            prop_assert!(reachable.contains(mode));
        }
    }
}
