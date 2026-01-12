use crate::JingleError;
use crate::JingleError::{MismatchedAddressSize, UnexpectedArraySort, ZeroSizedVarnode};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{SleighEndianness, SpaceInfo, SpaceType};
use std::borrow::Borrow;
use std::ops::Add;
use z3::ast::{Array, Ast, BV, Bool};
use z3::Sort;

/// SLEIGH models programs using many spaces. This struct serves as a helper for modeling a single
/// space. `jingle` uses an SMT Array sort to model a space.
///
/// `jingle` also maintains a separate Array holding "metadata" for the space. For right now, this
/// metadata has a single-bit bitvector as its word type, and it is only used for tracking whether
/// a given value originated from a CALLOTHER operation. This is necessary for distinguishing
/// between normal indirect jumps and some syscalls
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BMCModeledSpace {
    data: Array,
    word_size_bytes: u32,
    index_size_bytes: u32,
    endianness: SleighEndianness,
    _type: SpaceType,
}

impl BMCModeledSpace {
    /// Create a new modeling space with the given z3 context, using the provided space metadata
    pub fn fresh_const(space_info: &SpaceInfo) -> Self {
        let domain = Sort::bitvector(space_info.index_size_bytes * 8);
        let range = Sort::bitvector(space_info.word_size_bytes * 8);
        Self {
            endianness: space_info.endianness,
            data: Array::fresh_const(&space_info.name, &domain, &range),
            word_size_bytes: space_info.word_size_bytes,
            index_size_bytes: space_info.index_size_bytes,
            _type: space_info._type,
        }
    }

    /// Create a new modeling space with the given z3 context, using the provided space metadata
    pub fn new_const<T: AsRef<str>>(name: T, space_info: &SpaceInfo) -> Self {
        let domain = Sort::bitvector(space_info.index_size_bytes * 8);
        let range = Sort::bitvector(space_info.word_size_bytes * 8);
        Self {
            endianness: space_info.endianness,
            data: Array::new_const(
                format!("{}_{}", name.as_ref(), &space_info.name),
                &domain,
                &range,
            ),
            word_size_bytes: space_info.word_size_bytes,
            index_size_bytes: space_info.index_size_bytes,
            _type: space_info._type,
        }
    }

    pub fn new_for_address<T: Borrow<ConcretePcodeAddress>>(
        space_info: &SpaceInfo,
        addr: T,
    ) -> Self {
        let addr = addr.borrow();
        let domain = Sort::bitvector(space_info.index_size_bytes * 8);
        let range = Sort::bitvector(space_info.word_size_bytes * 8);
        Self {
            endianness: space_info.endianness,
            data: Array::fresh_const(
                &format!("{}_{:x}_{:x}", &space_info.name, addr.machine, addr.pcode),
                &domain,
                &range,
            ),
            word_size_bytes: space_info.word_size_bytes,
            index_size_bytes: space_info.index_size_bytes,
            _type: space_info._type,
        }
    }

    pub fn get_type(&self) -> SpaceType {
        self._type
    }
    /// Get the z3 Array for this space
    pub fn get_space(&self) -> &Array {
        &self.data
    }
    /// Read `size_bytes` bytes of data from the given BV `offset`, using the endianness
    /// of the space
    pub fn read(&self, offset: &BV, size_bytes: usize) -> Result<BV, JingleError> {
        if offset.get_size() != self.index_size_bytes * 8 {
            return Err(MismatchedAddressSize);
        }
        read_from_array(&self.data, offset, size_bytes, self.endianness)
    }

    /// Write the given bitvector of data to the given bitvector offset
    pub fn write(&mut self, val: &BV, offset: &BV) -> Result<(), JingleError> {
        if offset.get_size() != self.index_size_bytes * 8 {
            return Err(MismatchedAddressSize);
        }
        self.data = write_to_array::<8>(&self.data, val, offset, self.endianness);
        Ok(())
    }

    /// Symbolically equate two spaces. Note that these spaces MUST have the same dimensions.
    pub fn _eq(&self, other: &Self) -> Bool {
        self.data.eq(&other.data)
    }

    pub fn _meta_eq(&self, other: &Self) -> bool {
        self.word_size_bytes == other.word_size_bytes
            && self.endianness == other.endianness
            && self.index_size_bytes == other.index_size_bytes
            && self._type == other._type
    }

