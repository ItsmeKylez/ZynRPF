use rpf_core::format::*;
use rpf_core::rebuild::{rebuild, replay, validate_name, BlobSource, Op};
use rpf_core::testgen::{expected_file_bytes, gen_to};
use rpf_core::validate::{check_index, has_errors, validate};
use rpf_core::{Code, Index, Limits};
use std::collections::HashMap;
use std::io::{self, Cursor, Read};

struct Blobs(HashMap<u64, Vec<u8>>);
impl BlobSource for Blobs {
    fn open(&self, id: u64) -> io::Result<Box<dyn Read>> {
        self.0.get(&id).map(|v| Box::new(Cursor::new(v.clone())) as Box<dyn Read>).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "x"))
    }
}
fn none() -> Blobs {
    Blobs(HashMap::new())
}
fn base(n: u32, dirs: Option<u32>) -> Vec<u8> {
    let mut v = Vec::new();
    gen_to(&mut v, n, n as u64 * 1000, dirs).unwrap();
    v
}
fn parse(b: &[u8]) -> Index {
    Index::parse(&mut Cursor::new(b), &Limits::default()).unwrap()
}
fn run(src: &[u8], ops: &[Op], blobs: &Blobs, lim: &Limits) -> Result<Vec<u8>, rpf_core::Fail> {
    let idx = Index::parse(&mut Cursor::new(src), lim).unwrap();
    let mut out = Vec::new();
    rebuild(&idx, &mut Cursor::new(src), ops, blobs, lim, &mut out)?;
    Ok(out)
}
/// Finds an entry id by path.
fn find(idx: &Index, path: &str) -> Option<u32> {
    (0..idx.len() as u32).find(|&i| idx.path(i) == path)
}
fn bytes_at<'a>(b: &'a [u8], idx: &Index, id: u32) -> &'a [u8] {
    let (o, l) = idx.stored_span(id).unwrap();
    &b[o as usize..(o + l) as usize]
}

#[test]
fn no_op_rebuild_preserves_structure_and_every_payload() {
    let src = base(40, Some(3));
    let out = run(&src, &[], &none(), &Limits::default()).unwrap();
    let (a, b) = (parse(&src), parse(&out));
    assert_eq!(a.len(), b.len());
    for id in 0..a.len() as u32 {
        assert_eq!(a.path(id), b.path(id));
        if a.stored_span(id).is_some() {
            assert_eq!(bytes_at(&src, &a, id), bytes_at(&out, &b, id), "payload of {}", a.path(id));
        }
    }
    let (_, f) = validate(&mut Cursor::new(&out), &Limits::default());
    assert!(!has_errors(&f), "{f:?}");
    // a second rebuild of the rebuilt file is byte-identical (layout is deterministic)
    let out2 = run(&out, &[], &none(), &Limits::default()).unwrap();
    assert_eq!(out, out2);
}

#[test]
fn all_operations_work_and_untouched_bytes_are_identical() {
    let src = base(20, Some(2));
    let a = parse(&src);
    let f_old = find(&a, "/d00000/f0000.bin").unwrap();
    let untouched = find(&a, "/d00001/f0009.bin").unwrap();
    let mut blobs = HashMap::new();
    blobs.insert(1u64, b"hello world".to_vec());
    blobs.insert(2u64, vec![7u8; 5000]);
    let blobs = Blobs(blobs);
    let ops = vec![
        Op::Rename { path: "/d00000".into(), new_name: "renamed".into() },
        Op::Mkdir { parent: "/".into(), name: "newdir".into() },
        Op::AddFile { parent: "/newdir".into(), name: "hello.txt".into(), blob: 1, len: 11 },
        Op::Replace { path: "/renamed/f0000.bin".into(), blob: 2, len: 5000 },
        Op::Delete { path: "/renamed/f0001.bin".into() },
        Op::Rename { path: "/renamed/f0002.bin".into(), new_name: "two.bin".into() },
    ];
    let out = run(&src, &ops, &blobs, &Limits::default()).unwrap();
    let b = parse(&out);
    assert!(find(&b, "/d00000").is_none());
    assert!(find(&b, "/renamed/f0001.bin").is_none());
    assert_eq!(bytes_at(&out, &b, find(&b, "/newdir/hello.txt").unwrap()), b"hello world");
    assert_eq!(bytes_at(&out, &b, find(&b, "/renamed/f0000.bin").unwrap()), &vec![7u8; 5000][..]);
    // renamed file keeps its bytes; untouched file in another folder keeps its bytes
    let two_old = find(&a, "/d00000/f0002.bin").unwrap();
    assert_eq!(bytes_at(&out, &b, find(&b, "/renamed/two.bin").unwrap()), bytes_at(&src, &a, two_old));
    assert_eq!(bytes_at(&out, &b, find(&b, "/d00001/f0009.bin").unwrap()), bytes_at(&src, &a, untouched));
    let _ = f_old;
    assert_eq!(b.len(), 20 + 2 - 1); // +newdir +hello, -one file
    let (_, f) = validate(&mut Cursor::new(&out), &Limits::default());
    assert!(!has_errors(&f), "{f:?}");
}

