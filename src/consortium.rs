use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::culture::Culture;
use crate::metabolism::MetabolismProfile;

/// An interaction between two cultures — models a binary constraint as resource overlap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub a: String,
    pub b: String,
    /// Strength of the interaction (0..1). Derived from constraint tightness.
    pub strength: f64,
}

impl Interaction {
    pub fn new(a: impl Into<String>, b: impl Into<String>, strength: f64) -> Self {
        Self {
            a: a.into(),
            b: b.into(),
            strength: strength.max(0.0).min(1.0),
        }
    }

    pub fn involves(&self, id: &str) -> bool {
        self.a == id || self.b == id
    }

    pub fn partner(&self, id: &str) -> Option<&str> {
        if self.a == id {
            Some(&self.b)
        } else if self.b == id {
            Some(&self.a)
        } else {
            None
        }
    }
}

/// Multiple cultures coexisting. Resource competition drives arc consistency.
///
/// The consortium is the central data structure: cultures compete for shared resources
/// (constraint satisfaction), and mutualistic AC-3 helps neighbors reduce their domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consortium {
    pub cultures: Vec<Culture>,
    pub interactions: Vec<Interaction>,
    /// Temperature controls metabolism rate — higher = faster domain reduction
    pub temperature: f64,
    /// Track iteration count for convergence analysis
    #[serde(default)]
    pub iteration: usize,
}

impl Consortium {
    /// Create a new consortium with the given cultures, interactions, and temperature.
    pub fn new(
        cultures: Vec<Culture>,
        interactions: Vec<Interaction>,
        temperature: f64,
    ) -> Self {
        Self {
            cultures,
            interactions,
            temperature,
            iteration: 0,
        }
    }

    /// Get a culture by id.
    pub fn get_culture(&self, id: &str) -> Option<&Culture> {
        self.cultures.iter().find(|c| c.id == id)
    }

    /// Get a mutable reference to a culture by id.
    pub fn get_culture_mut(&mut self, id: &str) -> Option<&mut Culture> {
        self.cultures.iter_mut().find(|c| c.id == id)
    }

    /// Total population across all cultures.
    pub fn total_population(&self) -> f64 {
        self.cultures.iter().map(|c| c.population).sum()
    }

    /// Average fitness across all living cultures.
    pub fn average_fitness(&self) -> f64 {
        let alive: Vec<_> = self.cultures.iter().filter(|c| c.is_alive()).collect();
        if alive.is_empty() {
            return 0.0;
        }
        alive.iter().map(|c| c.fitness).sum::<f64>() / alive.len() as f64
    }

    /// Whether all cultures have converged (domains reduced to 1 or 0).
    pub fn is_converged(&self) -> bool {
        self.cultures.iter().all(|c| c.domain.len() <= 1)
    }

    /// Whether any culture has died (empty domain = unsatisfiable detected).
    pub fn has_dead(&self) -> bool {
        self.cultures.iter().any(|c| !c.is_alive())
    }

    /// Total remaining domain size across all cultures.
    pub fn total_domain_size(&self) -> usize {
        self.cultures.iter().map(|c| c.domain_size()).sum()
    }

