extern crate srglib;

pub use srglib::prelude::*;

#[test]
fn chain_simple() {
    let chained = chain!(
        SrgMappingsFormat::parse_lines(&[
            "CL: aa Entity",
            "CL: ab Cow",
            "CL: ac EntityPlayer",
            "CL: ad World",
            "CL: ae Server"
        ]).unwrap(),
        SrgMappingsFormat::parse_lines(&[
            "CL: af ForgetfulClass",
            "FD: Entity/a Entity/dead",
            "MD: Cow/a (LCow;)V Cow/love (LCow;)V",
            "MD: EntityPlayer/a (Ljava/lang/String;)V EntityPlayer/disconnect (Ljava/lang/String;)V",
            "FD: World/a World/time",
            "MD: World/a ()V World/tick ()V",
            "FD: Server/a Server/ticks",
            "MD: Server/a ()V Server/tick ()V"
        ]).unwrap()
    );
    let expected = SrgMappingsFormat::parse_lines(&[
        "CL: aa Entity",
        "CL: ab Cow",
        "CL: ac EntityPlayer",
        "CL: ad World",
        "CL: ae Server",
        "CL: af ForgetfulClass",
        "FD: ad/a World/time",
        "FD: aa/a Entity/dead",
        "MD: ab/a (Lab;)V net/minecraft/server/Cow/love (Lnet/minecraft/server/Cow;)V",
        "MD: ac/a (Ljava/lang/String;)V net/minecraft/server/EntityPlayer/disconnect (Ljava/lang/String;)V",
        "FD: ae/a net/minecraft/server/Server/ticks",
        "MD: ad/a ()V World/tick ()V",
        "MD: ae/a ()V net/minecraft/server/Server/tick ()V"
    ]).unwrap();
    expected.assert_equal(&chained)
}

#[test]
fn chain_complex() {
    let chained: FrozenMappings = chain!(
        SrgMappingsFormat::parse_lines(&[
            "CL: aa Entity",
            "CL: ab Cow",
            "CL: ac EntityPlayer",
            "CL: ad World",
            "CL: ae Server"
        ]).unwrap(),
        SrgMappingsFormat::parse_lines(&[
            "CL: af ForgetfulClass",
            "FD: Entity/a Entity/dead",
            "MD: Cow/a (LCow;)V Cow/love (LCow;)V",
            "MD: EntityPlayer/a (Ljava/lang/String;)V EntityPlayer/disconnect (Ljava/lang/String;)V",
            "FD: World/a World/time",
            "MD: World/a ()V World/tick ()V",
            "FD: Server/a Server/ticks",
            "MD: Server/a ()V Server/tick ()V"
        ]).unwrap(),
        SrgMappingsFormat::parse_lines(&[
            "CL: ForgetfulClass me/stupid/ChangedMind",
            "FD: World/time World/numTicks",
            "MD: World/tick ()V World/pulse ()V"
        ]).unwrap()
    );
    let actual = chained.transform_packages(|p| {
        if p.is_empty() {
            Some("net/minecraft/server".into())
        } else {
            None
        }
    });
    let expected = SrgMappingsFormat::parse_lines(&[
        "CL: aa net/minecraft/server/Entity",
        "CL: ab net/minecraft/server/Cow",
        "CL: ac net/minecraft/server/EntityPlayer",
        "CL: ad net/minecraft/server/World",
        "CL: ae net/minecraft/server/Server",
        "CL: af me/stupid/ChangedMind",
        "FD: aa/a net/minecraft/server/Entity/dead",
        "MD: ab/a (Lab;)V net/minecraft/server/Cow/love (Lnet/minecraft/server/Cow;)V",
        "MD: ac/a (Ljava/lang/String;)V net/minecraft/server/EntityPlayer/disconnect (Ljava/lang/String;)V",
        "FD: ad/a net/minecraft/server/World/numTicks",
        "MD: ad/a ()V net/minecraft/server/World/pulse ()V",
        "FD: ae/a net/minecraft/server/Server/ticks",
        "MD: ae/a ()V net/minecraft/server/Server/tick ()V"
    ]).unwrap();
    expected.assert_equal(&actual)
}
