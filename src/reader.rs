use crate::error::Error;
use crate::TagSet;

pub type IoResult = Result<u8, std::io::Error>;

pub trait Reader {
    fn read_tags(file: &mut impl Iterator<Item = IoResult>) -> Result<TagSet, Error>;
    fn write_tags(
        file: &mut impl Iterator<Item = IoResult>,
        tags: &TagSet,
    ) -> Result<Vec<u8>, Error>;
}
