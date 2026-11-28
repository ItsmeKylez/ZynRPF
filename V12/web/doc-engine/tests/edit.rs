//! Edit-engine tests against synthetic archives built by the real core.
use std::io::Read;
use std::sync::Arc;
use tlk_rpf_core::rpf::{Arsiv, Dosya, Dugum};
use zyn_doc_engine::tree::{Node, NodeKind};
use zyn_doc_engine::{Document, HISTORY_LIMIT, ImportItem};

fn sample() -> Document {
    let mut a = Arsiv::yeni();
    a.dizin_olustur("content").unwrap();
    a.dizin_olustur("content/sub").unwrap();
    a.koy("content", Dugum::Dosya(Dosya::yeni("readme.txt", b"hello".to_vec()))).unwrap();
    a.koy("content/sub", Dugum::Dosya(Dosya::yeni("nested.txt", b"n".to_vec()))).unwrap();
    a.koy("", Dugum::Dosya(Dosya::yeni("top.bin", vec![0, 1, 2, 3]))).unwrap();
    a.koy("", Dugum::Dosya(Dosya::yeni("visualsettings.dat", b"a 1\nb 2\n".to_vec()))).unwrap();
    a.koy("", Dugum::Dosya(Dosya::yeni("evil\\..\\x.txt", b"e".to_vec()))).unwrap();
    Document::open("t.rpf", Arc::from(a.yaz().unwrap())).unwrap()
}

fn flat(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        out.push(n.path.clone());
        flat(&n.children, out);
    }
}

fn paths(d: &Document) -> Vec<String> {
    let mut v = Vec::new();
    flat(&d.summary(1).tree, &mut v);
    v.sort();
    v
}

fn s(x: &str) -> String {
    x.to_string()
}

fn text(d: &Document, p: &str) -> String {
    String::from_utf8(d.read_file(p).unwrap()).unwrap()
}

fn rpf_bytes() -> Vec<u8> {
    let mut a = Arsiv::yeni();
    a.koy("", Dugum::Dosya(Dosya::yeni("in.txt", b"inside".to_vec()))).unwrap();
    a.yaz().unwrap()
}

#[test]
fn edit_undo_redo_and_dirty_flag() {
    let mut d = sample();
    assert!(!d.summary(1).dirty);
    d.write_text("content/readme.txt", "changed".into()).unwrap();
    let sm = d.summary(1);
    assert!(sm.dirty && sm.undo_depth == 1 && sm.redo_depth == 0);
    assert_eq!(text(&d, "content/readme.txt"), "changed");

    assert!(d.undo());
    let sm = d.summary(1);
    assert!(!sm.dirty, "undoing back to the saved state is clean again");
    assert_eq!((sm.undo_depth, sm.redo_depth), (0, 1));
    assert_eq!(text(&d, "content/readme.txt"), "hello");

    assert!(d.redo());
    assert!(d.summary(1).dirty);
    assert_eq!(text(&d, "content/readme.txt"), "changed");
    assert!(!d.redo() && d.undo() && d.undo() == false);
}

#[test]
fn new_edit_after_undo_clears_redo() {
    let mut d = sample();
    d.write_text("content/readme.txt", "one".into()).unwrap();
    d.undo();
    assert_eq!(d.summary(1).redo_depth, 1);
    d.write_text("content/readme.txt", "two".into()).unwrap();
    assert_eq!(d.summary(1).redo_depth, 0);
}

#[test]
fn failed_edit_rolls_back_and_keeps_history_and_redo() {
    let mut d = sample();
    d.write_text("content/readme.txt", "one".into()).unwrap();
    d.undo();
    let before = paths(&d);
    let rev = d.revision();
    let err = d.rename("content/missing.txt", "x.txt").unwrap_err();
    assert_eq!(err.code, "NOT_FOUND");
    let sm = d.summary(1);
    assert_eq!((sm.undo_depth, sm.redo_depth), (0, 1), "failed edit must not consume redo");
    assert_eq!(paths(&d), before);
    assert_eq!(d.revision(), rev);
}

#[test]
fn multi_path_delete_is_all_or_nothing() {
    let mut d = sample();
    let err = d.delete(vec![s("content/readme.txt"), s("content/nope")]).unwrap_err();
    assert_eq!(err.code, "NOT_FOUND");
    assert_eq!(text(&d, "content/readme.txt"), "hello", "first path must not stay deleted");
    assert_eq!(d.summary(1).undo_depth, 0);
}

