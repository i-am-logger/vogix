//! Modal input engine — drives the mode statechart from user input.
//!
//! The mode graph (loaded via [`super::schema`]) defines *which* modes exist and
//! which transitions are legal. This engine defines the *dynamics*: given the current
//! [`InputState`] and a [`ModeTransition`], it produces the next state, gated by
//! the [`ValidTransition`] precondition. It is the runtime an input daemon feeds
//! raw key events into.
//!
//! # Why this exists (the failure it designs out)
//!
//! "Stuck in a mode" — where a key does the wrong thing because the user is in a
//! mode they didn't realise — is a textbook **mode error** (Norman 1981). The
//! cure, per Raskin, is the **quasimode**: a mode held only by a sustained
//! physical action that *reverts automatically when released*, so the user
//! cannot be stranded in it. This engine makes that structural:
//! [`ModeTransition::ReleaseHold`] and [`ModeTransition::ExitToRoot`] are
//! *always* legal and the root (default) state is *always* reachable — a
//! property of well-formed statecharts (Harel 1987). There is no reachable state
//! from which the user cannot return to root. The bug is not patched; it is made
//! unrepresentable.
//!
//! # Literature
//! - Norman, D. A. (1981) Categorization of Action Slips, Psychological Review
//!   88(1), pp. 1-15 — defines the mode error this design prevents.
//! - Raskin, J. (2000) The Humane Interface, Addison-Wesley, Ch. 3 §3-2 — the
//!   quasimode / spring-loaded mode (hold = active, release = exit).
//! - Harel, D. (1987) Statecharts: A Visual Formalism for Complex Systems,
//!   Science of Computer Programming 8(3), pp. 231-274 — the default (root) state
//!   is always reachable in a well-formed hierarchical statechart.
//! - vi modal model (IEEE/Open Group POSIX.1 "vi") — a keystroke's meaning is a
//!   function of the active mode (prior art for modal key reinterpretation).
//!
//! This is the imperative runtime statechart over the config-loaded `ModeGraph`
//! (runtime, `String`-keyed modes). It complements praxis's categorical
//! `applied::hmi::input::ontology` (the compile-time proven model: free category,
//! runtime functors, no-stuck terminal); the two are kept in step.

use pr4xis::engine::{Action, Engine, EngineError, Precondition, Situation};
use pr4xis::logic::proof::{Counterexample, SimpleCounterexample, SimpleProof, Verdict};
use pr4xis::ontology::Axiom;
use pr4xis::ontology::meta::{Citation, Label, ModulePath, OntologyName, Provenance};
use pr4xis_domains::applied::hmi::input::modes::{ModeGraph, ModeId};

/// Build a [`Provenance`] for a precondition proof/counterexample, preserving
/// the rule name and the human-readable reason text.
fn meta(name: &'static str, description: &str) -> Provenance {
    Provenance {
        name: OntologyName::new_static(name),
        description: Label::new(description.to_string()),
        citation: Citation::EMPTY,
        module_path: ModulePath::new_static(module_path!()),
    }
}

/// The live interaction state: the current mode plus whether it is *sticky*.
///
/// `sticky = false` is a **quasimode** (Raskin §3-2): held by a sustained action
/// (e.g. holding CapsLock); releasing it reverts to root. `sticky = true` is a
/// **locked mode**: entered by a discrete toggle and exited only by an explicit
/// action — a release does nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputState {
    pub mode: ModeId,
    pub sticky: bool,
}

impl InputState {
    /// The initial state: at the root mode, not sticky.
    pub fn root(graph: &ModeGraph) -> Self {
        Self {
            mode: graph.root.clone(),
            sticky: false,
        }
    }
}

/// The input statechart has no terminal state — root is always re-enterable.
impl Situation for InputState {}

