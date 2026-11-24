use rpf_core::rsc::{self, Resource};
use rpf_core::ypt;
use rpf_core::testgen::sample_ypt_resource as sample;

#[test]
fn read_edit_pack_round_trip() {
    let file = sample();
    let mut r = Resource::decode(&file).unwrap();
    let g = ypt::read(&r).unwrap();
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].kind, g[0].name.as_str()), ("effect", "eff1"));
    assert_eq!(g[0].properties[0].name, "ptxEffectRule:m_colourTintMaxKFP");
    assert_eq!(g[0].properties[0].frames.len(), 2);
    let off = g[0].properties[0].frames[1].offset;
    assert_eq!(g[0].properties[0].frames[1].value, [1.5, 0.25, 0.125, 1.0]);

    assert!(ypt::write(&mut r, &[(off + 1, [0.0; 4])]).is_err(), "only keyframe offsets are accepted");
    assert!(ypt::write(&mut r, &[(off, [f32::NAN, 0.0, 0.0, 0.0])]).is_err());
    ypt::write(&mut r, &[(off, [0.1, 0.2, 0.3, 0.4])]).unwrap();

    let again = Resource::decode(&r.pack()).unwrap();
    assert_eq!(again.version, 0x42);
    let g2 = ypt::read(&again).unwrap();
    assert_eq!(g2[0].properties[0].frames[1].value, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(g2[0].properties[0].frames[0].value, [0.5, 0.25, 0.125, 1.0], "untouched keyframe is unchanged");
    assert!(ypt::to_json(&g2).contains("\"name\":\"eff1\""));
}

#[test]
fn rejects_non_resources() {
    assert!(Resource::decode(b"not a resource at all....").is_err());
    let mut r = Resource::decode(&sample()).unwrap();
    r.data[0x10..0x18].fill(0);
    assert!(ypt::read(&r).is_err());
}
