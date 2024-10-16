use crate::VarNode;
use std::cmp::min;
use std::ops::Range;

pub trait ImageProvider {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize;

    fn has_full_range(&self, vn: &VarNode) -> bool;
}

impl ImageProvider for &[u8] {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        //todo: check the space. Ignoring for now
        let vn_range: Range<usize> = Range::from(vn);
        let vn_range = Range {
            start: vn_range.start,
            end: min(vn_range.end, self.len()),
        };
        if let Some(s) = self.get(vn_range) {
            if let Some(mut o) = output.get_mut(0..s.len()) {
                o.copy_from_slice(s)
            }
            let o_len = output.len();
            if let Some(o) = output.get_mut(s.len()..o_len) {
                o.fill(0);
            }
            return s.len();
        } else {
            output.fill(0);
            0
        }
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        let vn_range: Range<usize> = Range::from(vn);
        vn_range.start >= 0 && vn_range.start < self.len() && vn_range.end <= self.len()
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
