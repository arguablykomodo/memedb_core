# MemeDB Core

Rust library for reading and writing tags to media files, specifically for the use of categorizing memes.

`memedb_core` reads and writes tags to common image and video formats. The objective of the library is for front-end applications to use it as a way to categorize and organize a user's meme collection.

## Formats

| Format | Status                                     |
| -----: | :----------------------------------------- |
|    PNG | Supported (via [ancilliary chunk][1])      |
|   JPEG | Supported (via [EXIF][2])                  |
|    GIF | Supported (via [application extension][3]) |
|   WEBM | Planned to support                         |

[1]: https://en.wikipedia.org/wiki/Portable_Network_Graphics#%22Chunks%22_within_the_file
[2]: https://en.wikipedia.org/wiki/Exif
[3]: https://www.matthewflickinger.com/lab/whatsinagif/bits_and_bytes.asp
