mod png;

#[derive(Copy, Clone, Debug, PartialEq)]
enum Handler {
    Png,
}

const HANDLERS: &[(&[u8], Handler)] = &[(png::SIGNATURE, Handler::Png)];

fn identify_format(bytes: std::io::Bytes<impl std::io::Read>) -> std::io::Result<Option<Handler>> {
    let mut handlers = HANDLERS.to_vec();
    let length = HANDLERS.iter().map(|(s, _)| s.len()).max().expect("no handlers found");
    for (i, byte) in bytes.take(length).enumerate() {
        let byte = byte?;
        handlers = handlers.into_iter().filter(|(s, _)| s[i] == byte).collect();
        match handlers.len() {
            1 => return Ok(Some(handlers[0].1)),
            0 => return Ok(None),
            _ => (),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn correctly_identify_handlers() {
        for handler in HANDLERS {
            assert_eq!(identify_format(handler.0.bytes()).unwrap(), Some(handler.1));
        }
    }

    #[test]
    fn unknown_format() {
        let bytes = &[0x2E, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(identify_format(bytes.bytes()).unwrap(), None);
    }

    #[test]
    fn short_file() {
        let bytes = &[0x00];
        assert_eq!(identify_format(bytes.bytes()).unwrap(), None);
    }

    #[test]
    fn empty_file() {
        let bytes = &[];
        assert_eq!(identify_format(bytes.bytes()).unwrap(), None);
    }
}
