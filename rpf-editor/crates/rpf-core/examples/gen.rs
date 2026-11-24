// cargo run --release -p rpf-core --example gen -- out.rpf <entries> <total_bytes>
use std::io::BufWriter;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut w = BufWriter::with_capacity(1 << 20, std::fs::File::create(&a[1]).unwrap());
    rpf_core::testgen::gen_to(&mut w, a[2].parse().unwrap(), a[3].parse().unwrap(), None).unwrap();
}
