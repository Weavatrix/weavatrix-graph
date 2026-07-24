use serde::{Deserialize, Serialize};

macro_rules! stable_key {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name {
            slot: u32,
            generation: u32,
        }

        impl $name {
            pub(crate) const fn new(slot: u32, generation: u32) -> Self {
                Self { slot, generation }
            }

            #[must_use]
            pub const fn slot(self) -> u32 {
                self.slot
            }

            #[must_use]
            pub const fn generation(self) -> u32 {
                self.generation
            }

            #[must_use]
            pub const fn index(self) -> usize {
                self.slot() as usize
            }
        }
    };
}

stable_key!(StableNodeKey);
stable_key!(StableEdgeKey);
