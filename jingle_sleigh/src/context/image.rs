use crate::VarNode;

pub trait ImageProvider{
    fn load(vn: &VarNode, output: &mut &[u8]) -> usize;
}