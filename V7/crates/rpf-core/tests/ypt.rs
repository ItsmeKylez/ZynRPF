use rpf_core::rsc::{self, Kaynak};
use rpf_core::testgen::sample_ypt_resource as sample;
use rpf_core::ypt;

#[test]
fn read_edit_pack_round_trip() {
    let file = sample();
    let mut r = Kaynak::coz(&file).unwrap();
    let g = ypt::oku(&r).unwrap();
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].tur, g[0].ad.as_str()), ("efekt", "eff1"));
    assert_eq!(g[0].ozellikler[0].ad, "ptxEffectRule:m_colourTintMaxKFP");
    assert_eq!(g[0].ozellikler[0].kareler.len(), 2);
    let off = g[0].ozellikler[0].kareler[1].ofs;
    assert_eq!(g[0].ozellikler[0].kareler[1].deger, [1.5, 0.25, 0.125, 1.0]);

    assert!(ypt::yaz(&mut r, &[(off + 1, [0.0; 4])]).is_err(), "only keyframe offsets are accepted");
    assert!(ypt::yaz(&mut r, &[(off, [f32::NAN, 0.0, 0.0, 0.0])]).is_err());
    ypt::yaz(&mut r, &[(off, [0.1, 0.2, 0.3, 0.4])]).unwrap();

    let packed = r.paketle();
    assert!(packed.len() < r.veri.len() / 2, "real compression: packed {} vs decoded {}", packed.len(), r.veri.len());
    let again = Kaynak::coz(&packed).unwrap();
    assert_eq!(again.surum, 0x42);
    let g2 = ypt::oku(&again).unwrap();
    assert_eq!(g2[0].ozellikler[0].kareler[1].deger, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(g2[0].ozellikler[0].kareler[0].deger, [0.5, 0.25, 0.125, 1.0], "untouched keyframe is unchanged");
}

#[test]
fn rejects_non_resources_and_bombs() {
    assert!(Kaynak::coz(b"not a resource at all....").is_err());
    let mut r = Kaynak::coz(&sample()).unwrap();
    r.veri[0x10..0x18].fill(0);
    assert!(ypt::oku(&r).is_err());
    // Header flags that declare an enormous decoded size must be refused before allocating.
    let mut huge = Vec::new();
    for x in [rsc::RSC7, 0x42, 0x0FFF_FFFFu32, 0x0FFF_FFFF] { huge.extend_from_slice(&x.to_le_bytes()); }
    huge.extend_from_slice(&[0x03, 0x00]);
    assert!(Kaynak::coz(&huge).is_err());
}
