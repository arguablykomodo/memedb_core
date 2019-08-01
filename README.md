# MemeDB Core

Rust library for reading and writing tags to media files, specifically for the use of categorizing memes.

`memedb_core` will read and write tags to common image and video formats. The objective of the library is for front-end applications to use it as a way to categorize and organize a user's meme collection.

## Formats

|                    |          PNG          | JPEG  |            GIF             | WEBM |
| -----------------: | :-------------------: | :---: | :------------------------: | :--: |
|          Supported |          ❌           |  ❌   |             ❌             |  ❌  |
| Planned to support |          ✅           |  ✅   |             ✅             |  ✅  |
|             Method | [Ancilliary chunk][1] | EXIF? | [Application Extension][2] | ???  |

[1]: https://en.wikipedia.org/wiki/Portable_Network_Graphics#%22Chunks%22_within_the_file
[2]: https://www.matthewflickinger.com/lab/whatsinagif/bits_and_bytes.asp