#[test]
fn history_is_capped() {
    let mut d = sample();
    for i in 0..(HISTORY_LIMIT + 10) {
        d.create_text(&format!("f{i}.txt"), "x".into()).unwrap();
    }
    assert_eq!(d.summary(1).undo_depth, HISTORY_LIMIT);
}

#[test]
fn import_builds_folders_opens_nested_rpf_and_is_one_undo_step() {
    let mut d = sample();
    let n = d
        .import(
            "content",
            vec![
                ImportItem { rel_path: s("skins/a/b.txt"), data: b"bb".to_vec() },
                ImportItem { rel_path: s("skins/c.bin"), data: vec![9, 9] },
                ImportItem { rel_path: s("inner.rpf"), data: rpf_bytes() },
                ImportItem { rel_path: s("broken.rpf"), data: b"not an rpf".to_vec() },
            ],
        )
        .unwrap();
    assert_eq!(n, 4);
    let p = paths(&d);
    for want in ["content/skins", "content/skins/a", "content/skins/a/b.txt", "content/skins/c.bin", "content/inner.rpf", "content/inner.rpf/in.txt", "content/broken.rpf"] {
        assert!(p.contains(&s(want)), "missing {want} in {p:?}");
    }
    let tree = d.summary(1).tree;
    let content = tree.iter().find(|n| n.name == "content").unwrap();
    let kind_of = |name: &str| content.children.iter().find(|n| n.name == name).map(|n| n.kind.clone());
    assert!(matches!(kind_of("inner.rpf"), Some(NodeKind::Archive)));
    assert!(matches!(kind_of("broken.rpf"), Some(NodeKind::File)), "an unparsable .rpf stays a plain file");
    assert_eq!(d.summary(1).undo_depth, 1);
    d.undo();
    assert!(!paths(&d).iter().any(|p| p.contains("skins") || p.contains("inner.rpf")));
}

#[test]
fn import_rejects_hostile_paths_without_changing_anything() {
    let mut d = sample();
    let rev = d.revision();
    for bad in ["../x.txt", "a//b.txt", "", "/abs.txt", "a/../b.txt", "x\\y.txt", "dir/"] {
        let r = d.import("", vec![ImportItem { rel_path: bad.to_string(), data: vec![1] }]);
        assert!(r.is_err(), "should reject {bad:?}");
    }
    assert!(d.import("../up", vec![ImportItem { rel_path: s("a.txt"), data: vec![] }]).is_err());
    assert!(d.import("", vec![]).is_err());
    assert_eq!(d.revision(), rev);
    assert_eq!(d.summary(1).undo_depth, 0);
}

#[test]
fn import_file_over_folder_name_is_a_conflict_and_rolls_back() {
    let mut d = sample();
    let err = d.import("", vec![ImportItem { rel_path: s("top.bin/x.txt"), data: vec![1] }]).unwrap_err();
    assert_eq!(err.code, "CONFLICT");
    assert_eq!(d.summary(1).undo_depth, 0);
}

#[test]
fn replace_file_and_nested_archive() {
    let mut d = sample();
    d.replace_file("top.bin", vec![7, 7, 7]).unwrap();
    assert_eq!(d.read_file("top.bin").unwrap(), vec![7, 7, 7]);
    assert_eq!(d.replace_file("nope.bin", vec![1]).unwrap_err().code, "NOT_FOUND");

    d.import("", vec![ImportItem { rel_path: s("pack.rpf"), data: rpf_bytes() }]).unwrap();
    let mut other = Arsiv::yeni();
    other.koy("", Dugum::Dosya(Dosya::yeni("new.txt", b"fresh".to_vec()))).unwrap();
    d.replace_file("pack.rpf", other.yaz().unwrap()).unwrap();
    assert_eq!(d.read_file("pack.rpf/new.txt").unwrap(), b"fresh");
    assert!(d.read_file("pack.rpf/in.txt").is_err());
    let depth = d.summary(1).undo_depth;
    let err = d.replace_file("pack.rpf", b"garbage".to_vec()).unwrap_err();
    assert!(["RPF_INVALID", "RPF_CORRUPT"].contains(&err.code), "{err}");
    assert_eq!(d.summary(1).undo_depth, depth);
    assert_eq!(d.read_file("pack.rpf/new.txt").unwrap(), b"fresh", "bad replacement must leave the archive intact");
}

