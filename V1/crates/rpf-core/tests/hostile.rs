use rpf_core::format::*;
use rpf_core::testgen::{expected_file_bytes, gen_to};
use rpf_core::{Code, Index, Limits};
use std::io::Cursor;
use std::panic;

fn parse(b: &[u8]) -> Result<Index, Code> {
    Index::parse(&mut Cursor::new(b), &Limits::default())
}
fn set32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn base(n: u32, dirs: Option<u32>) -> Vec<u8> {
    let mut v = Vec::new();
    gen_to(&mut v, n, n as u64 * 1024, dirs).unwrap();
    v
}
fn e(i: usize) -> usize {
    16 + i * 16
}
fn chain(n: u32) -> Vec<u8> {
    let names = b"\0d\0".to_vec();
    let mut v = Vec::new();
    v.extend_from_slice(&MAGIC.to_le_bytes());
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&(names.len() as u32).to_le_bytes());
    v.extend_from_slice(&OPEN.to_le_bytes());
    for i in 0..n {
        let en = Entry { name: if i == 0 { 0 } else { 1 }, kind: Kind::Dir { first: i + 1, count: if i + 1 < n { 1 } else { 0 } } };
        v.extend_from_slice(&encode(&en));
    }
    v.extend_from_slice(&names);
    v
}

#[test]
fn valid_archive_parses_and_browses() {
    let b = base(20, None);
    let idx = parse(&b).unwrap();
    assert_eq!(idx.len(), 20);
    assert_eq!(idx.name(0), "");
    assert_eq!(idx.path(0), "/");
    assert_eq!(idx.children(0), Some((1, 1)));
    assert_eq!(idx.name(1), "d00000");
    assert_eq!(idx.name(2), "f0000.bin");
    assert_eq!(idx.path(2), "/d00000/f0000.bin");
    assert_eq!(idx.info(2).unwrap().kind, "file");
    assert_eq!(idx.orphans(), 0);
    let hits = idx.search("F0003", 10);
    assert_eq!(hits, vec![5]);
    // stored bytes equal the generator's deterministic content
    let (off, len) = idx.stored_span(3).unwrap();
    assert_eq!(&b[off as usize..(off + len) as usize], &expected_file_bytes(1, len as usize)[..]);
}

