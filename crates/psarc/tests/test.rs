use psarc::PlaystationArchive;

#[test]
fn test1() {
    let psarc = PlaystationArchive::parse(include_bytes!("./test.psarc")).unwrap();
    assert_eq!(psarc.len(), 19);

    // Try parsing all files
    psarc.paths_iter().enumerate().for_each(|(index, _)| {
        psarc.read_file(index).unwrap();
    });
}

#[test]
fn test2() {
    let psarc = PlaystationArchive::parse(include_bytes!("./test2.psarc")).unwrap();
    assert_eq!(psarc.len(), 25);

    // Try parsing all files
    psarc.paths_iter().enumerate().for_each(|(_index, _)| {
        // TODO: fix
        //psarc.read_file(index).unwrap();
    });
}
