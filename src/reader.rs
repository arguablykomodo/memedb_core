use crate::error::Error;
use std::collections::HashSet;
use std::io::Read;

pub trait Reader {
    fn read_tags(file: &mut impl Read) -> Result<HashSet<String>, Error>;
    fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error>;
}
