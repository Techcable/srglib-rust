extern crate srglib;

use srglib::prelude::*;

const TEST_LINES: &[&str] = &[
    "CL: org/spigotmc/XRay net/techcable/xray/XRay",
    "CL: org/spigotmc/XRay$Manager net/techcable/xray/XRayManager",
    "CL: org/spigotmc/XRay$Injector net/techcable/xray/injector/Injector",
    "CL: org/spigotmc/XRay$Injector$Manager net/techcable/xray/injector/InjectorManager",
    "CL: obfs net/techcable/minecraft/NoHax",
    "CL: obf4 net/techcable/minecraft/Player",
    "FD: obf4/a net/techcable/minecraft/Player/dead",
    "FD: obf4/b net/techcable/minecraft/Player/blood",
    "FD: obf4/c net/techcable/minecraft/Player/health",
    "FD: obf4/d net/techcable/minecraft/Player/speed",
    "FD: org/spigotmc/XRay$Injector$Manager/taco net/techcable/xray/injector/InjectorManager/seriousVariableName",
    "MD: obfs/a (Lobf4;ID)Z net/techcable/minecraft/NoHax/isHacking (Lnet/techcable/minecraft/Player;ID)Z",
    "MD: org/spigotmc/XRay/deobfuscate ([BLjava/util/Set;)I net/techcable/xray/XRay/doAFunkyDance ([BLjava/util/Set;)I",
    "MD: org/spigotmc/XRay$Manager/aquire ()Lorg/spigotmc/XRay; net/techcable/xray/XRayManager/get ()Lnet/techcable/xray/XRay;"
];
const COMPACT_TEST_LINES: &[&str] = &[
    "org/spigotmc/XRay net/techcable/xray/XRay",
    "org/spigotmc/XRay$Manager net/techcable/xray/XRayManager",
    "org/spigotmc/XRay$Injector net/techcable/xray/injector/Injector",
    "org/spigotmc/XRay$Injector$Manager net/techcable/xray/injector/InjectorManager",
    "obfs net/techcable/minecraft/NoHax",
    "obf4 net/techcable/minecraft/Player",
    "obf4 a dead",
    "obf4 b blood",
    "obf4 c health",
    "obf4 d speed",
    "org/spigotmc/XRay$Injector$Manager taco seriousVariableName",
    "obfs a (Lobf4;ID)Z isHacking",
    "org/spigotmc/XRay deobfuscate ([BLjava/util/Set;)I doAFunkyDance",
    "org/spigotmc/XRay$Manager aquire ()Lorg/spigotmc/XRay; get"
];

#[test]
fn compact_srg() {
    test_parse::<CompactSrgMappingsFormat>(COMPACT_TEST_LINES);
    test_serialize::<CompactSrgMappingsFormat>(COMPACT_TEST_LINES);
}

#[test]
fn srg() {
    test_parse::<SrgMappingsFormat>(TEST_LINES);
    test_serialize::<SrgMappingsFormat>(TEST_LINES);
}

#[test]
fn srg_packages() {
    let result = SrgMappingsFormat::parse_lines(&[
        "CL: a food",
        "CL: b bathroom",
        "PK: ./ net/minecraft/server",
    ]).unwrap();
    assert_eq!(result.remap_class_name("a").internal_name(), "net/minecraft/server/food");
    assert_eq!(result.remap_class_name("b").internal_name(), "net/minecraft/server/bathroom");
}


fn test_parse<F: MappingsFormat>(test_lines: &[&str]) {
    let result = F::parse_lines(test_lines).unwrap();
    assert_eq!("net.techcable.xray.XRay", result.remap_class_name("org.spigotmc.XRay").name());
    assert_eq!("net.techcable.minecraft.Player", result.remap_class_name("obf4").name());
    assert_eq!(
        MethodData::new(
            "isHacking".into(),
            ReferenceType::from_name("net.techcable.minecraft.NoHax"),
            MethodSignature::new(
                PrimitiveType::Boolean.into_type_descriptor(),
                vec![
                    ReferenceType::from_name("net.techcable.minecraft.Player")
                        .into_type_descriptor(),
                    PrimitiveType::Int.into_type_descriptor(),
                    PrimitiveType::Double.into_type_descriptor(),
                ]
            )
        ),
        result.remap_method(&MethodData::new(
            "a".into(),
            ReferenceType::from_name("obfs"),
            MethodSignature::new(
                PrimitiveType::Boolean.into_type_descriptor(),
                vec![
                    ReferenceType::from_name("obf4")
                        .into_type_descriptor(),
                    PrimitiveType::Int.into_type_descriptor(),
                    PrimitiveType::Double.into_type_descriptor(),
                ]
            )
        ))
    );
    assert_eq!(
        FieldData::new(
            "dead".into(),
            ReferenceType::from_name("net.techcable.minecraft.Player"),
        ),
        result.remap_field(&FieldData::new(
            "a".into(),
            ReferenceType::from_name("obf4")
        ))
    );
}
fn test_serialize<T: MappingsFormat>(test_lines: &[&str]) {
    let expected = T::parse_lines(test_lines).unwrap();
    let serialized = T::write_line_array(&expected);
    let actual = T::parse_lines(&serialized).unwrap();
    assert_eq!(expected, actual);
}