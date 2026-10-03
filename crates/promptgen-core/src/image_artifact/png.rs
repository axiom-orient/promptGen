use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
pub const MAX_PNG_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_DECODED_BYTES: usize = 8_294_400 * 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PngInfo {
    pub width: u32,
    pub height: u32,
    pub decoded_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PngError(pub String);

impl fmt::Display for PngError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PngError {}

pub fn validate_file(path: &Path, expected: (u32, u32)) -> Result<PngInfo, PngError> {
    validate_file_with_expected(path, Some(expected))
}

pub fn inspect_file(path: &Path) -> Result<PngInfo, PngError> {
    validate_file_with_expected(path, None)
}

pub fn inspect_bytes(bytes: &[u8]) -> Result<PngInfo, PngError> {
    validate_bytes_with_expected(bytes, None)
}

fn validate_file_with_expected(
    path: &Path,
    expected: Option<(u32, u32)>,
) -> Result<PngInfo, PngError> {
    let file = File::open(path).map_err(|error| PngError(format!("open failed: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| PngError(format!("metadata failed: {error}")))?;
    if metadata.len() > MAX_PNG_FILE_BYTES as u64 {
        return Err(PngError(format!(
            "file exceeds limit: {} > {MAX_PNG_FILE_BYTES}",
            metadata.len()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_PNG_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| PngError(format!("read failed: {error}")))?;
    if bytes.len() > MAX_PNG_FILE_BYTES {
        return Err(PngError(format!(
            "file exceeds limit while reading: {} > {MAX_PNG_FILE_BYTES}",
            bytes.len()
        )));
    }
    validate_bytes_with_expected(&bytes, expected)
}

pub fn validate_bytes(bytes: &[u8], expected: (u32, u32)) -> Result<PngInfo, PngError> {
    validate_bytes_with_expected(bytes, Some(expected))
}

fn validate_bytes_with_expected(
    bytes: &[u8],
    expected: Option<(u32, u32)>,
) -> Result<PngInfo, PngError> {
    if bytes.len() < PNG_SIGNATURE.len() || &bytes[..8] != PNG_SIGNATURE {
        return Err(PngError("invalid PNG signature".to_owned()));
    }
    if bytes.len() > MAX_PNG_FILE_BYTES {
        return Err(PngError("PNG exceeds file-size limit".to_owned()));
    }

    let mut offset = 8_usize;
    let mut ihdr: Option<(u32, u32, u8, u8, u8)> = None;
    let mut idat = Vec::new();
    let mut seen_idat = false;
    let mut idat_ended = false;
    let mut seen_iend = false;
    let mut seen_plte = false;
    let mut chunk_index = 0_usize;

    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err(PngError(format!("truncated chunk header at byte {offset}")));
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"));
        let length =
            usize::try_from(length).map_err(|_| PngError("chunk length overflow".to_owned()))?;
        let chunk_type: [u8; 4] = bytes[offset + 4..offset + 8]
            .try_into()
            .expect("four bytes");
        let data_start = offset + 8;
        let data_end = data_start
            .checked_add(length)
            .ok_or_else(|| PngError("chunk length overflow".to_owned()))?;
        let crc_end = data_end
            .checked_add(4)
            .ok_or_else(|| PngError("chunk CRC offset overflow".to_owned()))?;
        if crc_end > bytes.len() {
            return Err(PngError(format!(
                "truncated {} chunk",
                chunk_name(chunk_type)
            )));
        }
        let expected_crc =
            u32::from_be_bytes(bytes[data_end..crc_end].try_into().expect("four bytes"));
        let actual_crc = crc32(&bytes[offset + 4..data_end]);
        if actual_crc != expected_crc {
            return Err(PngError(format!(
                "CRC mismatch in {} chunk: expected {expected_crc:08x}, actual {actual_crc:08x}",
                chunk_name(chunk_type)
            )));
        }
        let data = &bytes[data_start..data_end];

        match &chunk_type {
            b"IHDR" => {
                if chunk_index != 0 || ihdr.is_some() || length != 13 {
                    return Err(PngError(
                        "IHDR must be the first and unique 13-byte chunk".to_owned(),
                    ));
                }
                let width = u32::from_be_bytes(data[0..4].try_into().expect("four bytes"));
                let height = u32::from_be_bytes(data[4..8].try_into().expect("four bytes"));
                let bit_depth = data[8];
                let color_type = data[9];
                let compression = data[10];
                let filter = data[11];
                let interlace = data[12];
                if width == 0 || height == 0 {
                    return Err(PngError("PNG dimensions must be positive".to_owned()));
                }
                if let Some(expected) = expected
                    && (width, height) != expected
                {
                    return Err(PngError(format!(
                        "dimension mismatch: expected {}x{}, actual {width}x{height}",
                        expected.0, expected.1
                    )));
                }
                validate_color_mode(bit_depth, color_type)?;
                if compression != 0 || filter != 0 {
                    return Err(PngError(
                        "unsupported PNG compression or filter method".to_owned(),
                    ));
                }
                if interlace != 0 {
                    return Err(PngError(
                        "interlaced PNG is rejected because the adapter requires complete non-interlaced scanline verification"
                            .to_owned(),
                    ));
                }
                ihdr = Some((width, height, bit_depth, color_type, interlace));
            }
            b"PLTE" => {
                if ihdr.is_none()
                    || seen_idat
                    || seen_plte
                    || data.is_empty()
                    || !data.len().is_multiple_of(3)
                {
                    return Err(PngError("invalid PLTE placement or length".to_owned()));
                }
                if data.len() > 768 {
                    return Err(PngError("PLTE has more than 256 entries".to_owned()));
                }
                seen_plte = true;
            }
            b"IDAT" => {
                if ihdr.is_none() || idat_ended {
                    return Err(PngError(
                        "IDAT chunks must be consecutive and follow IHDR".to_owned(),
                    ));
                }
                seen_idat = true;
                idat.extend_from_slice(data);
                if idat.len() > MAX_PNG_FILE_BYTES {
                    return Err(PngError("combined IDAT data exceeds limit".to_owned()));
                }
            }
            b"IEND" => {
                if !seen_idat || seen_iend || length != 0 {
                    return Err(PngError(
                        "IEND must be unique, empty, and follow IDAT".to_owned(),
                    ));
                }
                seen_iend = true;
                if crc_end != bytes.len() {
                    return Err(PngError("trailing bytes after IEND".to_owned()));
                }
            }
            _ => {
                if seen_idat {
                    idat_ended = true;
                }
                if chunk_type[0].is_ascii_uppercase() {
                    return Err(PngError(format!(
                        "unknown critical chunk {}",
                        chunk_name(chunk_type)
                    )));
                }
            }
        }
        offset = crc_end;
        chunk_index += 1;
        if seen_iend {
            break;
        }
    }

    let Some((width, height, bit_depth, color_type, _)) = ihdr else {
        return Err(PngError("missing IHDR".to_owned()));
    };
    if color_type == 3 && !seen_plte {
        return Err(PngError("indexed-color PNG is missing PLTE".to_owned()));
    }
    if !seen_iend {
        return Err(PngError("missing IEND".to_owned()));
    }
    let row_bytes = row_bytes(width, bit_depth, color_type)?;
    let expected_decoded = (row_bytes + 1)
        .checked_mul(height as usize)
        .ok_or_else(|| PngError("decoded-size overflow".to_owned()))?;
    if expected_decoded > MAX_DECODED_BYTES {
        return Err(PngError(format!(
            "decoded scanlines exceed limit: {expected_decoded} > {MAX_DECODED_BYTES}"
        )));
    }
    let decoded = inflate_zlib(&idat, expected_decoded)?;
    if decoded.len() != expected_decoded {
        return Err(PngError(format!(
            "decoded scanline length mismatch: expected {expected_decoded}, actual {}",
            decoded.len()
        )));
    }
    for row in 0..height as usize {
        let filter = decoded[row * (row_bytes + 1)];
        if filter > 4 {
            return Err(PngError(format!(
                "invalid filter byte {filter} in row {row}"
            )));
        }
    }
    Ok(PngInfo {
        width,
        height,
        decoded_bytes: decoded.len(),
    })
}

fn validate_color_mode(bit_depth: u8, color_type: u8) -> Result<(), PngError> {
    let valid = match color_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        2 => matches!(bit_depth, 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        4 | 6 => matches!(bit_depth, 8 | 16),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(PngError(format!(
            "invalid bit-depth/color-type combination: bit_depth={bit_depth}, color_type={color_type}"
        )))
    }
}

fn row_bytes(width: u32, bit_depth: u8, color_type: u8) -> Result<usize, PngError> {
    let channels = match color_type {
        0 | 3 => 1_u64,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return Err(PngError("unsupported color type".to_owned())),
    };
    let bits = u64::from(width)
        .checked_mul(channels)
        .and_then(|value| value.checked_mul(u64::from(bit_depth)))
        .ok_or_else(|| PngError("row-size overflow".to_owned()))?;
    usize::try_from(bits.div_ceil(8)).map_err(|_| PngError("row-size overflow".to_owned()))
}

fn inflate_zlib(bytes: &[u8], expected_limit: usize) -> Result<Vec<u8>, PngError> {
    if bytes.len() < 6 {
        return Err(PngError("truncated zlib stream".to_owned()));
    }
    let cmf = bytes[0];
    let flg = bytes[1];
    if cmf & 0x0f != 8 || cmf >> 4 > 7 {
        return Err(PngError(
            "unsupported zlib compression method/window".to_owned(),
        ));
    }
    if (u16::from(cmf) << 8 | u16::from(flg)) % 31 != 0 {
        return Err(PngError("invalid zlib header checksum".to_owned()));
    }
    if flg & 0x20 != 0 {
        return Err(PngError(
            "zlib preset dictionaries are unsupported".to_owned(),
        ));
    }
    let checksum_offset = bytes.len() - 4;
    let expected_adler =
        u32::from_be_bytes(bytes[checksum_offset..].try_into().expect("four bytes"));
    let mut reader = BitReader::new(&bytes[2..checksum_offset]);
    let mut output = Vec::with_capacity(expected_limit.min(1024 * 1024));
    loop {
        let final_block = reader.read_bits(1)? != 0;
        let block_type = reader.read_bits(2)?;
        match block_type {
            0 => decode_stored(&mut reader, &mut output, expected_limit)?,
            1 => {
                let (literal, distance) = fixed_tables()?;
                decode_compressed(
                    &mut reader,
                    &literal,
                    &distance,
                    &mut output,
                    expected_limit,
                )?;
            }
            2 => {
                let (literal, distance) = dynamic_tables(&mut reader)?;
                decode_compressed(
                    &mut reader,
                    &literal,
                    &distance,
                    &mut output,
                    expected_limit,
                )?;
            }
            _ => return Err(PngError("reserved DEFLATE block type".to_owned())),
        }
        if final_block {
            break;
        }
    }
    reader.align_byte();
    if reader.byte_position() != reader.len() {
        return Err(PngError("trailing bytes inside DEFLATE payload".to_owned()));
    }
    if adler32(&output) != expected_adler {
        return Err(PngError("Adler-32 mismatch".to_owned()));
    }
    Ok(output)
}

fn decode_stored(
    reader: &mut BitReader<'_>,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), PngError> {
    reader.align_byte();
    let len = reader.read_u16_le()?;
    let nlen = reader.read_u16_le()?;
    if len != !nlen {
        return Err(PngError(
            "stored DEFLATE block length complement mismatch".to_owned(),
        ));
    }
    let bytes = reader.read_aligned_bytes(len as usize)?;
    if output
        .len()
        .checked_add(bytes.len())
        .is_none_or(|value| value > limit)
    {
        return Err(PngError(
            "decoded DEFLATE output exceeds expected limit".to_owned(),
        ));
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn fixed_tables() -> Result<(Huffman, Huffman), PngError> {
    let mut literal_lengths = vec![0_u8; 288];
    literal_lengths[..144].fill(8);
    literal_lengths[144..256].fill(9);
    literal_lengths[256..280].fill(7);
    literal_lengths[280..].fill(8);
    let distance_lengths = vec![5_u8; 32];
    Ok((
        Huffman::new(&literal_lengths)?,
        Huffman::new(&distance_lengths)?,
    ))
}

fn dynamic_tables(reader: &mut BitReader<'_>) -> Result<(Huffman, Huffman), PngError> {
    let literal_count = reader.read_bits(5)? as usize + 257;
    let distance_count = reader.read_bits(5)? as usize + 1;
    let code_count = reader.read_bits(4)? as usize + 4;
    let order = [
        16_usize, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_lengths = vec![0_u8; 19];
    for index in 0..code_count {
        code_lengths[order[index]] = reader.read_bits(3)? as u8;
    }
    let code_table = Huffman::new(&code_lengths)?;
    let total = literal_count + distance_count;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        match code_table.decode(reader)? {
            0..=15 => lengths.push(code_table.last_symbol() as u8),
            16 => {
                let previous = *lengths
                    .last()
                    .ok_or_else(|| PngError("repeat code 16 has no previous length".to_owned()))?;
                let count = reader.read_bits(2)? as usize + 3;
                extend_lengths(&mut lengths, previous, count, total)?;
            }
            17 => {
                let count = reader.read_bits(3)? as usize + 3;
                extend_lengths(&mut lengths, 0, count, total)?;
            }
            18 => {
                let count = reader.read_bits(7)? as usize + 11;
                extend_lengths(&mut lengths, 0, count, total)?;
            }
            symbol => return Err(PngError(format!("invalid code-length symbol {symbol}"))),
        }
    }
    if lengths[256] == 0 {
        return Err(PngError(
            "dynamic literal table has no end-of-block symbol".to_owned(),
        ));
    }
    let literal = Huffman::new(&lengths[..literal_count])?;
    let distance = Huffman::new(&lengths[literal_count..])?;
    Ok((literal, distance))
}

fn extend_lengths(
    lengths: &mut Vec<u8>,
    value: u8,
    count: usize,
    total: usize,
) -> Result<(), PngError> {
    if lengths
        .len()
        .checked_add(count)
        .is_none_or(|value| value > total)
    {
        return Err(PngError("code-length repeat exceeds table size".to_owned()));
    }
    lengths.extend(std::iter::repeat_n(value, count));
    Ok(())
}

fn decode_compressed(
    reader: &mut BitReader<'_>,
    literal: &Huffman,
    distance: &Huffman,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), PngError> {
    const LENGTH_BASE: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DISTANCE_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    loop {
        let symbol = literal.decode(reader)?;
        match symbol {
            0..=255 => {
                if output.len() >= limit {
                    return Err(PngError(
                        "decoded DEFLATE output exceeds expected limit".to_owned(),
                    ));
                }
                output.push(symbol as u8);
            }
            256 => return Ok(()),
            257..=285 => {
                let index = symbol as usize - 257;
                let length = LENGTH_BASE[index] + reader.read_bits(LENGTH_EXTRA[index])? as usize;
                let distance_symbol = distance.decode(reader)? as usize;
                if distance_symbol >= DISTANCE_BASE.len() {
                    return Err(PngError("reserved DEFLATE distance symbol".to_owned()));
                }
                let back = DISTANCE_BASE[distance_symbol]
                    + reader.read_bits(DISTANCE_EXTRA[distance_symbol])? as usize;
                if back == 0 || back > output.len() {
                    return Err(PngError(
                        "DEFLATE distance points before output start".to_owned(),
                    ));
                }
                if output
                    .len()
                    .checked_add(length)
                    .is_none_or(|value| value > limit)
                {
                    return Err(PngError(
                        "decoded DEFLATE output exceeds expected limit".to_owned(),
                    ));
                }
                for _ in 0..length {
                    let byte = output[output.len() - back];
                    output.push(byte);
                }
            }
            _ => {
                return Err(PngError(
                    "reserved DEFLATE literal/length symbol".to_owned(),
                ));
            }
        }
    }
}

/// Canonical Huffman decoding table.
///
/// Codes of one bit length are consecutive integers, so a decoder never has to
/// search: `counts` says how many codes each length holds and `symbols` lists them
/// in canonical order, which turns "which symbol is this code" into one subtraction
/// and one index. The previous table scanned every code of the current length for
/// each decoded symbol — up to 288 comparisons per symbol on a path that decodes
/// millions of symbols for a single full-size PNG.
struct Huffman {
    counts: Vec<u16>,
    symbols: Vec<u16>,
    max_len: u8,
    last: std::cell::Cell<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Self, PngError> {
        let max_len = lengths.iter().copied().max().unwrap_or(0);
        if max_len == 0 || max_len > 15 {
            return Err(PngError("empty or overlong Huffman table".to_owned()));
        }
        let mut counts = vec![0_u16; max_len as usize + 1];
        for &length in lengths {
            if length != 0 {
                counts[length as usize] += 1;
            }
        }
        let mut left = 1_i32;
        for &count in counts.iter().skip(1) {
            left = (left << 1) - i32::from(count);
            if left < 0 {
                return Err(PngError("oversubscribed Huffman table".to_owned()));
            }
        }
        // An incomplete table leaves valid-looking bit patterns undefined, so a corrupt
        // stream could decode to a symbol the encoder never wrote. DEFLATE permits
        // exactly one incomplete case — a single-symbol distance table — and nothing else.
        let assigned: u32 = counts.iter().skip(1).map(|count| u32::from(*count)).sum();
        if left > 0 && assigned != 1 {
            return Err(PngError("incomplete Huffman table".to_owned()));
        }
        // Offset of each length's first symbol inside the canonical symbol list.
        let mut offsets = vec![0_usize; max_len as usize + 2];
        for length in 1..=max_len as usize {
            offsets[length + 1] = offsets[length] + counts[length] as usize;
        }
        let mut symbols = vec![0_u16; offsets[max_len as usize + 1]];
        let mut cursor = offsets.clone();
        for (symbol, &length) in lengths.iter().enumerate() {
            if length == 0 {
                continue;
            }
            symbols[cursor[length as usize]] = symbol as u16;
            cursor[length as usize] += 1;
        }
        Ok(Self {
            counts,
            symbols,
            max_len,
            last: std::cell::Cell::new(0),
        })
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, PngError> {
        let mut code = 0_i32;
        let mut first = 0_i32;
        let mut index = 0_usize;
        for length in 1..=self.max_len as usize {
            code |= reader.read_bits(1)? as i32;
            let count = i32::from(self.counts[length]);
            if code - first < count {
                let symbol = self.symbols[index + (code - first) as usize];
                self.last.set(symbol);
                return Ok(symbol);
            }
            index += count as usize;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(PngError("invalid Huffman code".to_owned()))
    }

    fn last_symbol(&self) -> u16 {
        self.last.get()
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte: 0,
            bit: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, PngError> {
        if count > 16 {
            return Err(PngError("bit read exceeds 16 bits".to_owned()));
        }
        let mut value = 0_u32;
        for index in 0..count {
            if self.byte >= self.bytes.len() {
                return Err(PngError("truncated DEFLATE bitstream".to_owned()));
            }
            let bit = (self.bytes[self.byte] >> self.bit) & 1;
            value |= u32::from(bit) << index;
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.byte += 1;
            }
        }
        Ok(value)
    }

    fn align_byte(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.byte += 1;
        }
    }

    fn read_u16_le(&mut self) -> Result<u16, PngError> {
        let bytes = self.read_aligned_bytes(2)?;
        Ok(u16::from_le_bytes(bytes.try_into().expect("two bytes")))
    }

    fn read_aligned_bytes(&mut self, count: usize) -> Result<&'a [u8], PngError> {
        if self.bit != 0 {
            return Err(PngError("unaligned byte read".to_owned()));
        }
        let end = self
            .byte
            .checked_add(count)
            .ok_or_else(|| PngError("DEFLATE offset overflow".to_owned()))?;
        if end > self.bytes.len() {
            return Err(PngError("truncated DEFLATE byte sequence".to_owned()));
        }
        let bytes = &self.bytes[self.byte..end];
        self.byte = end;
        Ok(bytes)
    }

    const fn byte_position(&self) -> usize {
        self.byte
    }

    const fn len(&self) -> usize {
        self.bytes.len()
    }
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for chunk in bytes.chunks(5_552) {
        for byte in chunk {
            a += u32::from(*byte);
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn chunk_name(chunk_type: [u8; 4]) -> String {
    String::from_utf8_lossy(&chunk_type).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_complete_stored_block_png() {
        let png = make_png(2, 2);
        let info = validate_bytes(&png, (2, 2)).expect("valid PNG");
        assert_eq!(info.decoded_bytes, 18);
    }

    #[test]
    fn rejects_crc_and_dimension_mismatch() {
        let mut png = make_png(2, 2);
        png[20] ^= 1;
        assert!(validate_bytes(&png, (2, 2)).unwrap_err().0.contains("CRC"));
        let png = make_png(2, 2);
        assert!(
            validate_bytes(&png, (1, 1))
                .unwrap_err()
                .0
                .contains("dimension")
        );
    }

    /// The two fixtures cover one fixed and one dynamic block. The six shipped
    /// representative cards add production-encoder coverage.
    #[test]
    fn every_catalog_png_decodes_and_matches_its_declared_dimensions() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/assets");
        let mut checked = 0usize;
        let mut stack = vec![root];
        while let Some(directory) = stack.pop() {
            for entry in std::fs::read_dir(&directory).expect("catalog assets directory") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("png") {
                    continue;
                }
                let bytes = std::fs::read(&path).expect("read png");
                let declared = (
                    u32::from_be_bytes(bytes[16..20].try_into().expect("width")),
                    u32::from_be_bytes(bytes[20..24].try_into().expect("height")),
                );
                let info = validate_bytes(&bytes, declared)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                assert_eq!((info.width, info.height), declared, "{}", path.display());
                checked += 1;
            }
        }
        assert_eq!(checked, 6, "every published catalog card must be decoded");
    }

    #[test]
    fn incomplete_huffman_tables_are_rejected() {
        // Two two-bit codes leave half the code space undefined; only a single-symbol
        // table may be incomplete.
        assert!(Huffman::new(&[2, 2]).is_err());
        assert!(Huffman::new(&[1, 1]).is_ok());
        assert!(Huffman::new(&[1]).is_ok());
        assert!(Huffman::new(&[1, 1, 1]).is_err());
    }

    #[test]
    fn validates_fixed_and_dynamic_huffman_pngs() {
        let fixed = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../promptgen-codex/tests/fixtures/fixed-32x32.png"
        ));
        let dynamic = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../promptgen-codex/tests/fixtures/dynamic-32x32.png"
        ));
        assert_eq!(
            validate_bytes(fixed, (32, 32)).unwrap().decoded_bytes,
            4_128
        );
        assert_eq!(
            validate_bytes(dynamic, (32, 32)).unwrap().decoded_bytes,
            4_128
        );
    }

    #[test]
    fn rejects_corrupt_deflate_even_with_recomputed_chunk_crc() {
        let mut png = make_png(2, 2);
        let idat = locate_chunk(&png, b"IDAT");
        png[idat.0 + 7] ^= 0x40;
        let crc = crc32(&png[idat.0 - 4..idat.1]);
        png[idat.1..idat.1 + 4].copy_from_slice(&crc.to_be_bytes());
        assert!(validate_bytes(&png, (2, 2)).is_err());
    }

    fn make_png(width: u32, height: u32) -> Vec<u8> {
        let row_bytes = width as usize * 4;
        let mut raw = Vec::new();
        for row in 0..height as usize {
            raw.push(0);
            for column in 0..row_bytes {
                raw.push((row * row_bytes + column) as u8);
            }
        }
        let mut zlib = vec![0x78, 0x01, 0x01];
        let len = raw.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(&raw);
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        let mut png = PNG_SIGNATURE.to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        append_chunk(&mut png, b"IHDR", &ihdr);
        append_chunk(&mut png, b"IDAT", &zlib);
        append_chunk(&mut png, b"IEND", &[]);
        png
    }

    fn append_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = png.len();
        png.extend_from_slice(kind);
        png.extend_from_slice(data);
        let crc = crc32(&png[start..]);
        png.extend_from_slice(&crc.to_be_bytes());
    }

    fn locate_chunk(bytes: &[u8], kind: &[u8; 4]) -> (usize, usize) {
        let mut offset = 8;
        loop {
            let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            if &bytes[offset + 4..offset + 8] == kind {
                return (offset + 8, offset + 8 + length);
            }
            offset += 12 + length;
        }
    }
}
