use std::{io, io::prelude::*};
use std::fs::File;
use std::fmt;
use std::str;
use std::panic;

// Utility function to turn format a Vec<u8> into a LowerHex formatted String repr.
fn byte_vec_to_hex_string(v: &Vec<u8>) -> String {
    v.iter().map(|value| format!("{:02x} ", value)).collect::<String>()
}

// Tuple type. This is easy since we defined our ByteSlice to just hold that
// vec. We access the elements of a tuple with a .index notation, i.e. bs.0 for the 0th element,
// which in this case is our Vec<u8>.
struct ByteSlice(Vec<u8>);

// Create a ByteSlice from a Vec<u8>.
impl From<Vec<u8>> for ByteSlice {
    fn from(v: Vec<u8>) -> Self {
        ByteSlice(v)
    }
}

// LowerHex formatter impl for a ByteSlice tuple type, defined above.
impl fmt::LowerHex for ByteSlice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let repr = byte_vec_to_hex_string(&self.0);
        write!(f, "{}", repr)
    }
}

// Image File Directory, source: https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf
// https://www.exif.org/Exif2-2.PDF, 4.6.2 IFD Structure
#[derive(Default)]
struct IFD {
    tag: u16,
    tag_type: u16,
    count: u32,
    value_offset: u32,
}

// From for IFD. This enables IFD::from(&[u8]) (an IFD struct from a u8 slice)
impl From<&[u8]> for IFD {
    fn from(v: &[u8]) -> Self {
        // [u8; 2] is an array comprised of u8 values, here assigned a length 2 array filled with 0s.
        // This section is a bit kludgey, since it'd be cool to be able to instantiate an array
        // from an arbitrary slice. Arrays are typed over some contiguous type and a length, and
        // since slices are arbitrary over the length of a program (&v[..1], &v[2..e], etc), I can
        // see why we want to be careful.
        let mut tag_bytes: [u8; 2] = [0; 2];
        tag_bytes.copy_from_slice(&v[..2]);

        let mut type_bytes: [u8; 2] = [0; 2];
        type_bytes.copy_from_slice(&v[2..4]);

        let mut count_bytes: [u8; 4] = [0; 4];
        count_bytes.copy_from_slice(&v[4..8]);

        let mut value_offset_bytes: [u8; 4] = [0; 4];
        value_offset_bytes.copy_from_slice(&v[8..]);

        IFD {
            tag: u16::from_le_bytes(tag_bytes),
            tag_type: u16::from_le_bytes(type_bytes),
            count: u32::from_le_bytes(count_bytes),
            value_offset: u32::from_le_bytes(value_offset_bytes)
        }
    }
}

// LowerHex formatter for our IFD struct.
// We implement this formatter so that we can print out this struct with println!("{:x}", ifd);
impl fmt::LowerHex for IFD {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag = self.tag.to_le_bytes();
        let tag_type = self.tag_type.to_le_bytes();
        let count = self.count.to_le_bytes();
        let value_offset = self.value_offset.to_le_bytes();

        // :02 is format width, so we print '0f' instead of just 'f'
        let fields = format!("{:02x?}{:02x?}{:02x?}{:02x?}", tag, tag_type, count, value_offset);
        write!(f, "{}", fields)
    }
}

// Default formatter for our IFD struct.
// We implement this formatter so that we can print out this struct with println!("{}", ifd);
impl fmt::Display for IFD {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fields = format!("{}, {}, {}, {}", self.tag, self.tag_type, self.count, self.value_offset);
        write!(f, "{}", fields)
    }
}

// Methods on struct IFD.
impl IFD {
    // Basically a constructor (takes no `self` parameter, and returns a Self)
    // This wraps the From trait above. Technically we can just do this wherever we wanted to
    // generate an IFD, but I decided parameterizing from over from_offset was a nice to have.
    fn from_offset(buf: &Vec<u8>, offset: usize) -> Self {
        IFD::from(&buf[offset..offset+12])
    }

    // Takes buf, tiff header offset, since we may need to actually go get that value from some
    // other location (as designated from value_offset). "May", because according to the spec, if
    // the value of value_offset (latter 4 bytes in this slice) takes up less than or equal to the
    // 4 bytes for this field, the value itself will be inserted. We don't do that check here for
    // brevity.
    //
    // Also, we perform a panic catch here because we might be trying to read a random byte
    // offset as a utf8 string, and that offset might be expressed in the wrong endianness, and go
    // over the length of the byte buffer. I... guess this'd be a segfault in C? Traipsing off the
    // far end of a heap allocated byte buffer because your endianness was wrong? idk.
    fn print_value(&self, buf: &Vec<u8>, header_offset: usize) {
        match panic::catch_unwind(|| {
            print_offset_as_string(
                buf,
                header_offset + self.value_offset as usize,
                self.count as usize
            );
        }) {
            Ok(_) => {},
            Err(_) => {
                println!("Caught panic while printing value -- values may have been stored in other endianness.");
            }
        };
    }
}

