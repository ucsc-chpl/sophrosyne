mod atomic_throughput;
mod occupancy_discovery;
mod vector_add;

// Re-export benchmark functions directly.
pub use atomic_throughput::atomic_throughput;
pub use occupancy_discovery::occupancy_discovery;
pub use vector_add::vector_add;
