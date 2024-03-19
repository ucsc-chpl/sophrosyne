mod benchmarks;

use clap::Parser;
use std::process;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of benchmark to run.
    #[arg()]
    benchmark: Option<String>,

    /// Device to run benchmark on.
    #[arg(short, long)]
    device: Option<usize>,

    /// Enumerate all available devices.
    #[arg(short, long)]
    list_devices: bool,
}

fn main() {
    let args = Args::parse();

    if args.list_devices {
        let device_info = easywg::list_devices();
        for (i, info) in device_info.iter().enumerate() {
            println!("Adapter {}: {:#?}", i, info);
        }
        return;
    } else {
        // Creates a logger, filtering out all log messages except those from this module.
        env_logger::builder()
            .filter_level(log::LevelFilter::Off)
            .filter_module(module_path!(), log::LevelFilter::Info)
            .filter_module("easywg", log::LevelFilter::Info)
            .format_timestamp_nanos()
            .init();

        match args.device {
            Some(_) => {
                // TODO: Pass selected device to benchmark functions.
                match args.benchmark.as_deref() {
                    Some("occupancy_discovery") => {
                        pollster::block_on(benchmarks::occupancy_discovery());
                    }
                    Some("vector_add") => {
                        pollster::block_on(benchmarks::vector_add());
                    }
                    Some("atomic_throughput") => {
                        pollster::block_on(benchmarks::atomic_throughput());
                    }
                    _ => {
                        eprintln!("Please specify a benchmark.");
                        process::exit(1);
                    }
                }
            }
            None => {
                eprintln!("No device specified.");
                process::exit(1);
            }
        }
    }
}