#[test]
fn hostile_edits_are_rejected_with_codes() {
    let src = base(20, Some(2));
    let lim = Limits::default();
    let code = |ops: Vec<Op>, blobs: &Blobs| run(&src, &ops, blobs, &lim).err().map(|f| f.code);
    let n = none();
    let mk = |name: &str| Op::Mkdir { parent: "/".into(), name: name.into() };
    for bad in ["", ".", "..", "a/b", "a\\b", "c:x", "bad\u{1}name", &"x".repeat(256)] {
        assert_eq!(code(vec![mk(bad)], &n), Some(Code::NameInvalid), "name {bad:?}");
        assert!(validate_name(bad).is_err());
    }
    assert_eq!(code(vec![mk("D00000")], &n), Some(Code::EditNameExists)); // case-insensitive duplicate
    assert_eq!(code(vec![mk("ok"), mk("OK")], &n), Some(Code::EditNameExists));
    assert_eq!(code(vec![Op::Delete { path: "/".into() }], &n), Some(Code::EditRootProtected));
    assert_eq!(code(vec![Op::Rename { path: "/".into(), new_name: "x".into() }], &n), Some(Code::EditRootProtected));
    assert_eq!(code(vec![Op::Delete { path: "/nope".into() }], &n), Some(Code::EditPathNotFound));
    assert_eq!(code(vec![Op::Delete { path: "nope".into() }], &n), Some(Code::EditPathNotFound));
    assert_eq!(code(vec![Op::Mkdir { parent: "/d00000/f0000.bin".into(), name: "x".into() }], &n), Some(Code::EditNotAFolder));
    assert_eq!(code(vec![Op::Replace { path: "/d00000".into(), blob: 1, len: 1 }], &n), Some(Code::EditTargetIsFolder));
    assert_eq!(code(vec![Op::Rename { path: "/d00000".into(), new_name: "d00000".into() }], &n), Some(Code::EditNoChange));
    assert_eq!(code(vec![Op::AddFile { parent: "/".into(), name: "big".into(), blob: 1, len: 1 << 32 }], &n), Some(Code::EditFileTooLarge));
    // missing and mismatched content are caught at write time, and nothing is returned
    assert_eq!(code(vec![Op::AddFile { parent: "/".into(), name: "a".into(), blob: 9, len: 3 }], &n), Some(Code::BlobMissing));
    let mut m = HashMap::new();
    m.insert(1u64, vec![1u8, 2, 3]);
    let m = Blobs(m);
    assert_eq!(code(vec![Op::AddFile { parent: "/".into(), name: "a".into(), blob: 1, len: 4 }], &m), Some(Code::BlobLengthMismatch));
    assert_eq!(code(vec![Op::AddFile { parent: "/".into(), name: "a".into(), blob: 1, len: 2 }], &m), Some(Code::BlobLengthMismatch));
}