/// A transition the user can request. The legality of [`EnterMomentary`],
/// [`EnterSticky`] and [`Switch`] is checked against the [`ModeGraph`];
/// [`ReleaseHold`] and [`ExitToRoot`] are always legal (the no-stuck guarantee).
///
/// [`EnterMomentary`]: ModeTransition::EnterMomentary
/// [`EnterSticky`]: ModeTransition::EnterSticky
/// [`Switch`]: ModeTransition::Switch
/// [`ReleaseHold`]: ModeTransition::ReleaseHold
/// [`ExitToRoot`]: ModeTransition::ExitToRoot
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeTransition {
    /// Enter a mode as a quasimode (held). Release reverts to root. Raskin §3-2.
    EnterMomentary(ModeId),
    /// Enter a mode as a locked/sticky mode (toggled). Release is a no-op.
    EnterSticky(ModeId),
    /// Switch to another mode while keeping the current sticky/momentary kind
    /// (e.g. move ↔ resize). Must be a legal transition.
    // Constructed by the device layer (Phase 2b); exercised by the property tests.
    #[allow(dead_code)]
    Switch(ModeId),
    /// The held trigger was released. In a quasimode this reverts to root; in a
    /// locked mode it does nothing.
    ReleaseHold,
    /// Explicit return to the root/default mode (Esc, or toggling a locked mode
    /// off). Always legal — Harel's reachable default state.
    ExitToRoot,
}

impl Action for ModeTransition {
    type Sit = InputState;
}

/// Precondition: a mode-*entering* transition must be an edge in the graph; a
/// mode-*leaving* transition is unconditionally permitted.
///
/// The asymmetry is the heart of the design: you may only enter modes the
/// statechart sanctions, but you may *always* leave — so no input can strand the
/// user (Raskin §3-2; Harel 1987 reachable default).
pub struct ValidTransition {
    pub graph: ModeGraph,
}

impl Precondition<ModeTransition> for ValidTransition {
    fn check(&self, state: &InputState, action: &ModeTransition) -> Verdict {
        let entering = match action {
            ModeTransition::EnterMomentary(to)
            | ModeTransition::EnterSticky(to)
            | ModeTransition::Switch(to) => Some(to),
            // Leaving is always legal — the no-stuck guarantee.
            ModeTransition::ReleaseHold | ModeTransition::ExitToRoot => None,
        };
        match entering {
            None => Ok(Box::new(SimpleProof::new(meta(
                "valid_mode_transition",
                "leaving a mode is always permitted",
            )))),
            Some(to) if self.graph.is_valid_transition(&state.mode, to) => {
                Ok(Box::new(SimpleProof::new(meta(
                    "valid_mode_transition",
                    &format!("{} → {}", state.mode.0, to.0),
                ))))
            }
            Some(to) => Err(Box::new(SimpleCounterexample::new(meta(
                "valid_mode_transition",
                &format!("no legal edge {} → {}", state.mode.0, to.0),
            )))),
        }
    }
}

/// Build an input engine seeded at the root mode, validated against `graph`.
pub fn new_input_engine(graph: ModeGraph) -> Engine<ModeTransition> {
    let initial = InputState::root(&graph);
    let root = graph.root.clone();
    let apply = move |state: &InputState, action: &ModeTransition| {
        let next = match action {
            ModeTransition::EnterMomentary(to) => InputState {
                mode: to.clone(),
                sticky: false,
            },
            ModeTransition::EnterSticky(to) => InputState {
                mode: to.clone(),
                sticky: true,
            },
            ModeTransition::Switch(to) => InputState {
                mode: to.clone(),
                sticky: state.sticky,
            },
            // Quasimode reverts on release; a locked mode ignores it.
            ModeTransition::ReleaseHold => {
                if state.sticky {
                    state.clone()
                } else {
                    InputState {
                        mode: root.clone(),
                        sticky: false,
                    }
                }
            }
            ModeTransition::ExitToRoot => InputState {
                mode: root.clone(),
                sticky: false,
            },
        };
        Ok::<InputState, Box<dyn Counterexample>>(next)
    };
    Engine::new(initial, vec![Box::new(ValidTransition { graph })], apply)
}

/// Apply a transition, returning the engine whether or not the action was
/// accepted. [`Engine::next`] is move-based and returns the engine inside
/// [`EngineError`] on rejection; a rejected transition leaves the situation
/// unchanged, which is exactly the "you can try anything, illegal moves are
/// no-ops" semantics we want.
pub fn drive(engine: Engine<ModeTransition>, action: ModeTransition) -> Engine<ModeTransition> {
    match engine.next(action) {
        Ok(e) => e,
        Err(EngineError::Violated { engine, .. })
        | Err(EngineError::LogicalError { engine, .. }) => engine,
    }
}

// ── Axioms ──────────────────────────────────────────────────────────────────
//
// The pinned praxis 0.6 `Axiom` trait is `holds() -> bool`; the citations live in
// the descriptions and the strongest evidence is the property tests below.

