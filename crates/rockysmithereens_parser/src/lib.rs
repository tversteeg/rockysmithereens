use psarc::PlaystationArchive;

pub fn parse(file: &[u8]) {
    let archive = PlaystationArchive::parse(file).unwrap();
    dbg!(archive.read_file(0).unwrap());
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
