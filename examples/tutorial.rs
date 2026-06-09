//! # ferment-constraints Tutorial
//!
//! Learn how constraint satisfaction problems become sourdough fermentation.
//!
//! **Mathematical insight:** CSP domains are "cultures" in a consortium.
//! Mutualistic AC-3 fermentation prunes domains just like wild yeasts
//! consume shared sugars. Domain reduction is monotone, population is bounded,
//! and convergence follows O(n²d²) — matching classic AC-3 complexity.
//!
//! The metaphor: each CSP variable is a sourdough culture. Constraints are
//! resource overlaps. Fermentation (arc consistency) drives domains to
//! their irreducible core. When done, you "bake" the result into a solution.
//!
//! Run: `cargo run --example tutorial`

use ferment_constraints::{
    CSP, Starter, Baker,
    Culture, CultureBuilder,
    ConsortiumBuilder,
    MetabolismProfile,
    ConvergenceProof,
};

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!("  ferment-constraints: CSP as Sourdough Fermentation    ");
    println!("════════════════════════════════════════════════════════\n");

    lesson_1_culture_basics();
    lesson_2_consortium_fermentation();
    lesson_3_metabolism_profiles();
    lesson_4_csp_graph_coloring();
    lesson_5_convergence_proof();
    lesson_6_full_bake_pipeline();
    lesson_7_temperature_effects();
}

/// Lesson 1: Culture Basics — A single culture is a CSP variable domain.
///
/// Each culture has:
/// - A domain (set of feasible values)
/// - Population = ln(|domain|) — vitality proxy
/// - Fitness = fraction of satisfied constraints
///
/// When you prune a domain, population drops. Empty domain = dead culture
/// = unsatisfiable variable.
fn lesson_1_culture_basics() {
    println!("━━━ Lesson 1: Culture = Variable Domain ━━━\n");

    // Create cultures (CSP variables)
    let mut x = Culture::new("x", vec![1, 2, 3, 4, 5]);
    let y = Culture::new("y", vec![2, 3, 4]);

    println!("  Culture x: domain={:?}, population={:.3}", x.domain, x.population);
    println!("  Culture y: domain={:?}, population={:.3}", y.domain, y.population);

    // Prune: remove values from x that can't satisfy x != y with any y-value
    let constraint = |v: i32, n: i32| v != n;
    let removed = x.prune_domain(&y.domain, &constraint);
    println!("\n  After pruning x against y (constraint: ≠):");
    println!("    removed {} values, domain={:?}, population={:.3}", removed, x.domain, x.population);

    // Builder pattern
    let custom = CultureBuilder::new("z")
        .domain(vec![10, 20, 30])
        .fitness(0.8)
        .resources(vec![0.5, 0.3])
        .build();
    println!("\n  Built culture z: domain={:?}, fitness={}", custom.domain, custom.fitness);

    // Dead culture
    let dead = Culture::new("dead", vec![]);
    println!("\n  Dead culture: alive={}, population={}", dead.is_alive(), dead.population);
    println!();
}

/// Lesson 2: Consortium Fermentation — Multiple cultures cooperate.
///
/// A consortium is the fermentation vat: cultures interact through constraints.
/// The `ferment_step` method performs one round of mutualistic AC-3:
///   - For each constraint pair, cultures help neighbors prune domains
///   - Population decreases as domains shrink
///   - Convergence: all domains are singletons (or a culture dies)
fn lesson_2_consortium_fermentation() {
    println!("━━━ Lesson 2: Consortium = Fermentation Vat ━━━\n");

    // Two variables, domains [1,2], constraint: a < b
    let a = Culture::new("a", vec![1, 2]);
    let b = Culture::new("b", vec![1, 2]);

    let mut consortium = ConsortiumBuilder::new()
        .cultures(vec![a, b])
        .interaction("a", "b", 1.0)
        .temperature(1.0)
        .build();

    println!("  Initial: a.domain={:?}, b.domain={:?}",
             consortium.get_culture("a").unwrap().domain,
             consortium.get_culture("b").unwrap().domain);

    // Constraints: a < b
    let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
        ("a".into(), "b".into(), Box::new(|a, b| a < b)),
    ];

    // Step-by-step fermentation
    println!("\n  Fermenting...");
    for i in 0..10 {
        let pruned = consortium.ferment_step(&constraints);
        let ca = consortium.get_culture("a").unwrap();
        let cb = consortium.get_culture("b").unwrap();
        println!("    iter {}: pruned={} | a={:?}, b={:?} | fitness={:.2}",
                 i + 1, pruned, ca.domain, cb.domain, consortium.average_fitness());
        if pruned == 0 { break; }
    }

    println!("\n  Converged: {}, Dead: {}, Total domain size: {}",
             consortium.is_converged(),
             consortium.has_dead(),
             consortium.total_domain_size());

    let ca = consortium.get_culture("a").unwrap();
    let cb = consortium.get_culture("b").unwrap();
    println!("  Solution: a={}, b={} (a<b ✓)\n", ca.domain[0], cb.domain[0]);
}