    /// Perform one round of mutualistic AC-3 fermentation.
    ///
    /// This is the core algorithm: cultures help their neighbors reduce domains.
    /// Returns the number of values pruned in this round.
    pub fn ferment_step<F>(&mut self, constraints: &[(String, String, F)]) -> usize
    where
        F: Fn(i32, i32) -> bool,
    {
        let mut total_pruned = 0;
        // Build a map of constraints by (a, b) pairs
        let mut constraint_map: HashMap<(String, String), Vec<&(String, String, F)>> = HashMap::new();
        for c in constraints {
            constraint_map
                .entry((c.0.clone(), c.1.clone()))
                .or_default()
                .push(c);
            constraint_map
                .entry((c.1.clone(), c.0.clone()))
                .or_default()
                .push(c);
        }

        // AC-3 style: for each interaction, try to prune both sides
        // We collect prunes to avoid borrow issues
        let mut prunes: Vec<(String, Vec<i32>)> = vec![];

        for interaction in &self.interactions {
            let a_id = &interaction.a;
            let b_id = &interaction.b;

            // Find constraints between these cultures
            let key = (a_id.clone(), b_id.clone());
            let rev_key = (b_id.clone(), a_id.clone());

            // Prune A's domain against B
            if let (Some(a), Some(b)) = (
                self.get_culture(a_id),
                self.get_culture(b_id),
            ) {
                let b_domain = b.domain.clone();
                for entry in constraint_map.get(&key).into_iter().flatten() {
                    let mut a_domain = a.domain.clone();
                    a_domain.retain(|&v| b_domain.iter().any(|&n| (entry.2)(v, n)));
                    prunes.push((a_id.clone(), a_domain));
                }
            }

            // Prune B's domain against A (mutualistic!)
            if let (Some(a), Some(b)) = (
                self.get_culture(a_id),
                self.get_culture(b_id),
            ) {
                let a_domain = a.domain.clone();
                for entry in constraint_map.get(&rev_key).into_iter().flatten() {
                    let mut b_domain = b.domain.clone();
                    b_domain.retain(|&v| a_domain.iter().any(|&n| (entry.2)(n, v)));
                    prunes.push((b_id.clone(), b_domain));
                }
            }
        }

        // Apply prunes
        for (id, new_domain) in prunes {
            if let Some(culture) = self.get_culture_mut(&id) {
                if new_domain.len() < culture.domain.len() {
                    total_pruned += culture.domain.len() - new_domain.len();
                    culture.domain = new_domain;
                    culture.population = Culture::population_from_domain(&culture.domain);
                }
            }
        }

        // Apply metabolism: temperature-dependent resource consumption
        for culture in &mut self.cultures {
            let meta = MetabolismProfile::for_temperature(self.temperature);
            culture.consume(meta.rate * self.temperature, meta.decay);
        }

        // Update fitness for each culture
        // First collect partner info to avoid borrow issues
        let fitness_updates: Vec<(String, usize, usize)> = self.cultures.iter().map(|culture| {
            let interactions: Vec<_> = self
                .interactions
                .iter()
                .filter(|i| i.involves(&culture.id))
                .collect();
            let mut satisfied = 0;
            for inter in &interactions {
                if let Some(partner_id) = inter.partner(&culture.id) {
                    if let Some(partner) = self.get_culture(partner_id) {
                        if culture.domain.len() == 1 && partner.domain.len() == 1 {
                            satisfied += 1;
                        } else if culture.domain.len() > 0 && partner.domain.len() > 0 {
                            satisfied += 1;
                        }
                    }
                }
            }
            (culture.id.clone(), satisfied, interactions.len())
        }).collect();

        for (id, satisfied, total) in fitness_updates {
            if let Some(culture) = self.get_culture_mut(&id) {
                culture.update_fitness(satisfied, total);
            }
        }

        self.iteration += 1;
        total_pruned
    }

    /// Run fermentation until convergence or max iterations.
    /// Returns the number of iterations performed.
    pub fn ferment_until_converged<F>(
        &mut self,
        constraints: &[(String, String, F)],
        max_iterations: usize,
    ) -> usize
    where
        F: Fn(i32, i32) -> bool,
    {
        for _ in 0..max_iterations {
            if self.is_converged() || self.has_dead() {
                return self.iteration;
            }
            let pruned = self.ferment_step(constraints);
            if pruned == 0 && self.iteration > 1 {
                // No progress — converged or stuck
                return self.iteration;
            }
        }
        self.iteration
    }
}

/// Builder for consortium with fluent API.
#[derive(Debug, Clone)]
pub struct ConsortiumBuilder {
    cultures: Vec<Culture>,
    interactions: Vec<Interaction>,
    temperature: f64,
}

impl ConsortiumBuilder {
    pub fn new() -> Self {
        Self {
            cultures: vec![],
            interactions: vec![],
            temperature: 1.0,
        }
    }

    pub fn culture(mut self, culture: Culture) -> Self {
        self.cultures.push(culture);
        self
    }

    pub fn cultures(mut self, cultures: Vec<Culture>) -> Self {
        self.cultures = cultures;
        self
    }

    pub fn interaction(mut self, a: impl Into<String>, b: impl Into<String>, strength: f64) -> Self {
        self.interactions.push(Interaction::new(a, b, strength));
        self
    }

    pub fn interactions(mut self, interactions: Vec<Interaction>) -> Self {
        self.interactions = interactions;
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn build(self) -> Consortium {
        Consortium::new(self.cultures, self.interactions, self.temperature)
    }
}

impl Default for ConsortiumBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_cultures_compete_shared_resource() {
        let a = Culture::new("a", vec![1, 2, 3]);
        let b = Culture::new("b", vec![2, 3, 4]);
        let mut consortium = ConsortiumBuilder::new()
            .cultures(vec![a, b])
            .interaction("a", "b", 1.0)
            .temperature(1.0)
            .build();

        // Constraint: a != b (they compete, can't share the same value)
        let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
            ("a".into(), "b".into(), Box::new(|a, b| a != b)),
        ];

