## Hello Exif

This example program prints the Make exif tag in the provided images. The source is heavily annotated.
Fundamentals aren't covered, and this is quite far from the best implementation for reading exif data. This
just juggles some raw bytes and byte offsets to print out a single string.

### How do I run this?

It should be sufficient to have Rust and Cargo installed, through Rustup (https://rustup.rs/), and then to run
`cargo run` from the root of this repository. If for some reason this doesnt work for you, let me know!
