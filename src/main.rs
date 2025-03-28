use std::path::PathBuf;

use benchmark::{
    self,
    function::FunctionBenchmark,
    v8::{NewIsolate, SameIsolateNewContext, SameIsolateSameContext, V8Benchmark},
    wasm2c::Wasm2CBenchmark,
};

use benchmark::Benchmark;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name = "Arca Benchmark")]
#[command(version, about)]
struct Args {
    /// How many threads to use at maximum (default: 8x the number of CPUs)
    #[arg(short, long)]
    parallel: Option<usize>,
    /// How many iterations to run per thread
    #[arg(short, long, default_value = "1s")]
    duration: humantime::Duration,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a single benchmark
    Run {
        /// Which benchmark to run
        benchmark: BenchmarkMode,
        /// The path to the module to run (WAT for V8 benchmarks)
        module: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Run all available benchmarks with the same settings
    RunAll {
        /// Output directory
        output: PathBuf,
        /// Module for V8 benchmarks (WAT file)
        v8_module: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum BenchmarkMode {
    /// Native Rust add function
    Function,
    /// V8 with one isolate per thread and one context per thread
    V8,
    /// V8 with one isolate per thread but one context per call
    V8ContextPerCall,
    /// V8 with one isolate per call
    V8IsolatePerCall,
    /// wasm2c with software bounds checking
    Wasm2cBoundsChecked,
    /// wasm2c with hardware bounds checking
    Wasm2cMmap,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let parallel = args.parallel.unwrap_or(0);
    let cpus: usize = std::thread::available_parallelism().unwrap().into();
    let parallel = if parallel == 0 { cpus } else { parallel };
    let duration: std::time::Duration = args.duration.into();

    match args.command {
        Commands::Run {
            benchmark,
            module,
            output,
        } => {
            let benchmark: &dyn Benchmark = unsafe {
                match benchmark {
                    BenchmarkMode::Function => &FunctionBenchmark::new(),
                    BenchmarkMode::V8 => &V8Benchmark::<SameIsolateSameContext>::new(module)?,
                    BenchmarkMode::V8ContextPerCall => {
                        &V8Benchmark::<SameIsolateNewContext>::new(module)?
                    }
                    BenchmarkMode::V8IsolatePerCall => &V8Benchmark::<NewIsolate>::new(module)?,
                    BenchmarkMode::Wasm2cBoundsChecked => &Wasm2CBenchmark::new(module, false)?,
                    BenchmarkMode::Wasm2cMmap => &Wasm2CBenchmark::new(module, true)?,
                }
            };

            let mut writer = output.map(csv::Writer::from_path).transpose()?;

            for datum in benchmark.collect_data(parallel, duration) {
                if let Some(ref mut writer) = writer {
                    writer.serialize(datum)?;
                }
            }
        }
        Commands::RunAll { output, v8_module } => {
            std::fs::create_dir_all(output.clone())?;
            let benchmarks: Vec<(&str, Box<dyn Benchmark>)> = unsafe {
                vec![
                    ("Rust function", Box::new(FunctionBenchmark::new())),
                    (
                        "V8 (shared context)",
                        Box::new(V8Benchmark::<SameIsolateSameContext>::new(
                            v8_module.clone(),
                        )?),
                    ),
                    (
                        "V8 (new context)",
                        Box::new(V8Benchmark::<SameIsolateNewContext>::new(
                            v8_module.clone(),
                        )?),
                    ),
                    (
                        "V8 (new isolate)",
                        Box::new(V8Benchmark::<NewIsolate>::new(v8_module.clone())?),
                    ),
                    (
                        "wasm2c (bounds checked)",
                        Box::new(Wasm2CBenchmark::new(v8_module.clone(), false)?),
                    ),
                    (
                        "wasm2c (mmap)",
                        Box::new(Wasm2CBenchmark::new(v8_module.clone(), true)?),
                    ),
                ]
            };

            for (label, benchmark) in benchmarks {
                let mut file = output.clone();
                file.push(label);
                file.set_extension("csv");
                let mut writer = csv::Writer::from_path(file)?;
                for datum in benchmark.collect_data(parallel, duration) {
                    writer.serialize(datum)?;
                }
            }
        }
    }
    Ok(())
}