        let iters = consortium.ferment_until_converged(&constraints, 100);
        // Both should still have multiple options (not fully constrained by != alone)
        assert!(consortium.total_domain_size() > 0);
        assert!(iters < 100);
    }

    #[test]
    fn consortium_converges_for_trivial_csp() {
        // Two variables, each domain [1,2], constraint: a < b
        let a = Culture::new("a", vec![1, 2]);
        let b = Culture::new("b", vec![1, 2]);
        let mut consortium = ConsortiumBuilder::new()
            .cultures(vec![a, b])
            .interaction("a", "b", 1.0)
            .temperature(1.0)
            .build();

        let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
            ("a".into(), "b".into(), Box::new(|a, b| a < b)),
        ];

        consortium.ferment_until_converged(&constraints, 100);
        // a should be reduced to [1], b to [2]
        let ca = consortium.get_culture("a").unwrap();
        let cb = consortium.get_culture("b").unwrap();
        assert_eq!(ca.domain, vec![1]);
        assert_eq!(cb.domain, vec![2]);
    }

    #[test]
    fn population_bounded_throughout() {
        let a = Culture::new("a", vec![1, 2, 3]);
        let b = Culture::new("b", vec![1, 2, 3]);
        let mut consortium = ConsortiumBuilder::new()
            .cultures(vec![a, b])
            .interaction("a", "b", 0.5)
            .temperature(1.0)
            .build();

        let initial_pop = consortium.total_population();
        let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
            ("a".into(), "b".into(), Box::new(|a, b| a != b)),
        ];

        for _ in 0..50 {
            consortium.ferment_step(&constraints);
            // Population should never exceed initial (monotone non-increasing after domain reduction)
            assert!(consortium.total_population() <= initial_pop + 1e-10);
        }
    }

    #[test]
    fn is_converged_when_all_singletons() {
        let a = Culture::new("a", vec![1]);
        let b = Culture::new("b", vec![2]);
        let consortium = ConsortiumBuilder::new()
            .cultures(vec![a, b])
            .temperature(1.0)
            .build();
        assert!(consortium.is_converged());
    }

    #[test]
    fn not_converged_with_multi_value_domains() {
        let a = Culture::new("a", vec![1, 2]);
        let consortium = ConsortiumBuilder::new()
            .cultures(vec![a])
            .temperature(1.0)
            .build();
        assert!(!consortium.is_converged());
    }

    #[test]
    fn has_dead_detects_empty_domain() {
        let a = Culture::new("a", vec![]);
        let consortium = ConsortiumBuilder::new()
            .cultures(vec![a])
            .temperature(1.0)
            .build();
        assert!(consortium.has_dead());
    }

    #[test]
    fn graph_coloring_3_nodes_2_colors() {
        // Triangle graph, 2 colors — should be unsatisfiable
        let colors = vec![0, 1];
        let a = Culture::new("a", colors.clone());
        let b = Culture::new("b", colors.clone());
        let c = Culture::new("c", colors.clone());

        let mut consortium = ConsortiumBuilder::new()
            .cultures(vec![a, b, c])
            .interaction("a", "b", 1.0)
            .interaction("b", "c", 1.0)
            .interaction("a", "c", 1.0)
            .temperature(1.0)
            .build();

        let ne = |a: i32, b: i32| a != b;
        let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
            ("a".into(), "b".into(), Box::new(ne)),
            ("b".into(), "c".into(), Box::new(ne)),
            ("a".into(), "c".into(), Box::new(ne)),
        ];

        consortium.ferment_until_converged(&constraints, 100);
        // For 2-coloring a triangle, AC-3 won't necessarily detect unsatisfiability
        // but domains should be reduced
        assert!(consortium.total_domain_size() > 0);
    }

    #[test]
    fn graph_coloring_line_3_nodes_2_colors() {
        // Line graph a-b-c, 2 colors — satisfiable
        let colors = vec![0, 1];
        let a = Culture::new("a", colors.clone());
        let b = Culture::new("b", colors.clone());
        let c = Culture::new("c", colors.clone());

        let mut consortium = ConsortiumBuilder::new()
            .cultures(vec![a, b, c])
            .interaction("a", "b", 1.0)
            .interaction("b", "c", 1.0)
            .temperature(1.0)
            .build();

        let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
            ("a".into(), "b".into(), Box::new(|a, b| a != b)),
            ("b".into(), "c".into(), Box::new(|a, b| a != b)),
        ];

        consortium.ferment_until_converged(&constraints, 100);
        // Should converge with valid assignments
        // AC-3 reduces but may not give singletons for all — that's expected
        assert!(!consortium.has_dead());
    }
}
