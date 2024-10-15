use crate::VarNode;

pub trait ImageProvider{
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize;

    fn has_range(&self, vn: &VarNode) -> bool;
}