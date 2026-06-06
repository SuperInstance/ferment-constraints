# ferment-constraints

**Constraint satisfaction as fermentation.**

Sourdough starters and server fleets solve the same class of problem. In a bakery, cultures of yeast and bacteria compete for sugars, cooperate to acidify dough, and converge on a stable ecosystem. In a data center, constraint variables compete for values, cooperate through arc consistency, and converge on a valid assignment.

`ferment-constraints` models this isomorphism explicitly. Each constraint variable is a **culture**. Cultures coexist in a **consortium**, where resource competition drives arc consistency. The AC-3 algorithm becomes **mutualistic fermentation**: cultures help their neighbors reduce domains, and the consortium converges on a solution. Then you **bake** the result.

```
CSP → Starter → Consortium → Ferment → Bake → Solution
Recipe   Culture    Ecosystem   Ferment   Oven    Bread
```

## The Sourdough-Fleet Isomorphism

| Sourdough | Server Fleet |
|---|---|
| Starter culture | CSP seed |
| Flour & water | Variables & domains |
| Temperature | Search parameter |
| Fermentation time | Iteration count |
| Wild yeast strains | Constraint cultures |
| Resource competition | Domain pruning |
| Mutualistic co-metabolism | Arc consistency propagation |
| Converged ecosystem | Arc-consistent CSP |
| Baking | Solution extraction |
| Flat loaf | Unsatisfiable |
| Beautiful rise | Solved ✓ |

## Quick Start

```rust
use ferment_constraints::prelude::*;

// Define a CSP: graph coloring with 3 nodes, 3 colors, triangle graph
let csp = CSP::inequality_csp(
    vec!["a".into(), "b".into(), "c".into()],
    vec![vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
    vec![
        ("a".into(), "b".into()),
        ("b".into(), "c".into()),
        ("a".into(), "c".into()),
    ],
);

// Solve: seed → ferment → bake in one call
let result = Baker::solve(&csp, 1.0, 200);

if result.solved {
    println!("Solution found in {} iterations:", result.iterations);
    for (var, val) in &result.assignments {
        println!("  {} = {}", var, val);
    }
}
```

## Step by Step

### 1. Seed a Consortium

```rust
let csp = CSP::inequality_csp(
    vec!["x".into(), "y".into()],
    vec![vec![1, 2, 3], vec![1, 2, 3]],
    vec![("x".into(), "y".into())],
);

let mut consortium = Starter::seed(&csp, 1.0);
// Temperature 1.0 = moderate metabolism
```

### 2. Ferment

```rust
let constraints: Vec<(String, String, Box<dyn Fn(i32, i32) -> bool>)> = vec![
    ("x".into(), "y".into(), Box::new(|a, b| a != b)),
];

let iters = consortium.ferment_until_converged(&constraints, 100);
println!("Converged in {} iterations", iters);
```

### 3. Prove Convergence

```rust
let initial = Starter::seed(&csp, 1.0);
// ... ferment ...
let proof = ConvergenceProof::prove(&initial, &consortium);
println!("{}", proof.summary());
// ConvergenceProof: converged=true, iterations=2, n=2, d=3, ...
```

### 4. Bake

```rust
let result = Baker::bake(&consortium, &csp);
if result.solved {
    println!("x = {:?}", result.get("x"));
    println!("y = {:?}", result.get("y"));
}
```

## Core Concepts

### Cultures

A `Culture` represents a constraint variable with its current domain. Population scales with `ln(|domain|)`, and fitness measures constraint satisfaction quality. As fermentation progresses, domains shrink and populations decrease.

```rust
let mut culture = Culture::new("x", vec![1, 2, 3, 4, 5]);
assert!(culture.is_alive());

// Prune values unsupported by neighbor
let removed = culture.prune_domain(&vec![1, 3, 5], &|v, n| v == n);
assert_eq!(removed, 2); // removed 2 and 4
assert_eq!(culture.domain, vec![1, 3, 5]);
```

### Consortium

A `Consortium` is a population of cultures connected by interactions. Each interaction models a binary constraint as resource overlap. Mutualistic AC-3 fermentation runs in rounds:

