use crate::Vec;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PackedU32 {
    words: Vec<u64>,
    len: usize,
    bits: u8,
}

impl PackedU32 {
    pub(super) fn from_values(values: &[u32], bits: u8) -> Self {
        debug_assert!(bits <= 32);
        if bits == 0 || values.is_empty() {
            return Self {
                words: Vec::new(),
                len: values.len(),
                bits,
            };
        }
        let words_per_block = usize::from(bits);
        let block_count = values.len().div_ceil(64);
        let mut words = vec![0_u64; block_count * words_per_block];
        #[cfg(feature = "rayon")]
        if values.len() >= 262_144 {
            words
                .par_chunks_mut(words_per_block)
                .zip(values.par_chunks(64))
                .for_each(|(block, values)| fill_block(block, values, bits));
            return Self {
                words,
                len: values.len(),
                bits,
            };
        }
        for (index, &value) in values.iter().enumerate() {
            set(&mut words, words_per_block, index, u64::from(value), bits);
        }
        Self {
            words,
            len: values.len(),
            bits,
        }
    }

    #[inline]
    pub(super) fn get(&self, index: usize) -> u32 {
        debug_assert!(index < self.len);
        if self.bits == 0 {
            return 0;
        }
        let bits = usize::from(self.bits);
        let block = index / 64;
        let shift = (index % 64) * bits;
        let word = block * bits + shift / 64;
        let offset = shift % 64;
        let mut value = self.words[word] >> offset;
        if offset + bits > 64 {
            value |= self.words[word + 1] << (64 - offset);
        }
        u32::try_from(value & mask(self.bits)).unwrap_or(u32::MAX)
    }

    pub(super) fn storage_bytes(&self) -> usize {
        self.words.len() * size_of::<u64>()
    }

    #[inline]
    pub(super) fn for_each(&self, start: usize, end: usize, mut visit: impl FnMut(u32)) {
        debug_assert!(start <= end && end <= self.len);
        let bits = usize::from(self.bits);
        let mut index = start;
        while index < end {
            let block = index / 64;
            let block_end = end.min((block + 1) * 64);
            let words = &self.words[block * bits..(block + 1) * bits];
            let mut bit = (index % 64) * bits;
            while index < block_end {
                visit(read(words, bit, self.bits));
                bit += bits;
                index += 1;
            }
        }
    }
}

#[cfg(feature = "rayon")]
fn fill_block(words: &mut [u64], values: &[u32], bits: u8) {
    for (index, &value) in values.iter().enumerate() {
        set(words, usize::from(bits), index, u64::from(value), bits);
    }
}

fn set(words: &mut [u64], stride: usize, index: usize, value: u64, bits: u8) {
    let width = usize::from(bits);
    let block = index / 64;
    let shift = (index % 64) * width;
    let word = block * stride + shift / 64;
    let offset = shift % 64;
    let value = value & mask(bits);
    words[word] |= value << offset;
    if offset + width > 64 {
        words[word + 1] |= value >> (64 - offset);
    }
}

const fn mask(bits: u8) -> u64 {
    if bits == 0 {
        0
    } else if bits == 64 {
        u64::MAX
    } else {
        (1_u64 << bits) - 1
    }
}

#[inline]
fn read(words: &[u64], bit: usize, bits: u8) -> u32 {
    let width = usize::from(bits);
    let word = bit / 64;
    let offset = bit % 64;
    let mut value = words[word] >> offset;
    if offset + width > 64 {
        value |= words[word + 1] << (64 - offset);
    }
    u32::try_from(value & mask(bits)).unwrap_or(u32::MAX)
}
