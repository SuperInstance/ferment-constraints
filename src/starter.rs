use serde::{Deserialize, Serialize};

use crate::consortium::{Consortium, ConsortiumBuilder, Interaction};
use crate::culture::Culture;

/// A constraint satisfaction problem specification.
///
/// Variables become cultures, constraints become resource overlaps.
/// The CSP is the "recipe" — the starter transforms it into a living consortium.
#[derive(Serialize, Deserialize)]
pub struct CSP {
    /// Variable names
    pub variables: Vec<String>,
    /// Domain for each variable (parallel to variables)
    pub domains: Vec<Vec<i32>>,
    /// Binary constraints as (var_a, var_b, predicate_index)
    /// The actual predicates are stored separately for Serde compatibility
    pub constraint_pairs: Vec<(String, String, usize)>,
    /// Named predicates — maps index to description for serialization
    #[serde(default)]
    pub predicate_descriptions: Vec<String>,
}

impl CSP {
    /// Create a new CSP with the given variables, domains, and constraints.
    /// Constraints use a predicate index into registered predicates.
    pub fn new(
        variables: Vec<String>,
        domains: Vec<Vec<i32>>,
        constraint_pairs: Vec<(String, String, usize)>,
        predicate_descriptions: Vec<String>,
    ) -> Self {
        assert_eq!(variables.len(), domains.len());
        Self {
            variables,
            domains,
            constraint_pairs,
            predicate_descriptions,
        }
    }

    /// Create a simple CSP with inequality constraints (e.g., graph coloring).
    pub fn inequality_csp(variables: Vec<String>, domains: Vec<Vec<i32>>, edges: Vec<(String, String)>) -> Self {
        let constraint_pairs: Vec<_> = edges.into_iter().map(|(a, b)| (a, b, 0)).collect();
        Self {
            variables,
            domains,
            constraint_pairs,
            predicate_descriptions: vec!["a != b".into()],
        }
    }

    /// Create an N-queens CSP (simplified — row conflicts only for small N).
    pub fn n_queens(n: usize) -> Self {
        let vars: Vec<String> = (0..n).map(|i| format!("q{}", i)).collect();
        let domain: Vec<Vec<i32>> = (0..n).map(|_| (0..n as i32).collect()).collect();
        let mut constraints = vec![];
        // No two queens in same row, or on same diagonal
        for i in 0..n {
            for j in (i + 1)..n {
                // Row constraint + diagonal constraint combined
                constraints.push((format!("q{}", i), format!("q{}", j), 1));
            }
        }
        Self {
            variables: vars,
            domains: domain,
            constraint_pairs: constraints,
            predicate_descriptions: vec![
                "a != b".into(),
                "not same row or diagonal".into(),
            ],
        }
    }

    /// Number of variables.
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    /// Number of constraints.
    pub fn num_constraints(&self) -> usize {
        self.constraint_pairs.len()
    }
}

/// Seeds a consortium from a CSP specification.
///
/// The starter is where fermentation begins: you feed it a CSP recipe, and it
/// produces a living consortium of cultures ready to ferment toward a solution.
pub struct Starter;

impl Starter {
    /// Create a consortium from a CSP, ready for fermentation.
    /// Temperature controls the overall metabolism rate.
    pub fn seed(csp: &CSP, temperature: f64) -> Consortium {
        let cultures: Vec<Culture> = csp
            .variables
            .iter()
            .zip(csp.domains.iter())
            .map(|(name, domain)| Culture::new(name.clone(), domain.clone()))
            .collect();

        let interactions: Vec<Interaction> = csp
            .constraint_pairs
            .iter()
            .map(|(a, b, _)| {
                // Interaction strength based on domain overlap
                let dom_a = csp.domains.get(
                    csp.variables.iter().position(|v| v == a).unwrap_or(0),
                );
                let dom_b = csp.domains.get(
                    csp.variables.iter().position(|v| v == b).unwrap_or(0),
                );
                let strength = if let (Some(da), Some(db)) = (dom_a, dom_b) {
                    let shared = da.iter().filter(|v| db.contains(v)).count();
                    let total = (da.len() + db.len()).max(1);
                    1.0 - (shared as f64 / total as f64) // Less overlap = stronger constraint
                } else {
                    0.5
                };
                Interaction::new(a, b, strength)
            })
            .collect();

        ConsortiumBuilder::new()
            .cultures(cultures)
            .interactions(interactions)
            .temperature(temperature)
            .build()
    }

