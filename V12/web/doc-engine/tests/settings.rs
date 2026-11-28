use std::sync::Arc;
use tlk_rpf_core::rpf::{Arsiv, Dosya, Dugum};
use zyn_doc_engine::Document;

const VS: &str = "# comment line\r\nlight.intensity 1.5f\r\nlight.color.red 255\r\nlight.color.green 128\r\nlight.color.blue 0\r\n\r\nfog.density 0.25\r\nfog.density 0.30\r\n// another\r\nlonely\r\n";

fn doc() -> Document {
    let mut a = Arsiv::yeni();
    a.dizin_olustur("common/data").unwrap();
    a.koy("common/data", Dugum::Dosya(Dosya::yeni("visualsettings.dat", VS.as_bytes().to_vec()))).unwrap();
    a.koy("", Dugum::Dosya(Dosya::yeni("blob.bin", vec![0xff, 0xfe, 0, 1]))).unwrap();
    Document::open("t.rpf", Arc::from(a.yaz().unwrap())).unwrap()
}
const P: &str = "common/data/visualsettings.dat";

fn raw(d: &Document) -> String {
    String::from_utf8(d.read_file(P).unwrap()).unwrap()
}

#[test]
fn view_lists_entries_duplicates_and_colours() {
    let v = doc().settings_view(P).unwrap();
    let keys: Vec<_> = v.entries.iter().map(|e| e.key.as_str()).collect();
    assert!(keys.contains(&"light.intensity") && keys.contains(&"fog.density"));
    assert!(!keys.contains(&"lonely"), "a key without a value is not an entry");
    assert_eq!(v.entries.iter().find(|e| e.key == "fog.density").unwrap().repeat, 2);
    assert_eq!(v.entries.iter().find(|e| e.key == "light.intensity").unwrap().value, "1.5f");
    assert_eq!(v.colors.len(), 1);
    assert_eq!(v.colors[0].base, "light.color");
    assert_eq!(v.colors[0].keys, ["light.color.red", "light.color.green", "light.color.blue"]);
    assert_eq!(v.colors[0].scale, 255.0);
}

#[test]
fn apply_preserves_formatting_float_suffix_and_all_duplicates_and_is_one_undo_step() {
    let mut d = doc();
    d.settings_apply(P, vec![("light.intensity".into(), "2.25".into()), ("fog.density".into(), "0.5".into()), ("light.color.green".into(), "10".into())]).unwrap();
    let t = raw(&d);
    assert_eq!(t, VS.replace("1.5f", "2.25f").replace("0.25", "0.5").replace("0.30", "0.5").replace("128", "10"));
    assert!(t.contains("# comment line\r\n") && t.contains("// another\r\n") && t.contains("lonely\r\n"));
    assert_eq!(d.summary(1).undo_depth, 1);
    assert!(d.summary(1).dirty);
    d.undo();
    assert_eq!(raw(&d), VS);
}

#[test]
fn rejects_unknown_keys_and_corrupting_values_without_changing_anything() {
    let mut d = doc();
    let rev = d.revision();
    let cases: Vec<(Vec<(String, String)>, &str)> = vec![
        (vec![("nope".into(), "1".into())], "NOT_FOUND"),
        (vec![("light.intensity".into(), "1\r\ninjected.key 5".into())], "INVALID_REQUEST"),
        (vec![("light.intensity".into(), "1 2".into())], "INVALID_REQUEST"),
        (vec![("light.intensity".into(), "  ".into())], "INVALID_REQUEST"),
        (vec![("light intensity".into(), "1".into())], "INVALID_REQUEST"),
        (vec![("light.intensity".into(), "9".into()), ("missing".into(), "1".into())], "NOT_FOUND"),
        (vec![], "INVALID_REQUEST"),
    ];
    for (vals, code) in cases {
        assert_eq!(d.settings_apply(P, vals).unwrap_err().code, code);
    }
    assert_eq!(raw(&d), VS, "a refused batch changes nothing, not even its valid first entry");
    assert_eq!(d.revision(), rev);
    assert_eq!(d.summary(1).undo_depth, 0);
}

#[test]
fn missing_and_hostile_paths() {
    let mut d = doc();
    assert_eq!(d.settings_view("common/data/none.dat").unwrap_err().code, "NOT_FOUND");
    assert_eq!(d.settings_view("../x").unwrap_err().code, "INVALID_PATH");
    assert_eq!(d.settings_apply("common/data", vec![("a".into(), "1".into())]).unwrap_err().code, "NOT_FOUND");
    // A binary file is read lossily like on the desktop: it yields no entries rather than an error.
    assert!(d.settings_view("blob.bin").unwrap().entries.is_empty());
}
