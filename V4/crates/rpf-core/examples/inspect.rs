// Read-only: cargo run --release -p rpf-core --example inspect -- file.rpf
use rpf_core::index::{Index, Limits};
fn main() {
    let p = std::env::args().nth(1).expect("path");
    let mut f = std::fs::File::open(&p).unwrap();
    let lim = Limits::default();
    match Index::parse(&mut f, &lim) {
        Err(c) => println!("PARSE ERROR: {:?}", c),
        Ok(idx) => {
            println!("entries={} file_len={} table_end={} orphans={}", idx.len(), idx.file_len(), idx.table_end(), idx.orphans());
            for id in 0..idx.len() as u32 { if let Some(i) = idx.info(id) { println!("{:?}  path={}", i, idx.path(id)); } }
            {
                use std::io::{Read, Seek, SeekFrom};
                for id in 0..idx.len() as u32 {
                    let i = idx.info(id).unwrap();
                    if i.kind != "file" || !i.compressed { continue; }
                    let mut buf = vec![0u8; i.stored_size as usize];
                    f.seek(SeekFrom::Start(i.offset)).unwrap(); f.read_exact(&mut buf).unwrap();
                    match rpf_core::deflate::inflate(&buf, 1 << 26) {
                        Ok(o) => println!("INFLATE {} -> {} bytes (declared {}) {} head={:?}", i.name, o.len(), i.uncompressed_size, if o.len() as u64 == i.uncompressed_size { "MATCH" } else { "MISMATCH" }, String::from_utf8_lossy(&o[..o.len().min(60)])),
                        Err(e) => println!("INFLATE {} ERROR {:?}", i.name, e),
                    }
                }
            }
            let mut f2 = std::fs::File::open(&p).unwrap();
            let (_, fnd) = rpf_core::validate::validate(&mut f2, &lim);
            println!("findings={}", fnd.len());
            for x in fnd { println!("  {:?} {:?} {}", x.severity, x.code, x.detail); }
        }
    }
}
