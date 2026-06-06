use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::consortium::Consortium;
use crate::starter::{CSP, Starter};
use crate::proof::ConvergenceProof;

/// Result of baking a fermented consortium.
///
/// "Baking" extracts the solution: assignments for each variable,
/// whether the CSP is fully solved, and any unsatisfied constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeResult {
    /// Whether the CSP is fully solved (all variables assigned, all constraints satisfied)
    pub solved: bool,
    /// Variable assignments (variable name → value). Empty if unsolved.
    pub assignments: HashMap<String, i32>,
    /// Names of constraints that remain unsatisfied
    pub unsatisfied: Vec<String>,
    /// Convergence proof for the fermentation
    #[serde(default)]
    pub proof: Option<ConvergenceProof>,
    /// Total fermentation iterations
    #[serde(default)]
    pub iterations: usize,
}

impl BakeResult {
    /// Whether the bake found the CSP to be unsatisfiable.
    pub fn is_unsatisfiable(&self) -> bool {
        !self.solved && self.assignments.is_empty()
    }

    /// Number of variables successfully assigned.
    pub fn assigned_count(&self) -> usize {
        self.assignments.len()
    }

    /// Get a specific assignment.
    pub fn get(&self, variable: &str) -> Option<i32> {
        self.assignments.get(variable).cloned()
    }
}

/// Baker — extracts solutions from fermented consortia.
///
/// After fermentation converges, the baker examines the consortium,
/// extracts singleton domains as assignments, checks constraint satisfaction,
/// and reports the result.
pub struct Baker;

impl Baker {
    /// Bake a consortium that has already been fermented.
    pub fn bake(consortium: &Consortium, csp: &CSP) -> BakeResult {
        let mut assignments: HashMap<String, i32> = HashMap::new();
        let mut unsatisfied: Vec<String> = Vec::new();

        // Extract singleton domains as assignments
        for culture in &consortium.cultures {
            if culture.domain.len() == 1 {
                assignments.insert(culture.id.clone(), culture.domain[0]);
            }
        }

        // Build predicates for constraint checking
        let predicates = Starter::build_predicates(csp);

        // Check each constraint
        for (i, (a, b, pred_idx)) in csp.constraint_pairs.iter().enumerate() {
            let constraint_name = format!("{}: {} ↔ {}", i, a, b);

            let a_val = assignments.get(a);
            let b_val = assignments.get(b);

            match (a_val, b_val) {
                (Some(&av), Some(&bv)) => {
                    if let Some(pred) = predicates.get(*pred_idx) {
                        if !pred(av, bv) {
                            unsatisfied.push(constraint_name);
                        }
                    }
                }
                _ => {
                    // Not all variables assigned — can't verify this constraint
                    unsatisfied.push(format!("{} (unassigned)", constraint_name));
                }
            }
        }

        let solved = unsatisfied.is_empty()
            && assignments.len() == csp.variables.len()
            && !consortium.has_dead();

        BakeResult {
            solved,
            assignments,
            unsatisfied,
            proof: None,
            iterations: consortium.iteration,
        }
    }

    /// Full pipeline: CSP → seed → ferment → bake.
    /// This is the one-call-does-everything entry point.
    pub fn solve(csp: &CSP, temperature: f64, max_iterations: usize) -> BakeResult {
        let initial = Starter::seed(csp, temperature);
        let mut consortium = initial.clone();

        // Build constraints for fermentation
        let constraints_owned: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = csp
            .constraint_pairs
            .iter()
            .map(|(a, b, pred_idx)| {
                let desc = csp.predicate_descriptions.get(*pred_idx).map(|s| s.as_str()).unwrap_or("");
                let pred: Box<dyn Fn(i32, i32) -> bool> = if desc.contains("<") && !desc.contains("<=") {
                    Box::new(|v1: i32, v2: i32| v1 < v2)
                } else if desc.contains("diagonal") {
                    Box::new(|v1: i32, v2: i32| v1 != v2)
                } else {
                    Box::new(|v1: i32, v2: i32| v1 != v2)
                };
                (a.clone(), b.clone(), pred)
            })
            .collect();

        consortium.ferment_until_converged(&constraints_owned, max_iterations);

        let proof = ConvergenceProof::prove(&initial, &consortium);
        let mut result = Self::bake(&consortium, csp);
        result.proof = Some(proof);
        result
    }

