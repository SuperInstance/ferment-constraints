use serde::{Deserialize, Serialize};

/// Kind of metabolism profile for a culture.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MetabolismKind {
    /// Fast exponential domain reduction — aggressive pruning
    Exponential,
    /// Slow steady reduction — cautious, more stable
    Steady,
    /// Temperature-adaptive — switches between fast and slow
    Adaptive,
}

/// Culture metabolism profile: controls how fast domains shrink.
///
/// Fast (exponential) metabolism aggressively prunes domains.
/// Slow (steady) metabolism is more conservative but less likely to over-prune.
/// Temperature parameter modulates the rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolismProfile {
    /// Base consumption rate per step
    pub rate: f64,
    /// Exponential decay factor for resource consumption
    pub decay: f64,
    /// Optimum temperature for this metabolism
    pub optimum_temp: f64,
    /// Kind of metabolism
    pub kind: MetabolismKind,
}

impl MetabolismProfile {
    /// Create a fast (exponential) metabolism profile.
    pub fn fast() -> Self {
        Self {
            rate: 0.3,
            decay: 0.95,
            optimum_temp: 1.5,
            kind: MetabolismKind::Exponential,
        }
    }

    /// Create a slow (steady) metabolism profile.
    pub fn slow() -> Self {
        Self {
            rate: 0.05,
            decay: 0.99,
            optimum_temp: 0.5,
            kind: MetabolismKind::Steady,
        }
    }

    /// Create an adaptive metabolism profile.
    pub fn adaptive() -> Self {
        Self {
            rate: 0.15,
            decay: 0.97,
            optimum_temp: 1.0,
            kind: MetabolismKind::Adaptive,
        }
    }

    /// Select a metabolism profile appropriate for the given temperature.
    pub fn for_temperature(temperature: f64) -> Self {
        if temperature > 1.2 {
            Self::fast()
        } else if temperature < 0.5 {
            Self::slow()
        } else {
            Self::adaptive()
        }
    }

    /// Compute the effective rate at a given temperature.
    /// Follows an Arrhenius-like curve: rate * exp(-decay * (T - T_opt)^2)
    pub fn effective_rate(&self, temperature: f64) -> f64 {
        let diff = temperature - self.optimum_temp;
        self.rate * (-self.decay * diff * diff).exp()
    }

    /// Apply domain reduction based on metabolism kind.
    /// Returns the reduction factor (0..1) — multiply domain by this to get new size.
    pub fn domain_reduction_factor(&self, temperature: f64, current_domain_size: usize) -> f64 {
        let eff = self.effective_rate(temperature);
        match self.kind {
            MetabolismKind::Exponential => {
                // Aggressive: exponential decay
                (-eff * current_domain_size as f64 * 0.1).exp()
            }
            MetabolismKind::Steady => {
                // Linear decay, gentler
                (1.0 - eff * 0.1).max(0.0)
            }
            MetabolismKind::Adaptive => {
                // Uses exponential when hot, linear when cool
                if temperature > self.optimum_temp {
                    (-eff * current_domain_size as f64 * 0.05).exp()
                } else {
                    (1.0 - eff * 0.05).max(0.0)
                }
            }
        }
    }
}

impl Default for MetabolismProfile {
    fn default() -> Self {
        Self::adaptive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_has_higher_rate_than_slow() {
        let fast = MetabolismProfile::fast();
        let slow = MetabolismProfile::slow();
        assert!(fast.rate > slow.rate);
    }

    #[test]
    fn effective_rate_highest_at_optimum() {
        let profile = MetabolismProfile::fast();
        let at_opt = profile.effective_rate(profile.optimum_temp);
        let above = profile.effective_rate(profile.optimum_temp + 1.0);
        let below = profile.effective_rate(profile.optimum_temp - 1.0);
        assert!(at_opt > above);
        assert!(at_opt > below);
    }

    #[test]
    fn for_temperature_hot_gives_fast() {
        let profile = MetabolismProfile::for_temperature(2.0);
        assert_eq!(profile.kind, MetabolismKind::Exponential);
    }

    #[test]
    fn for_temperature_cold_gives_slow() {
        let profile = MetabolismProfile::for_temperature(0.1);
        assert_eq!(profile.kind, MetabolismKind::Steady);
    }

    #[test]
    fn for_temperature_medium_gives_adaptive() {
        let profile = MetabolismProfile::for_temperature(0.8);
        assert_eq!(profile.kind, MetabolismKind::Adaptive);
    }

    #[test]
    fn domain_reduction_factor_decreases_with_size() {
        let profile = MetabolismProfile::fast();
        let small = profile.domain_reduction_factor(1.5, 2);
        let large = profile.domain_reduction_factor(1.5, 20);
        assert!(large <= small + 1e-10); // larger domains reduce faster with exponential
    }

    #[test]
    fn temperature_affects_convergence_speed() {
        let fast = MetabolismProfile::fast();
        let slow = MetabolismProfile::slow();
        // At their respective optimums, fast should have higher effective rate
        assert!(fast.effective_rate(fast.optimum_temp) > slow.effective_rate(slow.optimum_temp));
    }
}
