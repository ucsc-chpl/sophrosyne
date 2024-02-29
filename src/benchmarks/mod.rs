mod vector_add;
mod occupancy_discovery;

// Re-export benchmark functions directly.
#[allow(unused_imports)]
pub use vector_add::vector_add;
pub use occupancy_discovery::occupancy_discovery;