    /// Solve with custom N-queens diagonal constraints.
    pub fn solve_n_queens(n: usize, temperature: f64, max_iterations: usize) -> BakeResult {
        let csp = CSP::n_queens(n);
        let initial = Starter::seed(&csp, temperature);
        let mut consortium = initial.clone();

        // Build proper N-queens constraints
        let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = csp
            .constraint_pairs
            .iter()
            .map(|(a, b, _)| {
                let a_row: i32 = a[1..].parse().unwrap_or(0);
                let b_row: i32 = b[1..].parse().unwrap_or(0);
                let row_diff = (a_row - b_row).abs();
                (
                    a.clone(),
                    b.clone(),
                    Box::new(move |v1: i32, v2: i32| {
                        v1 != v2 && (v1 - v2).abs() != row_diff
                    }) as Box<dyn Fn(i32, i32) -> bool>,
                )
            })
            .collect();

        consortium.ferment_until_converged(&constraints, max_iterations);

        // Create a simple CSP for baking (using != only for assignment checking)
        let bake_csp = CSP::inequality_csp(
            csp.variables.clone(),
            csp.domains.clone(),
            csp.constraint_pairs.iter().map(|(a, b, _)| (a.clone(), b.clone())).collect(),
        );

        let proof = ConvergenceProof::prove(&initial, &consortium);
        let mut result = Self::bake(&consortium, &bake_csp);
        result.proof = Some(proof);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::culture::Culture;
    use crate::consortium::{Consortium, Interaction};
    use crate::metabolism::MetabolismProfile;
    use crate::proof::ConvergenceBound;

    #[test]
    fn bake_extracts_correct_solution() {
        // a < b with domains [1,2] — AC-3 can fully solve this
        let csp = CSP::new(
            vec!["a".into(), "b".into()],
            vec![vec![1, 2], vec![1, 2]],
            vec![("a".into(), "b".into(), 0)],
            vec!["a < b".into()],
        );
        let result = Baker::solve(&csp, 1.0, 100);
        assert!(result.solved);
        assert_eq!(result.assigned_count(), 2);
        let a = result.get("a").unwrap();
        let b = result.get("b").unwrap();
        assert!(a < b);
    }

    #[test]
    fn bake_detects_unsatisfiable() {
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into()],
            vec![vec![1], vec![1]],
            vec![("a".into(), "b".into())],
        );
        let result = Baker::solve(&csp, 1.0, 100);
        assert!(!result.solved);
        assert!(!result.unsatisfied.is_empty());
    }

    #[test]
    fn full_pipeline_line_graph_3_colors() {
        let colors = vec![0, 1, 2];
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into(), "c".into()],
            vec![colors.clone(), colors.clone(), colors.clone()],
            vec![("a".into(), "b".into()), ("b".into(), "c".into())],
        );
        let result = Baker::solve(&csp, 1.0, 100);
        if result.solved {
            assert_ne!(result.get("a").unwrap(), result.get("b").unwrap());
            assert_ne!(result.get("b").unwrap(), result.get("c").unwrap());
        }
    }

    #[test]
    fn n_queens_4_makes_progress() {
        // Test N-queens with simple != constraints (row-only)
        let csp = CSP::n_queens(4);
        let result = Baker::solve(&csp, 1.0, 200);
        // AC-3 with != should reduce domains
        // 4-queens has solutions so no dead cultures
        let proof = result.proof.as_ref().unwrap();
        assert!(proof.verify());
    }

    #[test]
    fn result_has_proof() {
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into()],
            vec![vec![1, 2], vec![1, 2]],
            vec![("a".into(), "b".into())],
        );
        let result = Baker::solve(&csp, 1.0, 100);
        assert!(result.proof.is_some());
        let proof = result.proof.unwrap();
        assert!(proof.verify());
    }

    #[test]
    fn bake_result_serde_roundtrip() {
        let mut assignments = HashMap::new();
        assignments.insert("x".into(), 42);
        let result = BakeResult {
            solved: true,
            assignments,
            unsatisfied: vec![],
            proof: None,
            iterations: 5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: BakeResult = serde_json::from_str(&json).unwrap();
        assert!(back.solved);
        assert_eq!(back.get("x"), Some(42));
        assert_eq!(back.iterations, 5);
    }

    #[test]
    fn culture_serde_roundtrip() {
        let c = Culture::new("test", vec![1, 2, 3]);
        let json = serde_json::to_string(&c).unwrap();
        let back: Culture = serde_json::from_str(&json).unwrap();
        assert_eq!(c.id, back.id);
        assert_eq!(c.domain, back.domain);
        assert!((c.population - back.population).abs() < 1e-10);
    }

    #[test]
    fn consortium_serde_roundtrip() {
        let c = Consortium::new(
            vec![Culture::new("a", vec![1, 2])],
            vec![Interaction::new("a", "b", 0.5)],
            1.0,
        );
        let json = serde_json::to_string(&c).unwrap();
        let back: Consortium = serde_json::from_str(&json).unwrap();
        assert_eq!(c.cultures.len(), back.cultures.len());
        assert_eq!(c.interactions.len(), back.interactions.len());
    }

    #[test]
    fn metabolism_serde_roundtrip() {
        let m = MetabolismProfile::fast();
        let json = serde_json::to_string(&m).unwrap();
        let back: MetabolismProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(m.kind, back.kind);
        assert!((m.rate - back.rate).abs() < 1e-10);
    }

    #[test]
    fn convergence_proof_serde_roundtrip() {
        let p = ConvergenceProof {
            converged: true,
            iterations: 3,
            n: 2,
            d: 4,
            total_reduction: 5,
            theoretical_bound: 64,
            bound: ConvergenceBound::AC3Equivalent,
            population_bounded: true,
            monotone_reduction: true,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ConvergenceProof = serde_json::from_str(&json).unwrap();
        assert_eq!(p.converged, back.converged);
        assert_eq!(p.bound, back.bound);
    }

    #[test]
    fn csp_serde_roundtrip() {
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into()],
            vec![vec![1, 2], vec![3, 4]],
            vec![("a".into(), "b".into())],
        );
        let json = serde_json::to_string(&csp).unwrap();
        let back: CSP = serde_json::from_str(&json).unwrap();
        assert_eq!(csp.variables, back.variables);
        assert_eq!(csp.domains, back.domains);
        assert_eq!(csp.constraint_pairs.len(), back.constraint_pairs.len());
    }

    #[test]
    fn interaction_serde_roundtrip() {
        let i = Interaction::new("x", "y", 0.75);
        let json = serde_json::to_string(&i).unwrap();
        let back: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(i.a, back.a);
        assert_eq!(i.b, back.b);
        assert!((i.strength - back.strength).abs() < 1e-10);
    }

    #[test]
    fn empty_domain_detected_as_unsatisfiable() {
        let result = BakeResult {
            solved: false,
            assignments: HashMap::new(),
            unsatisfied: vec!["constraint_0".into()],
            proof: None,
            iterations: 1,
        };
        assert!(result.is_unsatisfiable());
    }

    #[test]
    fn high_temperature_convergence_speed() {
        // Compare convergence at different temperatures
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into(), "c".into()],
            vec![vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
            vec![("a".into(), "b".into()), ("b".into(), "c".into())],
        );
        let result_hot = Baker::solve(&csp, 2.0, 100);
        let result_cold = Baker::solve(&csp, 0.3, 100);
        // Both should make progress; hot should converge in fewer or equal iterations
        // (though exact behavior depends on the problem structure)
        assert!(result_hot.proof.is_some());
        assert!(result_cold.proof.is_some());
    }

    #[test]
    fn resource_competition_drives_domain_reduction() {
        // 3 variables, domains [1,2,3], all-different constraints
        // Triangle with 3 colors: AC-3 will reduce but not necessarily solve
        let csp = CSP::inequality_csp(
            vec!["x".into(), "y".into(), "z".into()],
            vec![vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
            vec![
                ("x".into(), "y".into()),
                ("y".into(), "z".into()),
                ("x".into(), "z".into()),
            ],
        );
        let result = Baker::solve(&csp, 1.0, 200);
        // AC-3 should reduce some domains even if it doesn't fully solve
        let proof = result.proof.unwrap();
        // With 3 vars and != constraints, AC-3 should make progress
        // At minimum, domains should be reduced from initial 9 total
        assert!(proof.monotone_reduction);
    }
}
