# Snappy framing format read and write

This is a rust implementation of Snappy framing format. It provides
rust std::io friendly Read and Write wrappers to stream to and from
.sz files.

The snappy library is assumed to be present on the system, but
snzip command-line is not required: this reimplements the framing
format in pure rust.

# Usage

Make sure snappy is installed.

Add snzip to your Cargo project:

```
[dependencies]
snzip = "*"
```

Mimicks `snzip -d` (decompress stdin to stdout).

```rust
let mut dec = snzip::framing::Decompressor::new(io::stdin());
io::copy(&mut dec, &mut io::stdout()).unwrap();
```

Mimicks `snzip` (compress stdin to stdout).

```rust
let mut dec = snzip::framing::Compressor::new(io::stdin());
io::copy(&mut dec, &mut io::stdout()).unwrap();
```

There is a `fast` option to be set on the decompressor. It will
ignore checksums and get about 10% faster (YMMV). It is off by
default.

# License
```
Copyright © 2015 Your Name <mathieu@poumeyrol.fr>
This work is free. You can redistribute it and/or modify it under the
terms of the Do What The Fuck You Want To Public License, Version 2,
as published by Sam Hocevar. See the COPYING file for more details.
```