#[test]
fn hostile_fixtures_are_rejected_with_expected_codes() {
    let b = base(20, None);
    let ns = 16 + 20 * 16;
    let names_len = u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize;
    let mut cases: Vec<(&str, Vec<u8>, Code)> = vec![
        ("empty", vec![], Code::TooShort),
        ("15 bytes", vec![0x37; 15], Code::TooShort),
        ("all zero", vec![0; 4096], Code::BadMagic),
        ("truncated", b[..600].to_vec(), Code::DataOob),
        ("chain300", chain(300), Code::DepthExceeded),
    ];
    let mut m = b.clone(); set32(&mut m, 4, 0xFFFF_FFFF); cases.push(("count max", m, Code::TooManyEntries));
    let mut m = b.clone(); set32(&mut m, 4, 1_000_001); cases.push(("count 1M+1", m, Code::TooManyEntries));
    let mut m = b.clone(); set32(&mut m, 8, 0xFFFF_FFFF); cases.push(("names max", m, Code::NamesTooLarge));
    let mut m = b.clone(); set32(&mut m, 4, 1_000_000); cases.push(("table oob", m, Code::TableOutOfBounds));
    let mut m = b.clone(); set32(&mut m, 12, 0x0FFF_FFF9); cases.push(("aes", m, Code::EncryptedUnsupported));
    let mut m = b.clone(); m[e(2)] = 0xFF; m[e(2) + 1] = 0xFF; cases.push(("name off oob", m, Code::NameOffsetOob));
    let mut m = b.clone(); m[ns + 1..ns + 4].copy_from_slice(b"../"); cases.push(("slash", m, Code::NameInvalid));
    let mut m = b.clone(); m[ns + 1..ns + 4].copy_from_slice(b"..\0"); cases.push(("dotdot", m, Code::NameInvalid));
    let mut m = b.clone(); m[ns + 1] = 0; cases.push(("empty name", m, Code::NameInvalid));
    let mut m = b.clone(); m[ns + 1] = 0xFF; cases.push(("bad utf8", m, Code::NameInvalid));
    let mut m = b.clone(); m[ns + 1] = b'\\'; cases.push(("backslash", m, Code::NameInvalid));
    let mut m = b.clone(); m[ns + 1] = 0x07; cases.push(("control char", m, Code::NameInvalid));
    let mut m = b.clone(); m[ns + names_len - 1] = b'x'; cases.push(("unterminated", m, Code::NameUnterminated));
    let mut m = b.clone(); set32(&mut m, e(1) + 8, 1); set32(&mut m, e(1) + 12, 1); cases.push(("self loop", m, Code::TableCycle));
    let mut m = base(20, Some(2)); let f1 = u32::from_le_bytes([m[e(1) + 8], m[e(1) + 9], m[e(1) + 10], m[e(1) + 11]]); set32(&mut m, e(2) + 8, f1); cases.push(("shared child", m, Code::SharedChild));
    let mut m = b.clone(); set32(&mut m, e(0) + 12, 1000); cases.push(("range oob", m, Code::DirRangeOob));
    let mut m = b.clone(); m[e(2) + 5..e(2) + 8].copy_from_slice(&[0xFE, 0xFF, 0x7F]); cases.push(("data oob", m, Code::DataOob));
    let mut m = b.clone(); m[e(2) + 5..e(2) + 8].copy_from_slice(&[0, 0, 0]); cases.push(("data in table", m, Code::DataInTable));
    let mut m = b.clone(); set32(&mut m, 4, 0); cases.push(("count 0", m, Code::NoRoot));
    let mut m = b.clone(); let s: Vec<u8> = m[e(2)..e(2) + 16].to_vec(); m[e(0)..e(0) + 16].copy_from_slice(&s); cases.push(("root is file", m, Code::NoRoot));
    let mut m = b.clone(); set32(&mut m, 8, 0); cases.push(("names len 0", m, Code::NameOffsetOob));
    for (name, data, want) in cases {
        match parse(&data) {
            Err(got) => assert_eq!(got, want, "case {}", name),
            Ok(_) => panic!("case {} was accepted", name),
        }
    }
}

#[test]
fn limits_are_configurable() {
    let b = base(20, None);
    let lim = Limits { max_entries: 10, ..Limits::default() };
    assert_eq!(Index::parse(&mut Cursor::new(&b), &lim).err(), Some(Code::TooManyEntries));
    let lim = Limits { max_depth: 0, ..Limits::default() };
    assert_eq!(Index::parse(&mut Cursor::new(&b), &lim).err(), Some(Code::DepthExceeded));
}

#[test]
fn mutation_fuzz_never_panics() {
    panic::set_hook(Box::new(|_| {}));
    let b = base(20, None);
    let mut s: u64 = 0xDEAD_BEEF_1234_5678;
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for _ in 0..100_000 {
        let mut m = b.clone();
        for _ in 0..(1 + rnd() % 4) {
            let p = (rnd() % m.len() as u64) as usize;
            m[p] = rnd() as u8;
        }
        if rnd() % 5 == 0 {
            let l = (rnd() % m.len() as u64) as usize;
            m.truncate(l);
        }
        let r = panic::catch_unwind(|| {
            if let Ok(idx) = parse(&m) {
                // browsing an accepted (mutated) index must also never panic
                for id in 0..idx.len() as u32 {
                    let _ = idx.info(id);
                    let _ = idx.path(id);
                }
                let _ = idx.search("f0", 5);
            }
        });
        assert!(r.is_ok(), "panic on mutated input");
    }
}
