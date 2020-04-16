pub const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub fn read_tags(bytes: &mut std::io::Bytes<impl std::io::Read>) -> crate::TagSet {
    unimplemented!()
}
