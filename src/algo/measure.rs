use core::cmp::Ordering;

/// Ordered path cost with checked addition.
///
/// Implementations must return a consistent total order for valid values.
pub trait Measure: Copy {
    fn zero() -> Self;
    fn checked_add(self, other: Self) -> Option<Self>;
    fn compare(self, other: Self) -> Option<Ordering>;

    fn is_negative(self) -> bool {
        false
    }

    fn is_valid(self) -> bool {
        self.compare(self).is_some()
    }
}

macro_rules! unsigned_measure {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Measure for $type {
                fn zero() -> Self {
                    0
                }

                fn checked_add(self, other: Self) -> Option<Self> {
                    self.checked_add(other)
                }

                fn compare(self, other: Self) -> Option<Ordering> {
                    Some(self.cmp(&other))
                }
            }
        )+
    };
}

macro_rules! signed_measure {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Measure for $type {
                fn zero() -> Self {
                    0
                }

                fn checked_add(self, other: Self) -> Option<Self> {
                    self.checked_add(other)
                }

                fn compare(self, other: Self) -> Option<Ordering> {
                    Some(self.cmp(&other))
                }

                fn is_negative(self) -> bool {
                    self < 0
                }
            }
        )+
    };
}

macro_rules! float_measure {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Measure for $type {
                fn zero() -> Self {
                    0.0
                }

                fn checked_add(self, other: Self) -> Option<Self> {
                    let sum = self + other;
                    sum.is_finite().then_some(sum)
                }

                fn compare(self, other: Self) -> Option<Ordering> {
                    self.partial_cmp(&other)
                }

                fn is_negative(self) -> bool {
                    self < 0.0
                }

                fn is_valid(self) -> bool {
                    self.is_finite()
                }
            }
        )+
    };
}

unsigned_measure!(u8, u16, u32, u64, u128, usize);
signed_measure!(i8, i16, i32, i64, i128, isize);
float_measure!(f32, f64);
