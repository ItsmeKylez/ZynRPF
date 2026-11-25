// Read-only on the input: cargo run --release -p rpf-core --example realtrip -- in.rpf out_dir
use rpf_core::rebuild::{rebuild, BlobSource, Op};
use rpf_core::validate::{has_errors, validate};
use rpf_core::{Index, Limits};
use std::io::{self, Cursor, Read};
struct B(Vec<u8>);
impl BlobSource for B { fn open(&self, _: u64) -> io::Result<Box<dyn Read>> { Ok(Box::new(Cursor::new(self.0.clone()))) } }
fn parse(b: &[u8]) -> Index { Index::parse(&mut Cursor::new(b), &Limits::default()).unwrap() }
fn run(src: &[u8], ops: &[Op], blob: &B) -> Vec<u8> {
    let lim = Limits::default(); let idx = parse(src); let mut out = Vec::new();
    rebuild(&idx, &mut Cursor::new(src), ops, blob, &lim, &mut out).expect("rebuild"); out
}
fn span<'a>(b: &'a [u8], i: &Index, id: u32) -> &'a [u8] { let (o, l) = i.stored_span(id).unwrap(); &b[o as usize..(o + l) as usize] }
fn check(name: &str, ok: bool) { println!("{} {}", if ok { "PASS" } else { "FAIL" }, name); if !ok { std::process::exit(1) } }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read(&a[1]).unwrap(); let before = src.clone();
    let none = B(vec![]); let sa = parse(&src);
    // 1. no-op
    let o1 = run(&src, &[], &none); let s1 = parse(&o1);
    let (_, f) = validate(&mut Cursor::new(&o1), &Limits::default());
    check("no-op output validates with no errors", !has_errors(&f));
    check("same entry count and paths", sa.len() == s1.len() && (0..sa.len() as u32).all(|i| sa.path(i) == s1.path(i)));
    let mut same = true;
    for i in 0..sa.len() as u32 { let (x, y) = (sa.info(i).unwrap(), s1.info(i).unwrap());
        if x.kind != y.kind || x.stored_size != y.stored_size || x.uncompressed_size != y.uncompressed_size || x.encryption_tag != y.encryption_tag || x.compressed != y.compressed { same = false; }
        if sa.stored_span(i).is_some() && span(&src, &sa, i) != span(&o1, &s1, i) { same = false; } }
    check("every entry's kind, sizes, flags and stored bytes identical", same);
    check("second rebuild is byte-identical (deterministic)", run(&o1, &[], &none) == o1);
    // 2. edit: replace root KaiWorks.txt, add file, rename dir, delete one
    let blob = B(b"Edited by the new RPF editor.\r\n".to_vec());
    let ops = vec![
        Op::Replace { path: "/KaiWorks.txt".into(), blob: 1, len: blob.0.len() as u64 },
        Op::AddFile { parent: "/".into(), name: "new.txt".into(), blob: 1, len: blob.0.len() as u64 },
        Op::Rename { path: "/content/80f93fd2-edbb-4ead-aada-84d2ad27f9e6".into(), new_name: "blood".into() },
        Op::Delete { path: "/content/KaiWorks.txt".into() },
    ];
    let o2 = run(&src, &ops, &blob); let s2 = parse(&o2);
    let (_, f2) = validate(&mut Cursor::new(&o2), &Limits::default());
    check("edited output validates with no errors", !has_errors(&f2));
    let find = |i: &Index, p: &str| (0..i.len() as u32).find(|&x| i.path(x) == p);
    let (ob, nb) = (find(&sa, "/content/80f93fd2-edbb-4ead-aada-84d2ad27f9e6/bloodfx.dat").unwrap(), find(&s2, "/content/blood/bloodfx.dat").unwrap());
    check("renamed folder's bloodfx.dat bytes identical", span(&src, &sa, ob) == span(&o2, &s2, nb));
    let (op, np) = (find(&sa, "/content/503a865a-6fa2-42ad-8187-4002e09f96ad/ptfx_hi.rpf").unwrap(), find(&s2, "/content/503a865a-6fa2-42ad-8187-4002e09f96ad/ptfx_hi.rpf").unwrap());
    check("3 MB nested ptfx_hi.rpf bytes identical", span(&src, &sa, op) == span(&o2, &s2, np));
    let (oa, na) = (find(&sa, "/assembly.xml").unwrap(), find(&s2, "/assembly.xml").unwrap());
    check("assembly.xml bytes identical", span(&src, &sa, oa) == span(&o2, &s2, na));
    check("replaced and added files hold the new bytes", find(&s2, "/KaiWorks.txt").map(|i| span(&o2, &s2, i) == &blob.0[..]).unwrap_or(false) && find(&s2, "/new.txt").is_some());
    check("deleted entry is gone", find(&s2, "/content/KaiWorks.txt").is_none());
    check("input bytes untouched in memory", src == before);
    std::fs::create_dir_all(&a[2]).unwrap();
    std::fs::write(format!("{}/Zyn_noop.rpf", a[2]), &o1).unwrap(); std::fs::write(format!("{}/Zyn_edited.rpf", a[2]), &o2).unwrap();
    println!("sizes: in={} noop={} edited={}", src.len(), o1.len(), o2.len());
}