    pub fn simplify(&self) -> Self {
        Self {
            data: self.data.simplify(),
            word_size_bytes: self.word_size_bytes,
            index_size_bytes: self.index_size_bytes,
            endianness: self.endianness,
            _type: self._type,
        }
    }
}

fn read_from_array(
    array: &Array,
    offset: &BV,
    size_bytes: usize,
    endianness: SleighEndianness,
) -> Result<BV, JingleError> {
    // concat left hand is most significant
    (0..size_bytes)
        .map(|i| {
            array
                .select(&offset.clone().add(i as u64))
                .as_bv()
                .ok_or(UnexpectedArraySort)
        })
        .reduce(|acc, byte_bv| match endianness {
            SleighEndianness::Big => Ok(acc?.concat(&byte_bv?)),
            SleighEndianness::Little => Ok(byte_bv?.concat(&acc?)),
        })
        .ok_or(ZeroSizedVarnode)?
}

fn write_to_array<const W: u32>(
    array: &Array,
    val: &BV,
    offset: &BV,
    endianness: SleighEndianness,
) -> Array {
    let mut scratch = array.clone();
    let size = val.get_size();
    for i in 0..size / W {
        let (high, low) = match endianness {
            SleighEndianness::Big => (size - W * i - 1, size - W * (i + 1)),
            SleighEndianness::Little => (W * (i + 1) - 1, W * i),
        };
        let ext = &val.extract(high, low);
        scratch = scratch.store(&offset.add(i as u64), ext);
    }
    scratch
}

#[cfg(test)]
mod tests {
    use crate::modeling::machine::memory::space::BMCModeledSpace;
    use jingle_sleigh::{SleighEndianness, SpaceInfo, SpaceType};
    use z3::ast::{Ast, BV};

    fn make_space(endianness: SleighEndianness) -> BMCModeledSpace {
        let space_info = SpaceInfo {
            endianness,
            name: "ram".to_string(),
            word_size_bytes: 1,
            index_size_bytes: 4,
            index: 0,
            _type: SpaceType::IPTR_PROCESSOR,
        };
        BMCModeledSpace::fresh_const(&space_info)
    }

    fn test_endian_write(e: SleighEndianness) {
        let mut space = make_space(e);
        space
            .write(&BV::from_u64(0xdead_beef, 32), &BV::from_u64(0, 32))
            .unwrap();
        let expected = match e {
            SleighEndianness::Big => [0xde, 0xad, 0xbe, 0xef],
            SleighEndianness::Little => [0xef, 0xbe, 0xad, 0xde],
        };
        for i in 0..4 {
            let data = space.read(&BV::from_u64(i, 32), 1).unwrap().simplify();
            assert!(data.is_const());
            assert_eq!(data.as_u64().unwrap(), expected[i as usize])
        }
    }

    fn test_endian_read(e: SleighEndianness) {
        let mut space = make_space(e);
        let byte_layout = match e {
            SleighEndianness::Big => [0xde, 0xad, 0xbe, 0xef],
            SleighEndianness::Little => [0xef, 0xbe, 0xad, 0xde],
        };
        for i in 0..4 {
            space
                .write(
                    &BV::from_u64(byte_layout[i as usize], 8),
                    &BV::from_u64(i, 32),
                )
                .unwrap();
        }
        let val = space.read(&BV::from_u64(0, 32), 4).unwrap().simplify();
        assert!(val.is_const());
        assert_eq!(val.as_u64().unwrap(), 0xdead_beef)
    }

    fn test_single_write(e: SleighEndianness) {
        let mut space = make_space(e);
        space
            .write(&BV::from_u64(0x42, 8), &BV::from_u64(0, 32))
            .unwrap();
        let expected = 0x42;
        let data = space.read(&BV::from_u64(0, 32), 1).unwrap().simplify();
        assert!(data.is_const());
        assert_eq!(data.as_u64().unwrap(), expected)
    }

    #[test]
    fn test_single_little_endian_write() {
        test_single_write(SleighEndianness::Little)
    }

    #[test]
    fn test_single_big_endian_write() {
        test_single_write(SleighEndianness::Big)
    }

    #[test]
    fn test_little_endian_write() {
        test_endian_write(SleighEndianness::Little)
    }

    #[test]
    fn test_big_endian_write() {
        test_endian_write(SleighEndianness::Big)
    }

    #[test]
    fn test_little_endian_read() {
        test_endian_read(SleighEndianness::Little)
    }

    #[test]
    fn test_big_endian_read() {
        test_endian_read(SleighEndianness::Big)
    }
}
