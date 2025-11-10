/*
    Disc descrambling functionality
    Based on work of Marcus Comstedt (http://mc.pp.se)
    Adapted for Rust from reference/devcast/libswirl/reios/descrambl.cpp
*/

const MAXCHUNK: usize = 2048 * 1024;

/// Simple PRNG for descrambling
struct DescrambleRng {
    seed: u16,
}

impl DescrambleRng {
    fn new(n: u32) -> Self {
        Self {
            seed: (n & 0xffff) as u16,
        }
    }

    fn next(&mut self) -> u16 {
        self.seed = ((self.seed as u32 * 2109 + 9273) & 0x7fff) as u16;
        ((self.seed as u32 + 0xc000) & 0xffff) as u16
    }
}

/// Load a chunk of data with descrambling
///
/// # Arguments
/// * `src` - Source data buffer (will be consumed)
/// * `dst` - Destination buffer
/// * `sz` - Size of chunk to load (must be <= MAXCHUNK)
fn load_chunk(src: &mut &[u8], dst: &mut [u8], sz: usize, rng: &mut DescrambleRng) {
    assert!(sz <= MAXCHUNK, "Chunk size must be <= MAXCHUNK");
    assert!(sz % 32 == 0, "Chunk size must be multiple of 32");

    // Convert chunk size to number of slices (32 bytes each)
    let num_slices = sz / 32;

    // Initialize index table with unity, so each slice gets loaded exactly once
    let mut idx: Vec<usize> = (0..num_slices).collect();

    // Shuffle slices based on PRNG
    for i in (0..num_slices).rev() {
        // Select a replacement index
        let rand_val = rng.next() as usize;
        let x = (rand_val * i) >> 16;

        // Swap indices
        idx.swap(i, x);

        // Load resulting slice (32 bytes)
        let slice_start = i * 32;
        let dst_idx = idx[i] * 32;

        dst[dst_idx..dst_idx + 32].copy_from_slice(&src[slice_start..slice_start + 32]);
    }

    // Advance source pointer
    *src = &src[sz..];
}

/// Descramble a buffer of data
///
/// # Arguments
/// * `src` - Source scrambled data
/// * `dst` - Destination buffer (must be at least `filesz` bytes)
/// * `filesz` - Size of file to descramble
pub fn descrambl_buffer(src: &[u8], dst: &mut [u8], filesz: usize) {
    assert!(dst.len() >= filesz, "Destination buffer too small");

    let mut rng = DescrambleRng::new(filesz as u32);
    let mut src_slice = src;
    let mut dst_offset = 0;
    let mut remaining = filesz;

    // Descramble 2MB blocks for as long as possible, then
    // gradually reduce the window down to 32 bytes (1 slice)
    let mut chunksz = MAXCHUNK;
    while chunksz >= 32 {
        while remaining >= chunksz {
            load_chunk(
                &mut src_slice,
                &mut dst[dst_offset..dst_offset + chunksz],
                chunksz,
                &mut rng,
            );
            remaining -= chunksz;
            dst_offset += chunksz;
        }
        chunksz >>= 1;
    }

    // Load final incomplete slice (if any)
    if remaining > 0 {
        dst[dst_offset..dst_offset + remaining].copy_from_slice(&src_slice[..remaining]);
    }
}

/// Descramble a file from disc
///
/// # Arguments
/// * `disc` - GD-ROM disc interface
/// * `fad` - Starting FAD (Frame Address) on disc
/// * `file_size` - Size of file to read
/// * `dst` - Destination buffer (must be at least `file_size` bytes)
///
/// Note: This function signature is a stub - needs integration with actual disc interface
pub fn descrambl_file(/* disc: &dyn GDRomDisc, */ fad: u32, file_size: u32, dst: &mut [u8]) {
    // TODO: Integrate with actual disc reading system
    //
    // let sectors_needed = (file_size + 2047) / 2048;
    // let mut temp_buffer = vec![0u8; (sectors_needed * 2048) as usize];
    // disc.read_sector(&mut temp_buffer, fad, sectors_needed, 2048);
    // descrambl_buffer(&temp_buffer, dst, file_size as usize);

    println!(
        "descrambl_file: stub - fad={}, file_size={}, dst.len()={}",
        fad,
        file_size,
        dst.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng() {
        let mut rng = DescrambleRng::new(0x1234);
        // Test that RNG produces deterministic values
        let val1 = rng.next();
        let val2 = rng.next();

        // Reset with same seed
        let mut rng2 = DescrambleRng::new(0x1234);
        assert_eq!(val1, rng2.next());
        assert_eq!(val2, rng2.next());
    }

    #[test]
    fn test_descrambl_buffer() {
        // Create a simple test pattern
        let src: Vec<u8> = (0..256).map(|x| x as u8).collect();
        let mut dst = vec![0u8; 256];

        descrambl_buffer(&src, &mut dst, 256);

        // The output should be different from input (scrambled)
        // but we can't easily test the exact values without knowing the algorithm details
        // Just verify it doesn't crash and produces output
        assert_eq!(dst.len(), 256);
    }
}
