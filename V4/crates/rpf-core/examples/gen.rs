// cargo run --release -p rpf-core --example gen -- out.rpf <entries> <total_bytes>
use std::io::BufWriter;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.get(2).map(|s| s.as_str()) == Some("compressed") {
        // cargo run --release -p rpf-core --example gen -- out.rpf compressed
        let mut w = BufWriter::new(std::fs::File::create(&a[1]).unwrap());
        rpf_core::testgen::gen_compressed_to(&mut w).unwrap();
        return;
    }
    let mut w = BufWriter::with_capacity(1 << 20, std::fs::File::create(&a[1]).unwrap());
    rpf_core::testgen::gen_to(&mut w, a[2].parse().unwrap(), a[3].parse().unwrap(), None).unwrap();
}
