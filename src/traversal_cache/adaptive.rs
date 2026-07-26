use crate::Vec;

const BLOCK_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdaptivePackedU32 {
    bases: Vec<u32>,
    word_offsets: Vec<u32>,
    widths: Vec<u8>,
    words: Vec<u64>,
    len: usize,
}

impl AdaptivePackedU32 {
    pub(super) fn estimated_storage_bytes(values: &[u32]) -> usize {
        let blocks = values.len().div_ceil(BLOCK_LEN);
        let words = values
            .chunks(BLOCK_LEN)
            .map(|block| words_for(block.len(), block_width(block)))
            .sum::<usize>();
        blocks * (size_of::<u32>() + size_of::<u32>() + size_of::<u8>())
            + size_of::<u32>()
            + words * size_of::<u64>()
    }

    pub(super) fn from_values(values: &[u32]) -> Self {
        let blocks = values.len().div_ceil(BLOCK_LEN);
        let mut bases = Vec::with_capacity(blocks);
        let mut word_offsets = Vec::with_capacity(blocks + 1);
        let mut widths = Vec::with_capacity(blocks);
        word_offsets.push(0);
        for block in values.chunks(BLOCK_LEN) {
            let (&minimum, &maximum) = (
                block.iter().min().expect("block is non-empty"),
                block.iter().max().expect("block is non-empty"),
            );
            let bits = value_bits(maximum - minimum);
            bases.push(minimum);
            widths.push(bits);
            let next = word_offsets.last().copied().unwrap_or(0)
                + u32::try_from(words_for(block.len(), bits)).expect("word count fits u32");
            word_offsets.push(next);
        }
        let mut words = vec![0_u64; word_offsets.last().copied().unwrap_or(0) as usize];
        for (block_index, values) in values.chunks(BLOCK_LEN).enumerate() {
            let start = word_offsets[block_index] as usize;
            let end = word_offsets[block_index + 1] as usize;
            fill(
                &mut words[start..end],
                values,
                bases[block_index],
                widths[block_index],
            );
        }
        Self {
            bases,
            word_offsets,
            widths,
            words,
            len: values.len(),
        }
    }

    #[inline]
    pub(super) fn get(&self, index: usize) -> u32 {
        debug_assert!(index < self.len);
        let block = index / BLOCK_LEN;
        let within = index % BLOCK_LEN;
        self.bases[block] + self.read(block, within)
    }

    #[inline]
    pub(super) fn for_each(&self, start: usize, end: usize, mut visit: impl FnMut(u32)) {
        debug_assert!(start <= end && end <= self.len);
        let mut index = start;
        while index < end {
            let block = index / BLOCK_LEN;
            let block_end = end.min((block + 1) * BLOCK_LEN);
            let base = self.bases[block];
            let mut within = index % BLOCK_LEN;
            while index < block_end {
                visit(base + self.read(block, within));
                within += 1;
                index += 1;
            }
        }
    }

    pub(super) fn storage_bytes(&self) -> usize {
        self.bases.len() * size_of::<u32>()
            + self.word_offsets.len() * size_of::<u32>()
            + self.widths.len() * size_of::<u8>()
            + self.words.len() * size_of::<u64>()
    }

    #[inline]
    fn read(&self, block: usize, within: usize) -> u32 {
        let bits = self.widths[block];
        if bits == 0 {
            return 0;
        }
        let words =
            &self.words[self.word_offsets[block] as usize..self.word_offsets[block + 1] as usize];
        read(words, within * usize::from(bits), bits)
    }
}

fn fill(words: &mut [u64], values: &[u32], base: u32, bits: u8) {
    for (index, &value) in values.iter().enumerate() {
        let bit = index * usize::from(bits);
        write(words, bit, u64::from(value - base), bits);
    }
}

fn write(words: &mut [u64], bit: usize, value: u64, bits: u8) {
    if bits == 0 {
        return;
    }
    let word = bit / 64;
    let offset = bit % 64;
    words[word] |= value << offset;
    if offset + usize::from(bits) > 64 {
        words[word + 1] |= value >> (64 - offset);
    }
}

fn read(words: &[u64], bit: usize, bits: u8) -> u32 {
    let word = bit / 64;
    let offset = bit % 64;
    let mut value = words[word] >> offset;
    if offset + usize::from(bits) > 64 {
        value |= words[word + 1] << (64 - offset);
    }
    u32::try_from(value & mask(bits)).expect("packed value is at most 32 bits")
}

fn block_width(values: &[u32]) -> u8 {
    let Some((&minimum, &maximum)) = values.iter().min().zip(values.iter().max()) else {
        return 0;
    };
    value_bits(maximum - minimum)
}

fn value_bits(maximum: u32) -> u8 {
    u8::try_from(u32::BITS - maximum.leading_zeros()).expect("u32 width fits u8")
}

fn words_for(values: usize, bits: u8) -> usize {
    (values * usize::from(bits)).div_ceil(64)
}

const fn mask(bits: u8) -> u64 {
    if bits == 0 { 0 } else { (1_u64 << bits) - 1 }
}
