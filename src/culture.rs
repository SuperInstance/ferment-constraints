use serde::{Deserialize, Serialize};

/// A constraint "culture" — each culture represents a constraint variable domain.
///
/// Cultures have population (domain size proxy), fitness (solution quality),
/// resources (constraint satisfaction pressure), and a domain of possible values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Culture {
    /// Unique identifier, maps to a CSP variable name
    pub id: String,
    /// Population — proportional to log(domain_size). Represents culture vitality.
    pub population: f64,
    /// Fitness — how well this culture satisfies its constraints (0..1)
    pub fitness: f64,
    /// Resources — one entry per constraint interaction. Higher = more pressure.
    pub resources: Vec<f64>,
    /// Domain — the remaining feasible values for this variable
    pub domain: Vec<i32>,
}

impl Culture {
    /// Create a new culture with the given id and domain.
    pub fn new(id: impl Into<String>, domain: Vec<i32>) -> Self {
        let pop = Self::population_from_domain(&domain);
        Self {
            id: id.into(),
            population: pop,
            fitness: 1.0,
            resources: vec![],
            domain,
        }
    }

    /// Compute initial population from domain size using log scaling.
    pub fn population_from_domain(domain: &[i32]) -> f64 {
        if domain.is_empty() {
            0.0
        } else {
            (domain.len() as f64).ln()
        }
    }

    /// Current domain size.
    pub fn domain_size(&self) -> usize {
        self.domain.len()
    }

    /// Whether this culture is still viable (non-empty domain).
    pub fn is_alive(&self) -> bool {
        !self.domain.is_empty()
    }

    /// Remove values from domain that don't satisfy a binary constraint.
    /// Returns the number of values removed.
    pub fn prune_domain<F: Fn(i32, i32) -> bool>(
        &mut self,
        neighbor_domain: &[i32],
        constraint: &F,
    ) -> usize {
        let before = self.domain.len();
        self.domain.retain(|&v| {
            neighbor_domain.iter().any(|&n| constraint(v, n))
        });
        let removed = before - self.domain.len();
        self.population = Self::population_from_domain(&self.domain);
        removed
    }

    /// Update fitness based on fraction of satisfied interactions.
    pub fn update_fitness(&mut self, satisfied: usize, total: usize) {
        self.fitness = if total == 0 {
            1.0
        } else {
            satisfied as f64 / total as f64
        };
    }

    /// Consume resources at a given rate, reducing population.
    pub fn consume(&mut self, rate: f64, decay: f64) {
        let total_resource: f64 = self.resources.iter().sum();
        let consumption = rate * total_resource * decay;
        self.population = (self.population - consumption).max(0.0);
    }

    /// Total resource pressure on this culture.
    pub fn total_resources(&self) -> f64 {
        self.resources.iter().sum()
    }
}

/// Builder for cultures with fluent API.
#[derive(Debug, Clone)]
pub struct CultureBuilder {
    id: String,
    domain: Vec<i32>,
    population: Option<f64>,
    fitness: Option<f64>,
    resources: Vec<f64>,
}

impl CultureBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            domain: vec![],
            population: None,
            fitness: None,
            resources: vec![],
        }
    }

    pub fn domain(mut self, domain: Vec<i32>) -> Self {
        self.domain = domain;
        self
    }

    pub fn population(mut self, population: f64) -> Self {
        self.population = Some(population);
        self
    }

    pub fn fitness(mut self, fitness: f64) -> Self {
        self.fitness = Some(fitness);
        self
    }

    pub fn resources(mut self, resources: Vec<f64>) -> Self {
        self.resources = resources;
        self
    }

    pub fn build(self) -> Culture {
        let mut culture = Culture::new(self.id, self.domain);
        if let Some(pop) = self.population {
            culture.population = pop;
        }
        if let Some(fit) = self.fitness {
            culture.fitness = fit;
        }
        culture.resources = self.resources;
        culture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_culture_maintains_population() {
        let c = Culture::new("x", vec![1, 2, 3]);
        assert!(c.population > 0.0);
        assert_eq!(c.domain_size(), 3);
        assert!(c.is_alive());
    }

    #[test]
    fn empty_domain_means_dead() {
        let c = Culture::new("x", vec![]);
        assert_eq!(c.population, 0.0);
        assert!(!c.is_alive());
        assert_eq!(c.domain_size(), 0);
    }

    #[test]
    fn population_scales_with_domain() {
        let c1 = Culture::new("a", vec![1]);
        let c2 = Culture::new("b", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(c2.population > c1.population);
    }

    #[test]
    fn prune_domain_removes_unsupported() {
        let mut c = Culture::new("x", vec![1, 2, 3, 4, 5]);
        let neighbor = vec![1, 3, 5];
        let removed = c.prune_domain(&neighbor, &|v, n| v == n);
        assert_eq!(removed, 2);
        assert_eq!(c.domain, vec![1, 3, 5]);
    }

    #[test]
    fn prune_domain_with_inequality_constraint() {
        let mut c = Culture::new("x", vec![1, 2, 3]);
        let neighbor = vec![2];
        let removed = c.prune_domain(&neighbor, &|v, n| v != n);
        assert_eq!(removed, 1);
        assert_eq!(c.domain, vec![1, 3]);
    }

    #[test]
    fn fitness_update() {
        let mut c = Culture::new("x", vec![1]);
        c.update_fitness(3, 5);
        assert!((c.fitness - 0.6).abs() < 1e-10);
    }

    #[test]
    fn consume_reduces_population() {
        let mut c = Culture::new("x", vec![1, 2, 3]);
        c.resources = vec![0.5, 0.5];
        let pop_before = c.population;
        c.consume(0.1, 1.0);
        assert!(c.population < pop_before);
    }

    #[test]
    fn consume_never_goes_negative() {
        let mut c = Culture::new("x", vec![1]);
        c.population = 0.01;
        c.resources = vec![10.0];
        c.consume(1.0, 1.0);
        assert!(c.population >= 0.0);
    }

    #[test]
    fn builder_produces_equivalent_culture() {
        let direct = Culture::new("x", vec![1, 2]);
        let built = CultureBuilder::new("x")
            .domain(vec![1, 2])
            .build();
        assert_eq!(direct.id, built.id);
        assert_eq!(direct.domain, built.domain);
        assert!((direct.population - built.population).abs() < 1e-10);
    }

    #[test]
    fn total_resources() {
        let mut c = Culture::new("x", vec![1]);
        c.resources = vec![1.0, 2.0, 3.0];
        assert!((c.total_resources() - 6.0).abs() < 1e-10);
    }
}
