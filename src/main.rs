use std::path::PathBuf;

use benchmark::{
    self,
    arca::ArcaBenchmark,
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
    /// How many threads to use at maximum (default: the number of CPUs)
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
        /// Which approach to benchmark
        benchmark: BenchmarkMode,
        /// Which benchmark to run
        program: BenchmarkType,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Run all available benchmarks with the same settings
    RunAll {
        /// Which benchmark to run
        program: BenchmarkType,
        /// Output directory
        output: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Debug, Copy)]
enum BenchmarkType {
    /// Add with no memory
    Add,
    /// Add with 64KiB of memory
    AddMem,
    /// Add two 4096-element vectors
    AddVec,
    /// Multiply two 64x64 matrices
    MatMul64,
    /// Multiply two 128x128 matrices
    MatMul128,
}

#[derive(clap::ValueEnum, Clone, Debug, Copy)]
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
    /// Arca
    Arca,
}

fn wat_benchmark(which: BenchmarkType) -> &'static [u8] {
    match which {
        BenchmarkType::Add => include_bytes!("wat/add.wat"),
        BenchmarkType::AddMem => include_bytes!("wat/add-mem.wat"),
        BenchmarkType::AddVec => include_bytes!("wat/add-vec.wat"),
        BenchmarkType::MatMul64 => include_bytes!("wat/matmul64.wat"),
        BenchmarkType::MatMul128 => include_bytes!("wat/matmul128.wat"),
    }
}

fn arca_benchmark(which: BenchmarkType) -> &'static [u8] {
    match which {
        BenchmarkType::Add => include_bytes!(env!("CARGO_BIN_FILE_UBENCH_add")),
        BenchmarkType::AddMem => include_bytes!(env!("CARGO_BIN_FILE_UBENCH_add-mem")),
        BenchmarkType::AddVec => include_bytes!(env!("CARGO_BIN_FILE_UBENCH_add-vec")),
        BenchmarkType::MatMul64 => include_bytes!(env!("CARGO_BIN_FILE_UBENCH_matmul64")),
        BenchmarkType::MatMul128 => include_bytes!(env!("CARGO_BIN_FILE_UBENCH_matmul128")),
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let parallel = args.parallel.unwrap_or(0);
    let cpus: usize = std::thread::available_parallelism().unwrap().into();
    let parallel = if parallel == 0 { cpus } else { parallel };
    let duration: std::time::Duration = args.duration.into();

    match args.command {
        Commands::Run {
            benchmark,
            program,
            output,
        } => {
            let benchmark: &dyn Benchmark = unsafe {
                match benchmark {
                    BenchmarkMode::Function => &FunctionBenchmark::new(),
                    BenchmarkMode::V8 => {
                        &V8Benchmark::<SameIsolateSameContext>::new(wat_benchmark(program))?
                    }
                    BenchmarkMode::V8ContextPerCall => {
                        &V8Benchmark::<SameIsolateNewContext>::new(wat_benchmark(program))?
                    }
                    BenchmarkMode::V8IsolatePerCall => {
                        &V8Benchmark::<NewIsolate>::new(wat_benchmark(program))?
                    }
                    BenchmarkMode::Wasm2cBoundsChecked => {
                        &Wasm2CBenchmark::new(wat_benchmark(program), false)?
                    }
                    BenchmarkMode::Wasm2cMmap => {
                        &Wasm2CBenchmark::new(wat_benchmark(program), true)?
                    }
                    BenchmarkMode::Arca => &ArcaBenchmark::new(arca_benchmark(program)),
                }
            };

            let mut writer = output.map(csv::Writer::from_path).transpose()?;

            for datum in benchmark.collect_data(parallel, duration) {
                if let Some(ref mut writer) = writer {
                    writer.serialize(datum)?;
                }
            }
        }
        Commands::RunAll { output, program } => {
            std::fs::create_dir_all(output.clone())?;
            let benchmarks: Vec<(&str, Box<dyn Benchmark>)> = unsafe {
                vec![
                    (
                        "v8",
                        Box::new(V8Benchmark::<SameIsolateSameContext>::new(wat_benchmark(
                            program,
                        ))?),
                    ),
                    (
                        "v8-context-per-call",
                        Box::new(V8Benchmark::<SameIsolateNewContext>::new(wat_benchmark(
                            program,
                        ))?),
                    ),
                    (
                        "v8-isolate-per-call",
                        Box::new(V8Benchmark::<NewIsolate>::new(wat_benchmark(program))?),
                    ),
                    (
                        "wasm2c-bounds-checked",
                        Box::new(Wasm2CBenchmark::new(wat_benchmark(program), false)?),
                    ),
                    (
                        "wasm2c-mmap",
                        Box::new(Wasm2CBenchmark::new(wat_benchmark(program), true)?),
                    ),
                    (
                        "arca",
                        Box::new(ArcaBenchmark::new(arca_benchmark(program))),
                    ),
                ]
            };

            for (label, benchmark) in benchmarks {
                let mut file = output.clone();
                file.push(label);
                file.set_extension("csv");
                let mut writer = csv::Writer::from_path(file)?;
                log::info!("running benchmark {label}");
                for datum in benchmark.collect_data(parallel, duration) {
                    writer.serialize(datum)?;
                }
            }
        }
    }
    Ok(())
}