#[test]
fn delete_prunes_nested_selection() {
    let mut d = sample();
    d.delete(vec![s("content"), s("content/readme.txt"), s("CONTENT/SUB")]).unwrap();
    assert!(!paths(&d).iter().any(|p| p.starts_with("content")));
    assert_eq!(d.summary(1).undo_depth, 1);
    assert!(d.delete(vec![]).is_err());
    assert!(d.delete(vec![s("../x")]).is_err());
}

#[test]
fn rename_mkdir_and_name_validation() {
    let mut d = sample();
    d.rename("content/readme.txt", "README2.txt").unwrap();
    assert_eq!(text(&d, "content/README2.txt"), "hello");
    for bad in ["a/b", "a\\b", "", "..", "x\n"] {
        assert_eq!(d.rename("content/README2.txt", bad).unwrap_err().code, "INVALID_NAME", "{bad:?}");
    }
    d.mkdir("content/newdir").unwrap();
    assert!(paths(&d).contains(&s("content/newdir")));
    assert!(d.mkdir("../esc").is_err());
    assert!(d.mkdir("a/b\\c").is_err());
}

#[test]
fn move_works_and_never_moves_a_folder_into_itself() {
    let mut d = sample();
    d.move_to(vec![s("top.bin")], "content/sub").unwrap();
    assert!(paths(&d).contains(&s("content/sub/top.bin")) && !paths(&d).contains(&s("top.bin")));
    d.move_to(vec![s("content/sub/top.bin")], "").unwrap();
    assert!(paths(&d).contains(&s("top.bin")), "moving to the root works");
    for (p, dir) in [("content", "content/sub"), ("content", "content"), ("Content", "content/SUB")] {
        let before = paths(&d);
        let e = d.move_to(vec![s(p)], dir).unwrap_err();
        assert_eq!(e.code, "INVALID_REQUEST", "{p} -> {dir}");
        assert_eq!(paths(&d), before);
    }
    assert_eq!(d.move_to(vec![s("nope")], "content").unwrap_err().code, "NOT_FOUND");
}

#[test]
fn stale_revision_is_detected() {
    let mut d = sample();
    let r = d.revision();
    assert!(d.ensure_revision(Some(r)).is_ok() && d.ensure_revision(None).is_ok());
    d.mkdir("x").unwrap();
    assert_eq!(d.ensure_revision(Some(r)).unwrap_err().code, "STALE_DOCUMENT");
    assert!(d.ensure_revision(Some(d.revision())).is_ok());
    d.undo();
    assert!(d.ensure_revision(Some(r)).is_ok(), "undo returns to the earlier state id");
}

#[test]
fn save_baseline_and_reload() {
    let mut d = sample();
    d.write_text("content/readme.txt", "v2".into()).unwrap();
    let bytes: Arc<[u8]> = Arc::from(d.serialize_verified().unwrap());
    d.mark_saved(bytes.clone()).unwrap();
    let sm = d.summary(1);
    assert!(!sm.dirty);
    assert_eq!(sm.undo_depth, 1, "history survives a save");
    assert_eq!(sm.disk_size, bytes.len() as u64);
    d.undo();
    assert!(d.summary(1).dirty, "undoing past the saved state makes the document dirty");
    d.redo();
    assert!(!d.summary(1).dirty);
    assert_eq!(text(&d, "content/readme.txt"), "v2");

    let before_rev = d.revision();
    d.reload(Arc::from(sample().serialize_verified().unwrap())).unwrap();
    let sm = d.summary(1);
    assert_eq!((sm.undo_depth, sm.redo_depth, sm.dirty), (0, 0, false));
    assert_ne!(d.revision(), before_rev);
    assert_eq!(text(&d, "content/readme.txt"), "hello");
    assert!(d.reload(Arc::from(b"junk".to_vec())).is_err());
    assert_eq!(text(&d, "content/readme.txt"), "hello", "failed reload keeps the document");
}

