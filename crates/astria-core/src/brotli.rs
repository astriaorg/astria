use std::io::Write as _;

use brotli::{
    enc::BrotliEncoderParams,
    CompressorWriter,
    DecompressorWriter,
};

const BROTLI_BUFFER_SIZE: usize = 4096;

/// Decompresses the given bytes using the Brotli algorithm.
///
/// Returns the decompressed bytes.
/// 
/// # Errors
/// 
/// Returns an error if the decompression fails.
pub fn decompress_bytes(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut output = Vec::with_capacity(BROTLI_BUFFER_SIZE);
    {
        let mut decompressor = DecompressorWriter::new(&mut output, BROTLI_BUFFER_SIZE);
        decompressor.write_all(data)?;
    }

    Ok(output)
}

/// Compresses the given bytes using the Brotli algorithm at setting 5.
///
/// Returns the compressed bytes.
///
/// # Errors
///
/// Returns an error if the compression fails.
pub fn compress_bytes(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let compression_params = BrotliEncoderParams {
        quality: 5,
        size_hint: data.len(),
        ..Default::default()
    };
    let mut output = Vec::with_capacity(BROTLI_BUFFER_SIZE);
    {
        let mut compressor =
            CompressorWriter::with_params(&mut output, BROTLI_BUFFER_SIZE, &compression_params);
        compressor.write_all(data)?;
    }

    Ok(output)
}
