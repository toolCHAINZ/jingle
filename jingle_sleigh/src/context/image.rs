use crate::VarNode;
use object::ReadRef;
use std::ops::Range;

pub trait ImageProvider {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize;

    fn has_full_range(&self, vn: &VarNode) -> bool;
}

impl ImageProvider for &[u8] {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        //todo: check the space. Ignoring for now
        if vn.offset >= self.len() as u64 {
            output.fill(0);
            0
        } else {
            let vn_range: Range<usize> = Range::from(vn);
            let output_len = output.len();
            output.copy_from_slice(&self[vn_range.clone()]);
            output[vn_range.end..output_len].fill(0);
            vn_range.len()
        }
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        let vn_range: Range<usize> = Range::from(vn);
        vn_range.start > 0 && vn_range.start < self.len() && vn_range.end < self.len()
    }
}

impl ImageProvider for Vec<u8> {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        self.as_slice().load(vn, output)
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        self.as_slice().has_full_range(vn)
    }
}
