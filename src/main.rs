use std::{path::PathBuf, time::Duration};

use benchmark::{
    self,
    arca::ArcaBenchmark,
    v8::{NewIsolate, SameIsolateNewContext, SameIsolateSameContext, V8Benchmark},
    wasm2c::Wasm2CBenchmark,
};

use benchmark::Benchmark;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name = "Arca Benchmark")]
#[command(version, about)]
struct Args {
    /// How many threads to use at maximum (default: the number of CPUs)
    #[arg(short, long)]
    parallel: Option<usize>,
    /// How long to benchmark
    #[arg(short, long, default_value = "1s")]
    duration: humantime::Duration,
    /// How long to warm up
    #[arg(short, long, default_value = "100ms")]
    warmup: humantime::Duration,
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
    Everything {
        /// Output directory
        directory: PathBuf,
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
    #[clap(name = "matmul64")]
    MatMul64,
    /// Multiply two 128x128 matrices
    #[clap(name = "matmul128")]
    MatMul128,
}

#[derive(clap::ValueEnum, Clone, Debug, Copy)]
enum BenchmarkMode {
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

fn run_benchmark(
    parallel: usize,
    warmup: Duration,
    duration: Duration,
    benchmark: BenchmarkMode,
    program: BenchmarkType,
    output: Option<PathBuf>,
) -> Result<()> {
    let benchmark: &dyn Benchmark = unsafe {
        match benchmark {
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
            BenchmarkMode::Wasm2cMmap => &Wasm2CBenchmark::new(wat_benchmark(program), true)?,
            BenchmarkMode::Arca => &ArcaBenchmark::new(arca_benchmark(program)),
        }
    };

    let mut writer = output.map(csv::Writer::from_path).transpose()?;

    for datum in benchmark.collect_data(parallel, warmup, duration) {
        if let Some(ref mut writer) = writer {
            writer.serialize(datum)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let parallel = args.parallel.unwrap_or(0);
    let cpus: usize = std::thread::available_parallelism().unwrap().into();
    let parallel = if parallel == 0 { cpus } else { parallel };
    let warmup: std::time::Duration = args.warmup.into();
    let duration: std::time::Duration = args.duration.into();

    let benchmarks = &[
        ("v8", BenchmarkMode::V8),
        ("v8-context-per-call", BenchmarkMode::V8ContextPerCall),
        ("v8-isolate-per-call", BenchmarkMode::V8IsolatePerCall),
        ("wasm2c-bounds-checked", BenchmarkMode::Wasm2cBoundsChecked),
        ("wasm2c-mmap", BenchmarkMode::Wasm2cMmap),
        ("arca", BenchmarkMode::Arca),
    ];

    let programs = &[
        ("add", BenchmarkType::Add),
        ("add-mem", BenchmarkType::AddMem),
        ("matmul64", BenchmarkType::MatMul64),
        ("matmul128", BenchmarkType::MatMul128),
    ];

    match args.command {
        Commands::Run {
            benchmark,
            program,
            output,
        } => {
            run_benchmark(parallel, warmup, duration, benchmark, program, output)?;
        }
        Commands::RunAll { output, program } => {
            std::fs::create_dir_all(&output)?;
            for (i, (label, benchmark)) in benchmarks.iter().enumerate() {
                let benchmarks_left = (benchmarks.len() - i) as u32;
                let iterations = parallel.ilog2();
                let time = duration + warmup;
                let time_left = time * benchmarks_left * iterations;
                log::info!("running benchmark \"{label}\"; {time_left:?} remaining");
                let mut file = output.clone();
                file.push(label);
                file.set_extension("csv");
                run_benchmark(parallel, warmup, duration, *benchmark, program, Some(file))?;
            }
        }
        Commands::Everything { directory } => {
            std::fs::create_dir_all(&directory)?;
            let iterations = parallel.ilog2();
            let time = duration + warmup;
            let benchmarks_per_program = benchmarks.len() as u32;
            for (i, (prog, program)) in programs.iter().enumerate() {
                let mut output = directory.clone();
                output.push(prog);
                std::fs::create_dir_all(&output)?;
                let programs_left = (programs.len() - i) as u32;
                let time_after = benchmarks_per_program * programs_left * time * iterations;
                log::info!("running program \"{prog}\"");
                for (j, (bench, benchmark)) in benchmarks.iter().enumerate() {
                    let benchmarks_left = benchmarks_per_program - j as u32;
                    let time_left = time * (benchmarks_left * iterations) + time_after;
                    log::info!(
                        "running benchmark \"{bench}\" on program \"{prog}\"; {time_left:?} remaining"
                    );
                    let mut file = output.clone();
                    file.push(bench);
                    file.set_extension("csv");
                    run_benchmark(parallel, warmup, duration, *benchmark, *program, Some(file))?;
                }
            }
        }
    }
    Ok(())
}