// Print a random offset as bytes. There's no display trait for lower hex values for &[u8] (byte
// slices), so we wrap our slice in a tuple type, and then we impl fmt::LowerHex on that tuple
// type. We'd implement fmt::LowerHex right on &[u8], but slices are defined outside this crate.
// Not being able to arbitrarily extend the standard library in your crate is deliberate.
fn print_offset(buf: &Vec<u8>, offset: usize, length: usize) {
    println!("{:02x}", ByteSlice(buf[offset..offset+length].to_vec()));
}

// Try to utf8 parse a random byte offset. This can panic.
fn print_offset_as_string(buf: &Vec<u8>, offset: usize, length: usize) {
    match str::from_utf8(&buf[offset..offset+length]) {
        Ok(s) => println!("'{}'", s),
        Err(e) => println!("Error while printing range: {}", e)
    }
}

// The whole thing.
fn read_all(mut file: &File) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

// String pointer into characters, into char vector, into an iterator of 2 character pairs,
// which we concat, and radix parse into a 16 bit value. There's probably an easier way to
// accomplish this. 🤔
fn bytes_from_str(s: &str) -> Vec<u8> {
    s.chars().collect::<Vec<char>>().chunks(2).map(|chars| {
        let mut byte = String::new();
        byte.push(chars[0]);
        if chars.len() > 1 {
            byte.push(chars[1]);
        } else {
            byte.push('0');
        }
        u8::from_str_radix(&byte, 16).unwrap_or(0)
    }).collect()
}

// Find the offset of a byte sequence. Inelegantly implemented, since our unwrap default value must
// match the type of the closure parameters (usize, &[u]). The first parameter comes from
// enumerate(), which is a usize, the second comes from windows(...), which returns seq.len()
// slices. We ignore the index value in the actual closure because we don't need it.
fn find(buf: &Vec<u8>, seq: &Vec<u8>) -> usize {
    buf.windows(seq.len()).enumerate().find(|(_, bytes)| {
        // Here we're dealing with a window of size seq.len() of buf, matched against seq.
        // Windows are created above, so we simply zip bytes, and seq iterators together, and then
        // reduce each pair to a boolean. Unrolled: ((a[0] == b[0]) && (a[1] == b[1]) && (a[2] == b[2]) ...n)
        bytes.iter().zip(seq.iter()).fold(true, |acc, (a, b)| acc && a == b)
    }).unwrap_or((0, &[0])).0
}

fn main() -> io::Result<()> {
    // These files should be included in the repository. They come from: https://github.com/ianare/exif-samples/tree/master/jpg
    //let filename = "corrupted.jpg";
    //let filename = "a.jpg";
    //let filename = "Canon_40D.jpg";
    let filename = "Kodak_CX7530.jpg";
    println!("Reading file: {}", filename);
    match File::open(filename) { // succeeds
        Ok(file) => {
            // Lets get some file stats
            let md = file.metadata()?;
            // Lets keep our images small so we can load them into memory
            if md.len() > (1024 * 1000) {
                eprintln!("This bin wasn't designed to handle files over 1mb!");
                return Ok(())
            }
            // Read the whole thing. ? after function call here means unwrap result or returns err.
            let buf = read_all(&file)?;

            // Defining some constants and finding offsets. Ref: https://www.media.mit.edu/pia/Research/deepview/exif.html
            let tiff_header_marker = bytes_from_str("4949");
            let tiff_header_offset = find(&buf, &tiff_header_marker);

            let ifd_make_marker = bytes_from_str("0f01");
            let ifd_make_offset = find(&buf, &ifd_make_marker);

            // Create our IFD structure from our byte buffer and an ifd offset.
            let ifd_make_tag = IFD::from_offset(&buf, ifd_make_offset);

            // If we couldn't find either of our offsets, we probably can't continue.
            if tiff_header_offset == 0 || ifd_make_offset == 0 {
                println!("Unable to find apropriate offsets. Exif data either not present or adheres to some other format.");
            } else {
                // Print out the first 100 bytes for reference -- our tags should be in that range.
                println!("First 100 file bytes, wrapped to 10:");
                for step in (0..100).step_by(10) {
                    print!("{:2}: ", step);
                    print_offset(&buf, step, 10);
                }
                println!("");
                // Display tiff header offset (jpegs have tiff format headers for exif, who knew)
                println!("tiff offset: {}", tiff_header_offset);
                println!("ifd make offset: {}", ifd_make_offset);
                // Print out IFD structure in numerical values, and hex values
                println!("ifd make numerical values: {}", ifd_make_tag);
                println!("ifd make le byte values: {:x}", ifd_make_tag);
                // Print out make value
                print!("make tag value: ");
                ifd_make_tag.print_value(&buf, tiff_header_offset);
            }
        },
        // Couldn't open our file for some reason, so exit
        Err(e) => println!("An error occurred while trying to open file: {}", e)
    }
    // This is required because our main function definition returns an io::Result type (so that we
    // can use ? here and elsewhere as a shorthand, instead of full result blocks.
    Ok(())
}
