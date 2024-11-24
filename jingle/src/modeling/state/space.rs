use crate::JingleError::{MismatchedAddressSize, UnexpectedArraySort, ZeroSizedVarnode};
use crate::{JingleContext, JingleError};
use jingle_sleigh::{SleighEndianness, SpaceInfo};
use std::ops::Add;
use z3::ast::{Array, BV};
use z3::Sort;

/// SLEIGH models programs using many spaces. This struct serves as a helper for modeling a single
/// space. `jingle` uses an SMT Array sort to model a space.
///
/// `jingle` also maintains a separate Array holding "metadata" for the space. For right now, this
/// metadata has a single-bit bitvector as its word type, and it is only used for tracking whether
/// a given value originated from a CALLOTHER operation. This is necessary for distinguishing
/// between normal indirect jumps and some syscalls
#[derive(Clone, Debug)]
pub(crate) struct ModeledSpace<'ctx> {
    endianness: SleighEndianness,
    data: Array<'ctx>,
    #[allow(unused)]
    metadata: Array<'ctx>,
    space_info: SpaceInfo,
}

impl<'ctx> ModeledSpace<'ctx> {
    /// Create a new modeling space with the given z3 context, using the provided space metadata
    pub(crate) fn new(jingle: &JingleContext<'ctx>, space_info: &SpaceInfo) -> Self {
        let domain = Sort::bitvector(jingle.z3, space_info.index_size_bytes * 8);
        let range = Sort::bitvector(jingle.z3, space_info.word_size_bytes * 8);
        Self {
            endianness: space_info.endianness,
            data: Array::fresh_const(jingle.z3, &space_info.name, &domain, &range),
            metadata: Array::const_array(jingle.z3, &domain, &BV::from_u64(jingle.z3, 0, 1)),
            space_info: space_info.clone(),
        }
    }

    /// Get the z3 Array for this space
    pub(crate) fn get_space(&self) -> &Array<'ctx> {
        &self.data
    }
    /// Read [size_bytes] bytes of data from the given BV [offset], using the endianness
    /// of the space
    pub(crate) fn read_data(
        &self,
        offset: &BV<'ctx>,
        size_bytes: usize,
    ) -> Result<BV<'ctx>, JingleError> {
        if offset.get_size() != self.space_info.index_size_bytes * 8 {
            return Err(MismatchedAddressSize);
        }
        read_from_array(&self.data, offset, size_bytes, self.endianness)
    }

    /// Read [size_bytes] bytes worth of metadata from the given BV [offset], using the endianness
    /// of the space
    pub(crate) fn read_metadata(
        &self,
        offset: &BV<'ctx>,
        size_bytes: usize,
    ) -> Result<BV<'ctx>, JingleError> {
        if offset.get_size() != self.space_info.index_size_bytes * 8 {
            return Err(MismatchedAddressSize);
        }
        read_from_array(&self.metadata, offset, size_bytes, self.endianness)
    }

    /// Write the given bitvector of data to the given bitvector offset
    pub(crate) fn write_data(
        &mut self,
        val: &BV<'ctx>,
        offset: &BV<'ctx>,
    ) -> Result<(), JingleError> {
        if offset.get_size() != self.space_info.index_size_bytes * 8 {
            return Err(MismatchedAddressSize);
        }
        self.data = write_to_array::<8>(&self.data, val, offset, self.endianness);
        Ok(())
    }

    /// Write the given bitvector of metadata to the given bitvector offset
    pub(crate) fn write_metadata(
        &mut self,
        val: &BV<'ctx>,
        offset: &BV<'ctx>,
    ) -> Result<(), JingleError> {
        if offset.get_size() != self.space_info.index_size_bytes * 8 {
            return Err(MismatchedAddressSize);
        }
        self.metadata = write_to_array::<1>(&self.metadata, val, offset, self.endianness);
        Ok(())
    }
}

fn read_from_array<'ctx>(
    array: &Array<'ctx>,
    offset: &BV<'ctx>,
    size_bytes: usize,
    endianness: SleighEndianness,
) -> Result<BV<'ctx>, JingleError> {
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

fn write_to_array<'ctx, const W: u32>(
    array: &Array<'ctx>,
    val: &BV<'ctx>,
    offset: &BV<'ctx>,
    endianness: SleighEndianness,
) -> Array<'ctx> {
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
    use crate::modeling::state::space::ModeledSpace;
    use crate::tests::SLEIGH_ARCH;
    use crate::JingleContext;
    use jingle_sleigh::context::SleighContextBuilder;
    use jingle_sleigh::{SleighEndianness, SpaceInfo, SpaceType};
    use z3::ast::{Ast, BV};
    use z3::{Config, Context};

    fn make_space<'ctx>(
        z3: &JingleContext<'ctx>,
        endianness: SleighEndianness,
    ) -> ModeledSpace<'ctx> {
        let space_info = SpaceInfo {
            endianness,
            name: "ram".to_string(),
            word_size_bytes: 1,
            index_size_bytes: 4,
            index: 0,
            _type: SpaceType::IPTR_PROCESSOR,
        };
        ModeledSpace::new(z3, &space_info)
    }
    fn test_endian_write(e: SleighEndianness) {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let z3 = Context::new(&Config::new());
        let jingle = JingleContext::new(&z3, &sleigh);
        let mut space = make_space(&jingle, e);
        space
            .write_data(
                &BV::from_u64(&z3, 0xdead_beef, 32),
                &BV::from_u64(&z3, 0, 32),
            )
            .unwrap();
        let expected = match e {
            SleighEndianness::Big => [0xde, 0xad, 0xbe, 0xef],
            SleighEndianness::Little => [0xef, 0xbe, 0xad, 0xde],
        };
        for i in 0..4 {
            let data = space
                .read_data(&BV::from_u64(&z3, i, 32), 1)
                .unwrap()
                .simplify();
            assert!(data.is_const());
            assert_eq!(data.as_u64().unwrap(), expected[i as usize])
        }
    }

    fn test_endian_read(e: SleighEndianness) {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let z3 = Context::new(&Config::new());
        let jingle = JingleContext::new(&z3, &sleigh);
        let mut space = make_space(&jingle, e);
        let byte_layout = match e {
            SleighEndianness::Big => [0xde, 0xad, 0xbe, 0xef],
            SleighEndianness::Little => [0xef, 0xbe, 0xad, 0xde],
        };
        for i in 0..4 {
            space
                .write_data(
                    &BV::from_u64(&z3, byte_layout[i as usize], 8),
                    &BV::from_u64(&z3, i, 32),
                )
                .unwrap();
        }
        let val = space
            .read_data(&BV::from_u64(&z3, 0, 32), 4)
            .unwrap()
            .simplify();
        assert!(val.is_const());
        assert_eq!(val.as_u64().unwrap(), 0xdead_beef)
    }

    fn test_single_write(e: SleighEndianness) {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let z3 = Context::new(&Config::new());
        let jingle = JingleContext::new(&z3, &sleigh);
        let mut space = make_space(&jingle, e);
        space
            .write_data(&BV::from_u64(&z3, 0x42, 8), &BV::from_u64(&z3, 0, 32))
            .unwrap();
        let expected = 0x42;
        let data = space
            .read_data(&BV::from_u64(&z3, 0, 32), 1)
            .unwrap()
            .simplify();
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