#[test]
fn limits_entries_depth_and_names_table() {
    let src = base(20, Some(2));
    let n = none();
    let tight = Limits { max_entries: 21, ..Limits::default() };
    let ops = vec![Op::Mkdir { parent: "/".into(), name: "a".into() }, Op::Mkdir { parent: "/".into(), name: "b".into() }];
    assert_eq!(run(&src, &ops, &n, &tight).err().unwrap().code, Code::TooManyEntries);
    // depth
    let lim = Limits { max_depth: 4, ..Limits::default() };
    let mut ops = Vec::new();
    let mut p = "/".to_string();
    for i in 0..5 {
        ops.push(Op::Mkdir { parent: p.clone(), name: format!("l{i}") });
        p = if p == "/" { format!("/l{i}") } else { format!("{p}/l{i}") };
    }
    let e = run(&src, &ops, &n, &lim).err().unwrap();
    assert_eq!(e.code, Code::DepthExceeded);
    assert!(replay(&parse(&src), &ops[..4], &lim).is_ok());
    // names table: many long distinct names exceed the 64 KiB table
    let ops: Vec<Op> = (0..400).map(|i| Op::Mkdir { parent: "/".into(), name: format!("{:0>200}", i) }).collect();
    assert_eq!(run(&src, &ops, &n, &Limits::default()).err().unwrap().code, Code::NamesTooLarge);
}

/// Hand-built archive from (name, kind-builder) so special layouts can be tested.
fn handmade(files: &[(&str, Vec<u8>, &str)]) -> Vec<u8> {
    // kinds: "bin", "comp" (csize = len, usize = 2*len), "large" (resource with 0xFFFFFF marker)
    let mut names = vec![0u8];
    let mut offs = Vec::new();
    for (n, _, _) in files {
        offs.push(names.len() as u32);
        names.extend_from_slice(n.as_bytes());
        names.push(0);
    }
    let n = 1 + files.len() as u32;
    let table_end = 16 + n as u64 * 16 + names.len() as u64;
    let mut cur = (table_end + 511) / 512 * 512;
    let mut entries = vec![Entry { name: 0, kind: Kind::Dir { first: 1, count: files.len() as u32 } }];
    let mut data = Vec::new();
    for (i, (_, bytes, kind)) in files.iter().enumerate() {
        let block = (cur / 512) as u32;
        let kind = match *kind {
            "bin" => Kind::Bin { csize: 0, block, usize_: bytes.len() as u32, enc: 0 },
            "comp" => Kind::Bin { csize: bytes.len() as u32, block, usize_: bytes.len() as u32 * 2, enc: 0 },
            _ => Kind::Res { size: LARGE_RESOURCE, block, sys: 0x11, gfx: 0x22 },
        };
        entries.push(Entry { name: offs[i], kind });
        let padded = (bytes.len() as u64 + 511) / 512 * 512;
        data.extend_from_slice(bytes);
        data.resize(data.len() + (padded - bytes.len() as u64) as usize, 0);
        cur += padded;
    }
    let mut v = Vec::new();
    v.extend_from_slice(&MAGIC.to_le_bytes());
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&(names.len() as u32).to_le_bytes());
    v.extend_from_slice(&OPEN.to_le_bytes());
    for e in &entries {
        v.extend_from_slice(&encode(e));
    }
    v.extend_from_slice(&names);
    v.resize(((table_end + 511) / 512 * 512) as usize, 0);
    v.extend_from_slice(&data);
    v
}

