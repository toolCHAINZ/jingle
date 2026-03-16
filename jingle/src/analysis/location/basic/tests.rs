use crate::{
    analysis::{
        cpa::{lattice::pcode::PcodeAddressLattice, state::AbstractState},
        location::basic::state::BasicLocationState,
    },
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
};

use super::*;
use jingle_sleigh::{PcodeOperation, VarNode};
use jingle_sleigh::varnode::IndirectVarNode;

#[test]
fn test_call_behavior_branch() {
    let state =
        BasicLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::Branch);

    let call_op = PcodeOperation::Call {
        dest: VarNode::new(0x2000, 8u32, 0u32),
        args: vec![],
        call_info: None,
    };

    let successors: Vec<_> = state.transfer(&call_op).into_iter().collect();
    assert_eq!(successors.len(), 1);
    assert_eq!(
        successors[0].inner(),
        &PcodeAddressLattice::Const(ConcretePcodeAddress::from(0x2000))
    );
}

#[test]
fn test_call_behavior_step_over() {
    let state =
        BasicLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::StepOver);

    let call_op = PcodeOperation::Call {
        dest: VarNode::new(0x2000, 8u32, 0u32),
        args: vec![],
        call_info: None,
    };

    let successors: Vec<_> = state.transfer(&call_op).into_iter().collect();
    assert_eq!(successors.len(), 1);
    // Should step over to next pcode address (machine: 0x1000, pcode: 1)
    let expected = ConcretePcodeAddress::from(0x1000).next_pcode();
    assert_eq!(successors[0].inner(), &PcodeAddressLattice::Const(expected));
}

#[test]
fn test_call_behavior_terminate() {
    let state =
        BasicLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::Terminate);

    let call_op = PcodeOperation::Call {
        dest: VarNode::new(0x2000, 8u32, 0u32),
        args: vec![],
        call_info: None,
    };

    let successors: Vec<_> = state.transfer(&call_op).into_iter().collect();
    assert_eq!(
        successors.len(),
        0,
        "Terminate should produce no successors"
    );
}

#[test]
fn test_callind_behavior_branch() {
    let state =
        BasicLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::Branch);

    let ptr = VarNode::new(0x2000, 8u32, 0u32);
    let ivn = IndirectVarNode::new(&ptr, 8u32, 0u32);
    let callind_op = PcodeOperation::CallInd { input: ivn.clone() };

    let successors: Vec<_> = state.transfer(&callind_op).into_iter().collect();
    assert_eq!(successors.len(), 1);
    // Branch falls through to inner lattice: produces Indirect(input)
    assert_eq!(successors[0].inner(), &PcodeAddressLattice::Indirect(ivn));
}

#[test]
fn test_callind_behavior_step_over() {
    let state =
        BasicLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::StepOver);

    let ptr = VarNode::new(0x2000, 8u32, 0u32);
    let callind_op = PcodeOperation::CallInd {
        input: IndirectVarNode::new(&ptr, 8u32, 0u32),
    };

    let successors: Vec<_> = state.transfer(&callind_op).into_iter().collect();
    assert_eq!(successors.len(), 1);
    let expected = ConcretePcodeAddress::from(0x1000).next_pcode();
    assert_eq!(successors[0].inner(), &PcodeAddressLattice::Const(expected));
}

#[test]
fn test_callind_behavior_terminate() {
    let state =
        BasicLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::Terminate);

    let ptr = VarNode::new(0x2000, 8u32, 0u32);
    let callind_op = PcodeOperation::CallInd {
        input: IndirectVarNode::new(&ptr, 8u32, 0u32),
    };

    let successors: Vec<_> = state.transfer(&callind_op).into_iter().collect();
    assert_eq!(
        successors.len(),
        0,
        "Terminate should produce no successors"
    );
}
