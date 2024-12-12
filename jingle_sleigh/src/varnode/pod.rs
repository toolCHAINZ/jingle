use crate::context::SleighContext;
use crate::VarNode;

/// A "plain-old-data" version of a [VarNode], suitable for passing between threads
/// and serialization/deserialization tasks.
pub struct PodVarNode{
    space_index: usize,
    size: usize,
    offset: u64,
}

impl From<&VarNode> for PodVarNode {
    fn from(value: &VarNode) -> Self {
        Self{
            size: value.size,
            offset: value.offset,
            space_index: value.space.index
        }
    }
}

impl From<VarNode> for PodVarNode {
    fn from(value: VarNode) -> Self {
        (&value).into()
    }
}

impl PodVarNode{
    pub fn to_varnode(self, ctx: &SleighContext) -> VarNode{
        todo!("Hmmm, might need to think this through more")
    }
}