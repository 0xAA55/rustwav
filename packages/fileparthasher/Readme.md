# File hasher

File hasher to calculate the hash for a section of a file, the hash is `u64` size. The `Write` trait was implemented for it.

## Overview

The file hasher uses the `DefaultHasher` to hash a file.
```rust
#[derive(Debug, Clone)]
pub struct FileHasher {
    hasher: DefaultHasher,
}
```

It can hash part of a file, its hash function protocol is this:
`pub fn hash<R>(&mut self, reader: &mut R, from_byte: u64, length: u64) -> Result<u64, Error> where R: Read + Seek;`