#[test]
fn edited_archive_round_trips_through_verified_serialization() {
    let mut d = sample();
    d.import("content", vec![ImportItem { rel_path: s("a/b.txt"), data: b"zz".to_vec() }]).unwrap();
    d.rename("top.bin", "renamed.bin").unwrap();
    d.delete(vec![s("content/sub")]).unwrap();
    let out = d.serialize_verified().unwrap();
    let again = Document::open("again.rpf", Arc::from(out)).unwrap();
    assert_eq!(again.read_file("content/a/b.txt").unwrap(), b"zz");
    assert_eq!(again.read_file("renamed.bin").unwrap(), vec![0, 1, 2, 3]);
    assert!(again.read_file("content/sub/nested.txt").is_err());
}

#[test]
fn entry_views() {
    let mut d = sample();
    let t = d.entry_view("content/readme.txt").unwrap();
    assert_eq!((t.kind, t.text.as_deref(), t.size), ("text", Some("hello"), 5));
    assert_eq!(d.entry_view("visualsettings.dat").unwrap().kind, "setting");
    let b = d.entry_view("top.bin").unwrap();
    assert_eq!(b.kind, "binary");
    assert!(b.hex_dump.unwrap().starts_with("00000000  00 01 02 03"));
    assert_eq!(d.entry_view("content").unwrap_err().code, "NOT_FOUND");
    assert_eq!(d.entry_view("../x").unwrap_err().code, "INVALID_PATH");
    // Huge text falls back to a hex preview so responses stay bounded.
    let big = vec![b'a'; zyn_doc_engine::entry::MAX_TEXT_VIEW_BYTES + 1];
    d.replace_file("content/readme.txt", big).unwrap();
    let v = d.entry_view("content/readme.txt").unwrap();
    assert_eq!(v.kind, "binary");
    assert!(v.text.is_none());
}

fn read_zip(bytes: Vec<u8>) -> (Vec<String>, zip::ZipArchive<std::io::Cursor<Vec<u8>>>) {
    let z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names = z.file_names().filter_map(|n| n.ok()).map(|n| n.to_string()).collect::<Vec<_>>();
    (names, z)
}

#[test]
fn export_zip_contents_nesting_and_safe_names() {
    let mut d = sample();
    d.import("", vec![ImportItem { rel_path: s("pack.rpf"), data: rpf_bytes() }]).unwrap();

    let (bytes, files) = d.export_zip(vec![s("content"), s("content/readme.txt"), s("pack.rpf")], 1 << 20).unwrap();
    assert_eq!(files, 3, "folder contents + nested archive; the nested duplicate selection is pruned");
    let (names, mut z) = read_zip(bytes);
    for want in ["content/", "content/readme.txt", "content/sub/", "content/sub/nested.txt", "pack.rpf"] {
        assert!(names.iter().any(|n| n == want), "missing {want} in {names:?}");
    }
    let mut got = String::new();
    z.by_name("content/readme.txt").unwrap().read_to_string(&mut got).unwrap();
    assert_eq!(got, "hello");
    let mut inner = Vec::new();
    z.by_name("pack.rpf").unwrap().read_to_end(&mut inner).unwrap();
    assert_eq!(Document::open("p", Arc::from(inner)).unwrap().read_file("in.txt").unwrap(), b"inside");

    // Whole archive: the hostile entry name (backslashes / "..") must not escape or create odd paths.
    let (bytes, _) = d.export_zip(vec![s("")], 1 << 20).unwrap();
    let (names, _) = read_zip(bytes);
    assert!(names.iter().all(|n| n.split('/').all(|c| c != "..") && !n.contains('\\') && !n.starts_with('/')), "{names:?}");
    assert!(names.iter().any(|n| n == "evil_.._x.txt"), "{names:?}");
}

#[test]
fn export_limits_and_errors() {
    let d = sample();
    assert_eq!(d.export_zip(vec![], 1 << 20).unwrap_err().code, "INVALID_REQUEST");
    assert_eq!(d.export_zip(vec![s("../x")], 1 << 20).unwrap_err().code, "INVALID_PATH");
    assert_eq!(d.export_zip(vec![s("nope")], 1 << 20).unwrap_err().code, "NOT_FOUND");
    assert_eq!(d.export_zip(vec![s("")], 3).unwrap_err().code, "EXPORT_TOO_LARGE");
}
