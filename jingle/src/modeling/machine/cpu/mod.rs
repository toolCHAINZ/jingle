pub mod concrete;
mod concretization;
mod relations;
pub mod symbolic;

#[cfg(test)]
mod tests {
    use crate::modeling::machine::cpu::concrete::{ConcretePcodeAddress, PcodeOffset};
    use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
    use std::iter::empty;
    use z3::ast::BV;
    use z3::{Config, Context};

    #[test]
    fn address_round_trip() {
        let addr = ConcretePcodeAddress {
            machine: 0xdeadbeefcafebabe,
            pcode: 0x50,
        };
        let z3 = Context::new(&Config::new());
        let symbolized = addr.symbolize(&z3);
        let new_concrete: Vec<_> = symbolized.concretize_with_assertions(empty()).collect();
        assert_eq!(addr, new_concrete[0])
    }

    #[test]
    fn increment_pcode_addr() {
        let addr = ConcretePcodeAddress {
            machine: 0,
            pcode: 0,
        };
        let z3 = Context::new(&Config::new());
        let symbolized = addr.symbolize(&z3);
        let new_concrete: Vec<_> = symbolized.concretize_with_assertions(empty()).collect();
        assert_eq!(
            new_concrete[0],
            ConcretePcodeAddress {
                machine: 0,
                pcode: 0
            }
        );
        let plus_1 = symbolized.increment_pcode();
        let new_concrete: Vec<_> = plus_1.concretize_with_assertions(empty()).collect();
        assert_eq!(
            new_concrete[0],
            ConcretePcodeAddress {
                machine: 0,
                pcode: 1
            }
        );
        let symbolized = ConcretePcodeAddress {
            machine: 0,
            pcode: 0xff,
        }
        .symbolize(&z3);
        let plus_1 = symbolized.increment_pcode();
        let new_concrete: Vec<_> = plus_1.concretize_with_assertions(empty()).collect();
        assert_eq!(
            new_concrete[0],
            ConcretePcodeAddress {
                machine: 0,
                pcode: 0
            }
        );
    }

    #[test]
    fn create_symbolic_addr() {
        let z3 = Context::new(&Config::new());
        let addr = BV::from_u64(&z3, 0xdeadbeef, 64);
        let wrong = BV::from_u64(&z3, 0xdeadbeef, 65);

        let sym = SymbolicPcodeAddress::try_from_symbolic_dest(&z3, &addr).unwrap();
        let concrete: Vec<_> = sym.concretize_with_assertions(empty()).collect();
        assert_eq!(
            concrete[0],
            ConcretePcodeAddress {
                machine: 0xdeadbeef,
                pcode: 0
            }
        );

        let sym = SymbolicPcodeAddress::try_from_symbolic_dest(&z3, &wrong);
        assert!(sym.is_err());
    }

    #[test]
    fn test_relative_math() {
        let addr = ConcretePcodeAddress {
            machine: 4,
            pcode: 4,
        };
        let dec1 = addr.add_pcode_offset(-1i8 as PcodeOffset);
        let add1 = addr.add_pcode_offset(1i8 as PcodeOffset);
        let add255 = addr.add_pcode_offset(255);

        assert_eq!(
            dec1,
            ConcretePcodeAddress {
                machine: 4,
                pcode: 3
            }
        );
        assert_eq!(
            add1,
            ConcretePcodeAddress {
                machine: 4,
                pcode: 5
            }
        );
        assert_eq!(dec1, add255);
    }
}