/// A quasimode reverts to root when released — no mode lock-in.
///
/// This is Raskin's defining property of a quasimode and the structural cure for
/// the mode error (Norman 1981): from a momentary mode, releasing the trigger
/// returns to the default state.
pub struct QuasimodeRevertsToRoot {
    pub graph: ModeGraph,
}

impl Axiom for QuasimodeRevertsToRoot {
    fn verify(&self) -> Verdict {
        // For every mode reachable from root by a single momentary entry,
        // ReleaseHold must return to root.
        for t in &self.graph.transitions {
            if t.from == self.graph.root {
                let eng = new_input_engine(self.graph.clone());
                let eng = drive(eng, ModeTransition::EnterMomentary(t.to.clone()));
                let eng = drive(eng, ModeTransition::ReleaseHold);
                if eng.situation().mode != self.graph.root {
                    return Err(Box::new(SimpleCounterexample::new(self.meta())));
                }
            }
        }
        Ok(Box::new(SimpleProof::new(self.meta())))
    }

    pr4xis::axiom_meta!(
        "QuasimodeRevertsToRoot",
        "a momentary (quasi)mode reverts to root on release — no mode lock-in \
         (Raskin 2000 §3-2; Norman 1981)",
        "Raskin (2000) The Humane Interface §3-2; Norman (1981) Categorization of Action Slips, Psychological Review 88(1)"
    );
}

/// From any reachable state, `ExitToRoot` returns to the root mode.
///
/// The runtime counterpart of Harel's reachable-default-state property: the root
/// is always re-enterable, so no sequence of inputs can strand the user.
pub struct ExitAlwaysReachesRoot {
    pub graph: ModeGraph,
}

impl Axiom for ExitAlwaysReachesRoot {
    fn verify(&self) -> Verdict {
        for mode in self.graph.modes.keys() {
            let eng = new_input_engine(self.graph.clone());
            // Try to lock into `mode` (sticky, so a stray release can't pre-empt),
            // then assert ExitToRoot returns to root regardless.
            let eng = drive(eng, ModeTransition::EnterSticky(mode.clone()));
            let eng = drive(eng, ModeTransition::ExitToRoot);
            if eng.situation().mode != self.graph.root {
                return Err(Box::new(SimpleCounterexample::new(self.meta())));
            }
        }
        Ok(Box::new(SimpleProof::new(self.meta())))
    }

    pr4xis::axiom_meta!(
        "ExitAlwaysReachesRoot",
        "ExitToRoot returns to root from any reachable state — no dead states \
         (Harel 1987)",
        "Harel (1987) Statecharts, Science of Computer Programming 8(3) pp. 231-274"
    );
}

