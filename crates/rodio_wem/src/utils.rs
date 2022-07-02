use bitvec::{
    field::BitField,
    macros::internal::funty::Integral,
    order::{BitOrder, Lsb0},
    prelude::BitVec,
    slice::BitSlice,
    store::BitStore,
    view::BitView,
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

/// Read bits from bit slice, write them to a bit vec and return the rest.
pub fn read_write<'a, 'b, I, T>(
    slice: &'b BitSlice<T, Lsb0>,
    out: &'a mut BitVec<T, Lsb0>,
    bits: usize,
) -> (&'b BitSlice<T, Lsb0>, I)
where
    I: Integral + BitStore,
    T: BitStore,
{
    let result: I = slice[..bits].load_le();

    write(result, out, bits);

    (&slice[bits..], result)
}

/// Write the number to the bitvec.
pub fn write<I, T>(value: I, out: &mut BitVec<T, Lsb0>, bits: usize)
where
    I: Integral + BitStore,
    T: BitStore,
{
    out.extend(&value.view_bits::<Lsb0>()[..bits]);
}

/// Read a single bool from the bit slice and return the rest.
pub fn read_bool<T>(slice: &BitSlice<T, Lsb0>) -> (&BitSlice<T, Lsb0>, bool)
where
    T: BitStore,
{
    let result: u8 = slice[..1].load_le();

    (&slice[1..], result != 0)
}

/// Read bool from bit slice, write them to a bit vec and return the rest.
pub fn read_write_bool<'a, 'b, T>(
    slice: &'b BitSlice<T, Lsb0>,
    out: &'a mut BitVec<T, Lsb0>,
) -> (&'b BitSlice<T, Lsb0>, bool)
where
    T: BitStore,
{
    let result: u8 = slice[..1].load_le();
    let result = result != 0;

    out.push(result);

    (&slice[1..], result)
}

/// Non-nightly `u32::log2'.
pub fn log2(mut value: u32) -> u32 {
    let mut ret = 0;
    while value != 0 {
        ret += 1;
        value >>= 1;
    }

    ret
}

#[cfg(test)]
mod tests {
    #[test]
    fn log2() {
        assert_eq!(super::log2(0), 0);
        assert_eq!(super::log2(1), 1);
        assert_eq!(super::log2(2), 2);
        assert_eq!(super::log2(3), 2);
        assert_eq!(super::log2(4), 3);
        assert_eq!(super::log2(7), 3);
    }
}
