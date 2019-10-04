use crate::error::Error;
use crate::TagSet;
use std::io::{BufRead, Bytes};

pub trait Reader {
    fn read_tags(file: &mut Bytes<impl BufRead>) -> Result<TagSet, Error>;
    fn write_tags(file: &mut Bytes<impl BufRead>, tags: &TagSet) -> Result<Vec<u8>, Error>;
}
