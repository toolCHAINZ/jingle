use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use petgraph::visit::Walker;
use std::mem;
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
struct ProgramPathSegment(pub Vec<(ConcretePcodeAddress, PcodeOperation)>);

impl ProgramPathSegment {
    pub fn push(&mut self, address: ConcretePcodeAddress, op: PcodeOperation) {
        self.0.push((address, op));
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProgramPath {
    prefix: Vec<Rc<ProgramPathSegment>>,
    current: ProgramPathSegment,
}

impl ProgramPath {
    pub fn push(&mut self, address: ConcretePcodeAddress, op: PcodeOperation) {
        self.current.push(address, op);
    }

    pub fn commit(&mut self) {
        let old = mem::replace(&mut self.current, ProgramPathSegment::default());
        self.prefix.push(Rc::new(old));
    }

    pub fn iter(&self) -> impl Iterator<Item = &(ConcretePcodeAddress, PcodeOperation)> {
        self.prefix
            .iter()
            .flat_map(|p| p.0.iter())
            .chain(self.current.0.iter())
    }
}
