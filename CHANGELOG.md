# Changelog

## Next version (Unreleased)

### BREAKING CHANGES

- Use trait impls to allow more generic inputs.
- Change error type.
- Change the way PNG tags are stored.
- Change the way JPEG tags are stored.
- Add some restrictions on which tags are allowed (check the documentation of
  `is_tag_valid` for more information).

### Additions

- Added support for RIFF containers, which include WEBP, WAV, and AVI, among
  others.

### Changes

- Try to store tags near the beggining of files.

### Internal Changes

- Completely rewrite library.
- Follow [Conventional Commits guidelines](https://www.conventionalcommits.org).
- Start [keeping a changelog](https://keepachangelog.com).
- Add usage examples.

## [1.0.2] - 2019-12-09

### Fixes

- Fix false positive from format identifier due to eager return.

## [1.0.1] - 2019-10-28

### Fixes

- Fix crash in the XML parser of the JPEG reader.

### Internal Changes

- Improve logging in JPEG reader.

## [1.0.0] - 2019-10-23

### Additions

- Initial support for PNG, JPEG, and GIF.

[1.0.2]: https://gitlab.com/memedb/memedb_core/-/compare/v1.0.1...v1.0.2
[1.0.1]: https://gitlab.com/memedb/memedb_core/-/compare/v1.0.0...v1.0.1
[1.0.0]: https://gitlab.com/memedb/memedb_core/-/commits/v1.0.0