/// Lesson 3: Metabolism Profiles — Temperature controls fermentation speed.
///
/// Just like real sourdough, fermentation speed depends on temperature:
/// - Hot → exponential (aggressive domain reduction)
/// - Cold → steady (cautious, less likely to over-prune)
/// - Medium → adaptive (switches between strategies)
///
/// The effective rate follows an Arrhenius-like curve:
///   rate_eff = rate × exp(-decay × (T - T_opt)²)
fn lesson_3_metabolism_profiles() {
    println!("━━━ Lesson 3: Metabolism = Temperature Control ━━━\n");

    let profiles = [
        ("Fast (hot)", MetabolismProfile::fast()),
        ("Slow (cold)", MetabolismProfile::slow()),
        ("Adaptive (medium)", MetabolismProfile::adaptive()),
    ];

    for (name, profile) in &profiles {
        println!("  {}: rate={:.2}, decay={:.2}, opt_temp={:.1}, kind={:?}",
                 name, profile.rate, profile.decay, profile.optimum_temp, profile.kind);
    }

    // Show how temperature selects metabolism
    println!("\n  Auto-selected metabolism by temperature:");
    for temp in [0.2, 0.8, 1.5] {
        let m = MetabolismProfile::for_temperature(temp);
        println!("    T={:.1} → {:?}", temp, m.kind);
    }

    // Domain reduction factor at different temperatures
    println!("\n  Reduction factor (domain size=10) by temperature:");
    for temp in [0.3, 0.8, 1.5] {
        let m = MetabolismProfile::for_temperature(temp);
        let factor = m.domain_reduction_factor(temp, 10);
        println!("    T={:.1} ({:?}): factor={:.3}", temp, m.kind, factor);
    }
    println!();
}

/// Lesson 4: CSP to Fermentation — Graph coloring via Starter.
///
/// The `CSP` type defines a constraint satisfaction problem.
/// The `Starter` transforms it into a living consortium.
/// The metaphor: CSP is the recipe, Starter is the sourdough starter,
/// Consortium is the living dough, Fermentation is the proofing.
fn lesson_4_csp_graph_coloring() {
    println!("━━━ Lesson 4: Graph Coloring via CSP → Starter ━━━\n");

    // Graph coloring: 3 colors, line graph A-B-C
    let colors = vec![1, 2, 3];
    let csp = CSP::inequality_csp(
        vec!["A".into(), "B".into(), "C".into()],
        vec![colors.clone(), colors.clone(), colors.clone()],
        vec![("A".into(), "B".into()), ("B".into(), "C".into())],
    );

    println!("  CSP: {} variables, {} constraints, {} colors",
             csp.num_variables(), csp.num_constraints(), colors.len());

    // Seed the consortium
    let consortium = Starter::seed(&csp, 1.0);
    println!("  Seeded consortium: {} cultures, {} interactions",
             consortium.cultures.len(), consortium.interactions.len());
    println!("  Initial domain sizes: {:?}",
             consortium.cultures.iter().map(|c| c.domain_size()).collect::<Vec<_>>());

    // Ferment with inequality constraints
    let mut fermented = consortium.clone();
    let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
        ("A".into(), "B".into(), Box::new(|a, b| a != b)),
        ("B".into(), "C".into(), Box::new(|a, b| a != b)),
    ];

    let iters = fermented.ferment_until_converged(&constraints, 100);
    println!("\n  Fermented in {} iterations", iters);
    for c in &fermented.cultures {
        println!("    {}: domain={:?} (alive={})", c.id, c.domain, c.is_alive());
    }

    // Triangle graph with 2 colors — unsatisfiable!
    let triangle_csp = CSP::inequality_csp(
        vec!["X".into(), "Y".into(), "Z".into()],
        vec![vec![0, 1], vec![0, 1], vec![0, 1]],
        vec![("X".into(), "Y".into()), ("Y".into(), "Z".into()), ("X".into(), "Z".into())],
    );
    let unsat_consortium = Starter::seed(&triangle_csp, 1.0);
    println!("\n  Triangle graph, 2 colors (unsatisfiable?):");
    println!("    {} cultures, {} interactions",
             unsat_consortium.cultures.len(), unsat_consortium.interactions.len());
    println!();
}

