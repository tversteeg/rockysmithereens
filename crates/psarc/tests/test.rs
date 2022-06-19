use psarc::PlaystationArchive;

#[test]
fn test1() {
    let psarc = PlaystationArchive::parse(include_bytes!("./test.psarc")).unwrap();
    // Parse the manifest
    psarc.read_file(0).unwrap();
    dbg!(psarc);
    todo!()
}

#[test]
fn test2() {
    let psarc = PlaystationArchive::parse(include_bytes!("./test2.psarc")).unwrap();
}