/// The input invariants over a given (loaded) graph, for diagnostics
/// (`vogix input check`).
pub fn axioms(graph: ModeGraph) -> Vec<Box<dyn Axiom>> {
    vec![
        Box::new(QuasimodeRevertsToRoot {
            graph: graph.clone(),
        }),
        Box::new(ExitAlwaysReachesRoot { graph }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use pr4xis_domains::applied::hmi::input::modes::ModeProperties;
    use proptest::prelude::*;

    /// A small graph built directly (the engine is generic over any graph; the
    /// real graph is loaded from config — see `super::super::schema`).
    /// app(root) ↔ desktop; desktop ↔ move; move/desktop → app.
    fn graph() -> ModeGraph {
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
            ModeId::new("move"),
            ModeProperties {
                catchall: true,
                parent: Some(ModeId::new("app")),
                depth: 1,
            },
        );
        g.add_transition(ModeId::new("app"), ModeId::new("desktop"));
        g.add_transition(ModeId::new("desktop"), ModeId::new("app"));
        g.add_transition(ModeId::new("desktop"), ModeId::new("move"));
        g.add_transition(ModeId::new("move"), ModeId::new("app"));
        g
    }

    fn child_of_root(g: &ModeGraph) -> ModeId {
        g.transitions
            .iter()
            .find(|t| t.from == g.root)
            .map(|t| t.to.clone())
            .expect("graph has a transition out of root")
    }

    #[test]
    fn starts_at_root_not_sticky() {
        let g = graph();
        let eng = new_input_engine(g.clone());
        assert_eq!(eng.situation().mode, g.root);
        assert!(!eng.situation().sticky);
    }

    #[test]
    fn momentary_enter_then_release_returns_to_root() {
        let g = graph();
        let child = child_of_root(&g);
        let eng = new_input_engine(g.clone());
        let eng = eng
            .next(ModeTransition::EnterMomentary(child.clone()))
            .expect("entering a child of root is a legal transition");
        assert_eq!(eng.situation().mode, child);
        let eng = eng.next(ModeTransition::ReleaseHold).unwrap();
        assert_eq!(
            eng.situation().mode,
            g.root,
            "quasimode must revert on release"
        );
    }

    #[test]
    fn sticky_enter_ignores_release_but_exits_explicitly() {
        let g = graph();
        let child = child_of_root(&g);
        let eng = new_input_engine(g.clone());
        let eng = eng
            .next(ModeTransition::EnterSticky(child.clone()))
            .unwrap();
        let eng = eng.next(ModeTransition::ReleaseHold).unwrap();
        assert_eq!(eng.situation().mode, child, "locked mode ignores release");
        let eng = eng.next(ModeTransition::ExitToRoot).unwrap();
        assert_eq!(eng.situation().mode, g.root, "explicit exit always works");
    }

    #[test]
    fn illegal_transition_is_rejected() {
        let g = graph();
        let bogus = ModeId::new("does-not-exist");
        let eng = new_input_engine(g.clone());
        match eng.next(ModeTransition::EnterMomentary(bogus)) {
            Err(EngineError::Violated { engine, .. }) => assert_eq!(
                engine.situation().mode,
                g.root,
                "rejected action leaves state unchanged"
            ),
            _ => panic!("illegal transition should be rejected"),
        }
    }

    #[test]
    fn axioms_hold() {
        let q = QuasimodeRevertsToRoot { graph: graph() };
        assert!(q.verify().is_ok(), "{}", q.description().as_str());
        let e = ExitAlwaysReachesRoot { graph: graph() };
        assert!(e.verify().is_ok(), "{}", e.description().as_str());
    }

    proptest! {
        /// THE anti-mode-error property (Norman 1981 / Raskin §3-2): no matter
        /// what sequence of (possibly illegal, possibly random) transitions the
        /// user throws at the engine, a final ExitToRoot always lands at root.
        #[test]
        fn prop_exit_to_root_always_reaches_root(
            steps in proptest::collection::vec(0u8..5, 0..50)
        ) {
            let g = graph();
            let modes: Vec<ModeId> = g.modes.keys().cloned().collect();
            let mut eng = new_input_engine(g.clone());
            for (i, code) in steps.into_iter().enumerate() {
                let target = modes[i % modes.len()].clone();
                let action = match code {
                    0 => ModeTransition::EnterMomentary(target),
                    1 => ModeTransition::EnterSticky(target),
                    2 => ModeTransition::Switch(target),
                    3 => ModeTransition::ReleaseHold,
                    _ => ModeTransition::ExitToRoot,
                };
                // Illegal transitions are rejected by the precondition; `drive`
                // recovers the (unchanged) engine in that case.
                eng = drive(eng, action);
            }
            let eng = eng.next(ModeTransition::ExitToRoot).unwrap();
            prop_assert_eq!(eng.situation().mode.clone(), g.root,
                "ExitToRoot must reach root from every reachable state");
        }

        /// The engine never applies an illegal mode entry: after any accepted
        /// EnterMomentary, the new mode was a legal target from where we were.
        #[test]
        fn prop_only_legal_modes_are_entered(
            idx in proptest::collection::vec(any::<prop::sample::Index>(), 0..40)
        ) {
            let g = graph();
            let modes: Vec<ModeId> = g.modes.keys().cloned().collect();
            let mut eng = new_input_engine(g.clone());
            for ix in idx {
                let target = ix.get(&modes).clone();
                let before = eng.situation().mode.clone();
                let legal = g.is_valid_transition(&before, &target);
                eng = match eng.next(ModeTransition::EnterMomentary(target.clone())) {
                    Ok(e) => {
                        prop_assert!(legal, "engine entered an illegal mode {:?} from {:?}", target, before);
                        prop_assert_eq!(e.situation().mode.clone(), target.clone());
                        e
                    }
                    Err(EngineError::Violated { engine, .. }) => {
                        prop_assert!(!legal);
                        prop_assert_eq!(engine.situation().mode.clone(), before, "rejected action must not mutate state");
                        engine
                    }
                    Err(EngineError::LogicalError { engine, .. }) => engine,
                };
            }
        }
    }
}
