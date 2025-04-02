use anyhow::Result;

fn main() -> Result<()> {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo::rerun-if-changed=etc/memmap.ld");
    println!("cargo::rustc-link-arg=-T{}/etc/memmap.ld", dir);
    println!("cargo::rustc-link-arg=-no-pie");
    Ok(())
}