1. For each interaction, both cultures help each other prune unsupported domain values
2. Metabolism consumes resources (temperature-dependent)
3. Fitness updates reflect constraint satisfaction

```rust
let consortium = ConsortiumBuilder::new()
    .culture(Culture::new("a", vec![1, 2, 3]))
    .culture(Culture::new("b", vec![1, 2, 3]))
    .interaction("a", "b", 0.8)
    .temperature(1.5)
    .build();
```

### Metabolism

Cultures have metabolic profiles controlling domain reduction speed:

- **Exponential** (fast): Aggressive pruning, works best at high temperature
- **Steady** (slow): Conservative, more stable convergence  
- **Adaptive**: Temperature-responsive, switches strategies

Temperature follows an Arrhenius-like model: effective rate peaks at the profile's optimum temperature.

```rust
let fast = MetabolismProfile::fast();   // rate=0.3, optimum=1.5
let slow = MetabolismProfile::slow();   // rate=0.05, optimum=0.5
let adaptive = MetabolismProfile::for_temperature(1.0); // picks adaptive
```

### Convergence Proof

The library provides formal guarantees matching AC-3:

- **Population bounded**: Total population is monotone non-increasing
- **Domain reduction monotone**: Domains only shrink
- **Complexity**: O(n²d²) where n = variables, d = max domain size

This matches the known bound for AC-3 arc consistency.

## Mathematical Foundation

### Constraint Satisfaction as Fermentation

Given a binary CSP with variables X = {x₁, ..., xₙ}, domains D = {D₁, ..., Dₙ}, and constraints C:

**Culture mapping**: Each variable xᵢ → culture cᵢ with:
- Population: P(cᵢ) = ln(|Dᵢ|)
- Domain: Dom(cᵢ) = Dᵢ ⊆ ℤ
- Resources: One entry per constraint involving xᵢ

**Mutualistic AC-3**: For each constraint (xᵢ, xⱼ, R):

```
Dᵢ ← {v ∈ Dᵢ | ∃w ∈ Dⱼ : R(v, w)}    // culture i helps culture j
Dⱼ ← {w ∈ Dⱼ | ∃v ∈ Dᵢ : R(v, w)}    // culture j helps culture i
```

Both cultures prune simultaneously — mutualism, not just parasitism.

### Convergence Theorem

**Theorem**: Mutualistic fermentation converges in O(n²d²) steps.

*Proof sketch*:
1. Each domain value can be removed at most once (monotonicity)
2. Total values: Σ|Dᵢ| ≤ nd
3. Each iteration removes ≥ 1 value or terminates
4. Each iteration examines O(n) interactions, each O(d²) work
5. Total: O(n²d²) ∎

### Temperature and Metabolism

Effective metabolic rate follows Arrhenius kinetics:

```
r_eff(T) = r₀ · exp(-λ · (T - T_opt)²)
```

Where r₀ is base rate, λ is decay constant, and T_opt is the profile's optimum temperature. Higher temperatures activate exponential metabolism, lower temperatures favor steady reduction.

## Serde Support

All public types implement `Serialize` and `Deserialize`:

```rust
let culture = Culture::new("x", vec![1, 2, 3]);
let json = serde_json::to_string(&culture).unwrap();
let back: Culture = serde_json::from_str(&json).unwrap();
```

## API Overview

| Module | Key Types | Purpose |
|---|---|---|
| `culture` | `Culture`, `CultureBuilder` | Single constraint variable |
| `consortium` | `Consortium`, `ConsortiumBuilder`, `Interaction` | Population of cultures |
| `metabolism` | `MetabolismProfile`, `MetabolismKind` | Domain reduction strategy |
| `starter` | `CSP`, `Starter` | CSP specification & seeding |
| `proof` | `ConvergenceProof`, `ConvergenceBound` | Formal convergence guarantees |
| `bake` | `BakeResult`, `Baker` | Solution extraction |

## Installation

```toml
[dependencies]
ferment-constraints = "0.1"
```

## License

MIT
