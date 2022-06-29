use bitvec::{
    field::BitField,
    macros::internal::funty::Integral,
    order::{BitOrder, Lsb0},
    slice::BitSlice,
    store::BitStore,
};

/// Read bits from bit slice and return the rest of the slice.
pub fn read<I, T>(slice: &BitSlice<T, Lsb0>, bits: usize) -> (&BitSlice<T, Lsb0>, I)
where
    I: Integral,
    T: BitStore,
{
    let result = slice[..bits].load_le();

    (&slice[bits..], result)
}

/// Read a single bool from the bit slice and return the rest.
pub fn read_bool<T>(slice: &BitSlice<T, Lsb0>) -> (&BitSlice<T, Lsb0>, bool)
where
    T: BitStore,
{
    let result: u8 = slice[..1].load_le();

    (&slice[1..], result != 0)
}
