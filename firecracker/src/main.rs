use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Result;
use clap::Parser;
use firecracker_rs_sdk::{
    firecracker::FirecrackerOption,
    instance::Instance,
    models::{
        BootSource, Drive, MachineConfiguration, SnapshotCreateParams, SnapshotLoadParams,
        SnapshotType,
    },
};
use serde::Serialize;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 10.)]
    duration: f64,
    output: PathBuf,
}

#[derive(Serialize)]
struct Record {
    memory_size: usize,
    iterations: usize,
    duration: f64,
}

fn boot(mem_size_mib: isize) -> Result<()> {
    let mut instance = FirecrackerOption::new("firecracker")
        .api_sock("socket")
        .id("test-instance")
        .level("off")
        .build()?;

    instance.start_vmm()?;
    let version = instance.get_firecracker_version()?;
    println!("{:?}", version);

    instance.put_machine_configuration(&MachineConfiguration {
        cpu_template: None,
        smt: None,
        mem_size_mib,
        track_dirty_pages: None,
        vcpu_count: 1,
        huge_pages: None,
    })?;

    instance.put_guest_boot_source(&BootSource {
        boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off quiet".into()),
        initrd_path: None,
        kernel_image_path: "hello-vmlinux.bin".into(),
    })?;

    instance.put_guest_drive_by_id(&Drive {
        drive_id: "rootfs".into(),
        partuuid: None,
        is_root_device: true,
        cache_type: None,
        is_read_only: true,
        path_on_host: "rootfs.ext4".into(),
        rate_limiter: None,
        io_engine: None,
        socket: None,
    })?;

    instance.start()?;
    std::thread::sleep(Duration::from_secs(1));
    save_instance(instance)
}

fn save_instance(mut instance: Instance) -> Result<()> {
    instance.pause()?;
    instance.create_snapshot(&SnapshotCreateParams {
        mem_file_path: "/tmp/firecracker.memory".into(),
        snapshot_path: "/tmp/firecracker.snapshot".into(),
        snapshot_type: Some(SnapshotType::Full),
        version: None,
    })?;
    stop_instance(instance)?;
    Ok(())
}

fn run() -> Result<()> {
    let instance = resume_instance()?;
    save_instance(instance)
}

fn resume_instance() -> Result<Instance> {
    let mut instance = FirecrackerOption::new("firecracker")
        .api_sock("socket")
        .id("test-instance")
        .level("off")
        .build()?;
    instance.start_vmm()?;

    instance.load_snapshot(&SnapshotLoadParams {
        enable_diff_snapshots: None,
        mem_file_path: Some("/tmp/firecracker.memory".into()),
        mem_backend: None,
        resume_vm: Some(true),
        snapshot_path: "/tmp/firecracker.snapshot".into(),
    })?;
    Ok(instance)
}

fn stop_instance(mut instance: Instance) -> Result<()> {
    instance.stop()?;
    Ok(())
}

fn experiment(mib: usize, duration: Duration) -> Result<Record> {
    println!("running with {mib} MiB of RAM");
    let memory_size = mib * 1024 * 1024;
    boot(mib as isize)?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        run()?;
    }

    let start = Instant::now();
    let mut iterations = 0;
    loop {
        run()?;
        if start.elapsed() > duration {
            break;
        }
        iterations += 1;
    }

    println!(
        "ran {iterations} iterations in {duration:?} ({:?}/iter)",
        if iterations > 0 {
            Some(duration / iterations as u32)
        } else {
            None
        }
    );

    let _ = std::fs::remove_file("socket");
    let _ = std::fs::remove_file("/tmp/firecracker.memory");
    let _ = std::fs::remove_file("/tmp/firecracker.snapshot");
    Ok(Record {
        memory_size,
        iterations,
        duration: duration.as_secs_f64(),
    })
}

fn main() -> Result<()> {
    let args = Args::parse();
    let _ = std::fs::remove_file("socket");
    let _ = std::fs::remove_file("/tmp/firecracker.memory");
    let _ = std::fs::remove_file("/tmp/firecracker.snapshot");

    let mut output = csv::Writer::from_path(args.output)?;

    for mib in [64, 128, 256, 512, 1024, 2048, 4096] {
        output.serialize(experiment(mib, Duration::from_secs_f64(args.duration))?)?;
    }

    Ok(())
}