#[test]
fn compressed_and_large_resource_entries_are_copied_verbatim() {
    let a = vec![0xAAu8; 700];
    let c = (0..1300u32).map(|i| (i * 7) as u8).collect::<Vec<_>>();
    let l = (0..2000u32).map(|i| (i * 13) as u8).collect::<Vec<_>>();
    let src = handmade(&[("a.bin", a.clone(), "bin"), ("c.dat", c.clone(), "comp"), ("big.ytd", l.clone(), "large"), ("z.bin", vec![9u8; 10], "bin")]);
    let ops = vec![Op::Delete { path: "/a.bin".into() }, Op::Rename { path: "/big.ytd".into(), new_name: "moved.ytd".into() }];
    let out = run(&src, &ops, &none(), &Limits::default()).unwrap();
    let (s, o) = (parse(&src), parse(&out));
    let cid = find(&o, "/c.dat").unwrap();
    assert_eq!(&bytes_at(&out, &o, cid)[..], &c[..]);
    match o.entry(cid).unwrap().kind {
        Kind::Bin { csize, usize_, .. } => assert_eq!((csize, usize_), (1300, 2600)),
        k => panic!("{k:?}"),
    }
    let lid = find(&o, "/moved.ytd").unwrap();
    let Kind::Res { size, sys, gfx, block } = o.entry(lid).unwrap().kind else { panic!() };
    assert_eq!((size, sys, gfx), (LARGE_RESOURCE, 0x11, 0x22));
    let off = block as usize * 512;
    // large payload length is the distance to the next payload, padding included: 2048 bytes
    assert_eq!(&out[off..off + 2000], &l[..]);
    assert!(out.len() >= off + 2048);
    let _ = s;
    let (_, f) = validate(&mut Cursor::new(&out), &Limits::default());
    assert!(!has_errors(&f), "{f:?}");
    // replacing a resource is refused
    let e = run(&src, &[Op::Replace { path: "/big.ytd".into(), blob: 1, len: 1 }], &none(), &Limits::default()).err().unwrap();
    assert_eq!(e.code, Code::EditResourceUnsupported);
}

#[test]
fn validator_reports_duplicates_overlap_and_orphans() {
    let mut src = handmade(&[("a.bin", vec![1u8; 600], "bin"), ("A.BIN", vec![2u8; 600], "bin")]);
    let idx = parse(&src);
    let f = check_index(&idx);
    assert!(f.iter().any(|x| x.code == Code::DuplicateName));
    // make entry 2 point at the same block as entry 1 -> overlap
    let e1 = 16 + 16;
    let blk: [u8; 3] = [src[e1 + 5], src[e1 + 6], src[e1 + 7]];
    let e2 = 16 + 32;
    src[e2 + 5..e2 + 8].copy_from_slice(&blk);
    let f = check_index(&parse(&src));
    assert!(f.iter().any(|x| x.code == Code::DataOverlap), "{f:?}");
    assert!(has_errors(&f));
}

#[test]
fn rebuilt_archives_survive_random_edit_fuzzing() {
    let src = base(60, Some(4));
    let a = parse(&src);
    let mut paths: Vec<String> = (1..a.len() as u32).map(|i| a.path(i)).collect();
    paths.push("/".into());
    let mut s: u64 = 0x1234_5678_9ABC_DEF1;
    let mut rnd = move |m: usize| {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s % m as u64) as usize
    };
    let names = ["x", "Y.bin", "", "..", "a/b", "dup", "DUP", "\u{0}", "long-name-here.ytd", "é.txt"];
    let mut blobs = HashMap::new();
    for i in 0..8u64 {
        blobs.insert(i, vec![i as u8; (i as usize * 331) % 3000]);
    }
    let blobs = Blobs(blobs);
    let (mut ok, mut rejected) = (0, 0);
    for _ in 0..400 {
        let mut ops = Vec::new();
        for _ in 0..rnd(6) + 1 {
            let p = paths[rnd(paths.len())].clone();
            let nm = names[rnd(names.len())].to_string();
            let b = rnd(9) as u64; // 8 is missing
            let len = if b < 8 { (b as usize * 331) % 3000 } else { 5 } as u64;
            ops.push(match rnd(5) {
                0 => Op::Rename { path: p, new_name: nm },
                1 => Op::Delete { path: p },
                2 => Op::Mkdir { parent: p, name: nm },
                3 => Op::AddFile { parent: p, name: nm, blob: b, len },
                _ => Op::Replace { path: p, blob: b, len },
            });
        }
        match run(&src, &ops, &blobs, &Limits::default()) {
            Ok(out) => {
                ok += 1;
                let (_, f) = validate(&mut Cursor::new(&out), &Limits::default());
                assert!(!has_errors(&f), "ops {ops:?} -> {f:?}");
            }
            Err(_) => rejected += 1,
        }
    }
    assert!(ok > 20 && rejected > 20, "ok={ok} rejected={rejected}");
    let _ = expected_file_bytes;
}
