use crate::error::Error;
use crate::reader::Reader;
use std::collections::HashSet;
use std::io::Read;

const SIGNATURE: &[u8] = &[];

pub struct GifReader {}

impl Reader for GifReader {
    fn read_tags(file: &mut impl Read) -> Result<HashSet<String>, Error> {
        Ok(HashSet::new())
    }
    fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
        Ok(vec![])
    }
}
