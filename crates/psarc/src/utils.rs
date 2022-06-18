use nom::error::ParseError;
use nom::error::{make_error, ErrorKind};
use nom::lib::std::ops::RangeFrom;
use nom::{Err, IResult, InputIter, InputLength, Slice};

/// Nom number for 5 bytes.
#[inline]
pub fn be_u40<I, E: ParseError<I>>(input: I) -> IResult<I, u64, E>
where
    I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
    let bound: usize = 5;
    if input.input_len() < bound {
        Err(Err::Error(make_error(input, ErrorKind::Eof)))
    } else {
        let mut res = 0u64;
        for byte in input.iter_elements().take(bound) {
            res = (res << 8) + byte as u64;
        }

        Ok((input.slice(bound..), res))
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::be_u40;

    macro_rules! assert_parse(
    ($left: expr, $right: expr) => {
      let res: nom::IResult<_, _, (_, nom::error::ErrorKind)> = $left;
      assert_eq!(res, $right);
    };
  );
    #[test]
    fn test_be_u40() {
        assert_parse!(
            be_u40(&b"\x00\x03\x05\x07\x09abc"[..]),
            Ok((&b"abc"[..], 0x0003050709))
        );
    }
}
