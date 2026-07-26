use super::packed::PackedU32;
use crate::Vec;

const SAMPLE_RATE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EliasFano {
    low: PackedU32,
    high: Vec<u64>,
    samples: Vec<usize>,
    len: usize,
    low_bits: u8,
}

impl EliasFano {
    pub(super) fn from_monotone(values: &[u32]) -> Self {
        let len = values.len();
        let maximum = values.last().copied().unwrap_or(0) as usize;
        let low_bits = low_bits(maximum, len);
        let low_values = values
            .iter()
            .map(|value| low_part(*value, low_bits))
            .collect::<Vec<_>>();
        let upper_bound = (maximum >> low_bits) + len + 1;
        let mut high = vec![0_u64; upper_bound.div_ceil(64)];
        let mut samples = Vec::with_capacity(len.div_ceil(SAMPLE_RATE));
        for (index, &value) in values.iter().enumerate() {
            let position = (value as usize >> low_bits) + index;
            high[position / 64] |= 1_u64 << (position % 64);
            if index % SAMPLE_RATE == 0 {
                samples.push(position);
            }
        }
        let low_bits = u8::try_from(low_bits).expect("u32 width fits u8");
        Self {
            low: PackedU32::from_values(&low_values, low_bits),
            high,
            samples,
            len,
            low_bits,
        }
    }

    #[inline]
    pub(super) fn get(&self, index: usize) -> u32 {
        debug_assert!(index < self.len);
        let position = self.select(index);
        let upper = (position - index) as u64;
        u32::try_from((upper << self.low_bits) | u64::from(self.low.get(index)))
            .expect("decoded offset originated as u32")
    }

    pub(super) fn storage_bytes(&self) -> usize {
        self.low.storage_bytes()
            + self.high.len() * size_of::<u64>()
            + self.samples.len() * size_of::<usize>()
    }

    pub(super) const fn low_bits(&self) -> u8 {
        self.low_bits
    }

    fn select(&self, index: usize) -> usize {
        let sample_index = index / SAMPLE_RATE;
        let sample_item = sample_index * SAMPLE_RATE;
        let start = self.samples[sample_index];
        if index == sample_item {
            return start;
        }
        select_after(&self.high, start + 1, index - sample_item - 1)
    }
}

fn low_bits(maximum: usize, len: usize) -> usize {
    let ratio = maximum.checked_div(len.max(1)).unwrap_or(0);
    if ratio == 0 {
        0
    } else {
        usize::BITS as usize - 1 - ratio.leading_zeros() as usize
    }
}

fn low_part(value: u32, bits: usize) -> u32 {
    if bits == 0 {
        0
    } else {
        value & ((1_u32 << bits) - 1)
    }
}

fn select_after(words: &[u64], start: usize, mut remaining: usize) -> usize {
    let mut word_index = start / 64;
    let mut word = words[word_index] & (u64::MAX << (start % 64));
    loop {
        let ones = word.count_ones() as usize;
        if remaining < ones {
            return word_index * 64 + select_in_word(word, remaining);
        }
        remaining -= ones;
        word_index += 1;
        word = words[word_index];
    }
}

fn select_in_word(mut word: u64, rank: usize) -> usize {
    for _ in 0..rank {
        word &= word - 1;
    }
    word.trailing_zeros() as usize
}