    /// Build a predicate lookup from common constraint types.
    /// Returns a vector of boxed predicates indexed by predicate index.
    pub fn build_predicates(csp: &CSP) -> Vec<Box<dyn Fn(i32, i32) -> bool>> {
        csp.predicate_descriptions
            .iter()
            .map(|desc| {
                if desc.contains("!=") {
                    Box::new(|a: i32, b: i32| a != b) as Box<dyn Fn(i32, i32) -> bool>
                } else if desc.contains("<") && !desc.contains("<=") {
                    Box::new(|a: i32, b: i32| a < b) as Box<dyn Fn(i32, i32) -> bool>
                } else if desc.contains("<=") {
                    Box::new(|a: i32, b: i32| a <= b) as Box<dyn Fn(i32, i32) -> bool>
                } else if desc.contains(">") && !desc.contains(">=") {
                    Box::new(|a: i32, b: i32| a > b) as Box<dyn Fn(i32, i32) -> bool>
                } else if desc.contains(">=") {
                    Box::new(|a: i32, b: i32| a >= b) as Box<dyn Fn(i32, i32) -> bool>
                } else if desc.contains("diagonal") {
                    Box::new(|a: i32, b: i32| a != b) as Box<dyn Fn(i32, i32) -> bool>
                } else {
                    Box::new(|a: i32, b: i32| a != b) as Box<dyn Fn(i32, i32) -> bool>
                }
            })
            .collect()
    }

    /// Seed and immediately ferment a CSP to convergence.
    /// Returns the consortium after fermentation.
    pub fn seed_and_ferment(
        csp: &CSP,
        temperature: f64,
        max_iterations: usize,
    ) -> Consortium {
        let mut consortium = Self::seed(csp, temperature);
        let predicates = Self::build_predicates(csp);

        // Convert constraint_pairs + predicates into a usable form
        let _constraints: Vec<_> = csp
            .constraint_pairs
            .iter()
            .map(|(a, b, pred_idx)| {
                let pred = predicates.get(*pred_idx).unwrap();
                (a.clone(), b.clone(), pred.as_ref())
            })
            .collect();

        // We need owned predicates for ferment_until_converged
        // Rebuild with owned closures
        let constraints_owned: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = csp
            .constraint_pairs
            .iter()
            .map(|(a, b, pred_idx)| {
                let desc = csp.predicate_descriptions.get(*pred_idx).map(|s| s.as_str()).unwrap_or("");
                let pred: Box<dyn Fn(i32, i32) -> bool> = if desc.contains("!=") || desc.contains("diagonal") {
                    Box::new(|v1: i32, v2: i32| v1 != v2)
                } else if desc.contains("<") {
                    Box::new(|v1: i32, v2: i32| v1 < v2)
                } else {
                    Box::new(|v1: i32, v2: i32| v1 != v2)
                };
                (a.clone(), b.clone(), pred)
            })
            .collect();

        consortium.ferment_until_converged(&constraints_owned, max_iterations);
        consortium
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_creates_culture_per_variable() {
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into(), "c".into()],
            vec![vec![1, 2], vec![1, 2], vec![1, 2]],
            vec![("a".into(), "b".into()), ("b".into(), "c".into())],
        );
        let consortium = Starter::seed(&csp, 1.0);
        assert_eq!(consortium.cultures.len(), 3);
        assert_eq!(consortium.interactions.len(), 2);
    }

    #[test]
    fn n_queens_small() {
        let csp = CSP::n_queens(4);
        assert_eq!(csp.num_variables(), 4);
        assert_eq!(csp.num_constraints(), 6); // C(4,2)
    }

    #[test]
    fn interaction_strength_based_on_overlap() {
        let csp = CSP::inequality_csp(
            vec!["a".into(), "b".into()],
            vec![vec![1, 2, 3], vec![3, 4, 5]],
            vec![("a".into(), "b".into())],
        );
        let consortium = Starter::seed(&csp, 1.0);
        // Domains share only {3}, so strength should be high
        assert!(consortium.interactions[0].strength > 0.5);
    }

    #[test]
    fn full_pipeline_inequality_csp() {
        let csp = CSP::inequality_csp(
            vec!["x".into(), "y".into()],
            vec![vec![1, 2], vec![1, 2]],
            vec![("x".into(), "y".into())],
        );
        let consortium = Starter::seed_and_ferment(&csp, 1.0, 50);
        // Should converge (each variable has at least one value)
        assert!(!consortium.has_dead());
    }
}
