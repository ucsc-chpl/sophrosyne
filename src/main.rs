mod benchmarks;

fn main() {
    // Creates a logger, filtering out all log messages except those from this module.
    env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module(module_path!(), log::LevelFilter::Info)
        .filter_module("easywg", log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();

    // Needs to be run in an async environment.
    // Pollster provides a simple way to do this (should work on all platforms).
    pollster::block_on(benchmarks::vector_add());
}
