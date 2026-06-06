pub mod culture;
pub mod consortium;
pub mod metabolism;
pub mod starter;
pub mod proof;
pub mod bake;

pub use culture::{Culture, CultureBuilder};
pub use consortium::{Consortium, ConsortiumBuilder, Interaction};
pub use metabolism::{MetabolismProfile, MetabolismKind};
pub use starter::{CSP, Starter};
pub use proof::{ConvergenceProof, ConvergenceBound};
pub use bake::{BakeResult, Baker};

/// Re-export core types for convenience
pub mod prelude {
    pub use crate::culture::{Culture, CultureBuilder};
    pub use crate::consortium::{Consortium, ConsortiumBuilder, Interaction};
    pub use crate::metabolism::{MetabolismProfile, MetabolismKind};
    pub use crate::starter::{CSP, Starter};
    pub use crate::proof::{ConvergenceProof, ConvergenceBound};
    pub use crate::bake::{BakeResult, Baker};
}
