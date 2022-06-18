use psarc::PlaystationArchive;

#[test]
fn test1() {
    let psarc = PlaystationArchive::parse(include_bytes!("./test.psarc")).unwrap();
}

#[test]
fn test2() {
    let psarc = PlaystationArchive::parse(include_bytes!("./test2.psarc")).unwrap();
}