/// Lesson 5: Convergence Proof — Fermentation is guaranteed to converge.
///
/// The ConvergenceProof module verifies:
/// - Domain reduction is monotone (never increases)
/// - Population is bounded (≤ initial)
/// - Convergence in O(n²d²) matching standard AC-3
///
/// Two bound classes: AC3Equivalent (normal) or Accelerated (faster than theory).
fn lesson_5_convergence_proof() {
    println!("━━━ Lesson 5: Convergence Proof ━━━\n");

    // Create and solve a simple CSP
    let csp = CSP::new(
        vec!["a".into(), "b".into(), "c".into()],
        vec![vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
        vec![
            ("a".into(), "b".into(), 0),
            ("b".into(), "c".into(), 0),
        ],
        vec!["a != b".into()],
    );

    let initial = Starter::seed(&csp, 1.0);
    let mut final_c = initial.clone();

    let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
        ("a".into(), "b".into(), Box::new(|a, b| a != b)),
        ("b".into(), "c".into(), Box::new(|a, b| a != b)),
    ];
    final_c.ferment_until_converged(&constraints, 100);

    let proof = ConvergenceProof::prove(&initial, &final_c);
    println!("  {}", proof.summary());
    println!("\n  Self-consistent: {}", proof.verify());
    println!("  Population bounded: {}", proof.population_bounded);
    println!("  Monotone reduction: {}", proof.monotone_reduction);
    println!("  Theoretical bound: O(n²d²) = O({}²×{}²) = {}",
             proof.n, proof.d, proof.theoretical_bound);
    println!("  Actual iterations: {}", proof.iterations);
    println!();
}

/// Lesson 6: Full Pipeline — CSP → Seed → Ferment → Bake.
///
/// The Baker extracts the final solution after fermentation.
/// It checks constraint satisfaction and reports the result.
/// This is the "one-call-does-everything" entry point.
fn lesson_6_full_bake_pipeline() {
    println!("━━━ Lesson 6: Full Bake Pipeline (CSP → Solution) ━━━\n");

    // Example: ordering constraint — a < b < c
    let csp = CSP::new(
        vec!["a".into(), "b".into(), "c".into()],
        vec![vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
        vec![
            ("a".into(), "b".into(), 0),
            ("b".into(), "c".into(), 1),
        ],
        vec!["a < b".into(), "b < c".into()],
    );

    let result = Baker::solve(&csp, 1.0, 200);

    println!("  Solved: {}", result.solved);
    println!("  Iterations: {}", result.iterations);
    println!("  Assignments:");
    for (var, val) in &result.assignments {
        println!("    {} = {}", var, val);
    }

    if let Some(ref proof) = result.proof {
        println!("\n  Convergence: {:?}", proof.bound);
        println!("  Total domain reduction: {}", proof.total_reduction);
    }

    // N-Queens example
    println!("\n  --- N-Queens (4×4) ---");
    let queens_csp = CSP::n_queens(4);
    println!("  Variables: {}, Constraints: {}",
             queens_csp.num_variables(), queens_csp.num_constraints());

    let queens_result = Baker::solve(&queens_csp, 1.0, 200);
    println!("  Solved: {}", queens_result.solved);
    if queens_result.solved {
        println!("  Queen positions:");
        for var in &queens_csp.variables {
            if let Some(val) = queens_result.get(var) {
                println!("    {} → row {}", var, val);
            }
        }
    }

    // Unsatisfiable case
    println!("\n  --- Unsatisfiable CSP ---");
    let unsat = CSP::inequality_csp(
        vec!["x".into(), "y".into()],
        vec![vec![1], vec![1]],
        vec![("x".into(), "y".into())],
    );
    let unsat_result = Baker::solve(&unsat, 1.0, 100);
    println!("  Solved: {} (unsatisfiable: {})",
             unsat_result.solved, unsat_result.is_unsatisfiable());
    println!("  Unsatisfied constraints: {:?}", unsat_result.unsatisfied);
    println!();
}

/// Lesson 7: Temperature Effects — Hot vs Cold Fermentation.
///
/// Temperature controls fermentation speed:
/// - High temperature → faster convergence but may over-prune
/// - Low temperature → slower but more conservative
/// - The Arrhenius curve means there's an optimal temperature
fn lesson_7_temperature_effects() {
    println!("━━━ Lesson 7: Temperature Effects on Convergence ━━━\n");

    // Same problem, different temperatures
    let csp = CSP::inequality_csp(
        vec!["a".into(), "b".into(), "c".into(), "d".into()],
        vec![vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
        vec![
            ("a".into(), "b".into()),
            ("b".into(), "c".into()),
            ("c".into(), "d".into()),
            ("a".into(), "c".into()),
        ],
    );

    let temperatures = [0.2, 0.5, 1.0, 1.5, 2.5];
    println!("  Temperature | Iterations | Converged | Bound");
    println!("  ------------|------------|-----------|----------");

    for temp in temperatures {
        let result = Baker::solve(&csp, temp, 200);
        let converged = result.proof.as_ref().map(|p| p.converged).unwrap_or(false);
        let bound = result.proof.as_ref().map(|p| format!("{:?}", p.bound)).unwrap_or("N/A".into());
        println!("  {:>11.1} | {:>10} | {:>9} | {}",
                 temp, result.iterations, converged, bound);
    }

    println!("\n  ✦ Key insight: Fermentation converges because domain reduction is");
    println!("    monotone — values are only ever removed, never added. Population");
    println!("    (vitality) is bounded. The proof is self-verifying.\n");
}
