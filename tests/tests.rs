use glob::glob;
use memedb_core::{read_tags, TagSet};

macro_rules! tagset {
    {$($tag:expr),+} => {{
        let mut m = TagSet::new();
        $(m.insert($tag.to_string());)*
        m
    }};
    {} => {TagSet::new()}
}

#[test]
fn test_read_tags() {
    for file in glob("**/empty.*").unwrap().map(|f| f.unwrap()) {
        assert_eq!(read_tags(&file).unwrap(), tagset! {});
    }
    for file in glob("**/untagged.*").unwrap().map(|f| f.unwrap()) {
        assert_eq!(read_tags(&file).unwrap(), tagset! {});
    }
    for file in glob("**/tagged.*").unwrap().map(|f| f.unwrap()) {
        assert_eq!(read_tags(&file).unwrap(), tagset! {"foo", "bar"});
    }
}

#[test]
fn test_write_tags() {
    // Here be dragons
}
