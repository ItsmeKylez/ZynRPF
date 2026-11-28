//! Writes a small synthetic RPF7 archive (built by the real core) for manual testing:
//! `cargo run -p zyn-server --example make_sample_rpf -- out.rpf [hostile]`
//! With `hostile`, adds an entry whose name contains HTML, to check that UIs never render names as markup.
use tlk_rpf_core::rpf::{Arsiv, Dosya, Dugum};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "sample.rpf".into());
    let mut a = Arsiv::yeni();
    a.dizin_olustur("content").unwrap();
    a.koy("content", Dugum::Dosya(Dosya::yeni("readme.txt", b"hello".to_vec()))).unwrap();
    if std::env::args().nth(2).as_deref() == Some("hostile") {
        a.koy("content", Dugum::Dosya(Dosya::yeni("<img src=x onerror=window.__xss=1>.txt", b"x".to_vec()))).unwrap();
    }
    std::fs::write(&out, a.yaz().unwrap()).unwrap();
    println!("wrote {out}");
}
