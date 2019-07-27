use crate::error::Error;
use crate::TagSet;
use std::io::Read;

pub trait Reader {
    fn read_tags(file: &mut impl Read) -> Result<TagSet, Error>;
    fn write_tags(file: &mut impl Read, tags: &TagSet) -> Result<Vec<u8>, Error>;
}
