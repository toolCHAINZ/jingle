pub use bridge::OpCode;

#[cxx::bridge]
pub(crate) mod bridge {
    #[namespace = "ghidra"]
    #[derive(Debug, Copy, Clone, Hash)]
    #[repr(u32)]
    pub(super) enum OpCode {
        /// Copy one operand to another
        CPUI_COPY = 1,
        ///Load from a pointer into a specified address space
        CPUI_LOAD = 2,
        /// Store at a pointer into a specified address space
        CPUI_STORE = 3,
        /// Always branch
        CPUI_BRANCH = 4,
        /// Conditional branch
        CPUI_CBRANCH = 5,
        /// Indirect branch (jumptable)
        CPUI_BRANCHIND = 6,
        /// Call to an absolute address
        CPUI_CALL = 7,
        /// Call through an indirect address
        CPUI_CALLIND = 8,
        /// User-defined operation
        CPUI_CALLOTHER = 9,
        /// Return from subroutine
        CPUI_RETURN = 10,
        /// Integer comparison, equality (==)
        CPUI_INT_EQUAL = 11,
        /// Integer comparison, in-equality (!=)
        CPUI_INT_NOTEQUAL = 12,
        /// Integer comparison, signed less-than (<)
        CPUI_INT_SLESS = 13,
        /// Integer comparison, signed less-than-or-equal (<=)
        CPUI_INT_SLESSEQUAL = 14,
        /// Integer comparison, unsigned less-than (<)
        CPUI_INT_LESS = 15,
        /// Integer comparison, unsigned less-than-or-equal (<=)
        /// This also indicates a borrow on unsigned subtraction
        CPUI_INT_LESSEQUAL = 16,
        /// Zero extension
        CPUI_INT_ZEXT = 17,
        /// Sign extension
        CPUI_INT_SEXT = 18,
        /// Addition, signed or unsigned (+)
        CPUI_INT_ADD = 19,
        /// Subtraction, signed or unsigned (-)
        CPUI_INT_SUB = 20,
        /// Test for unsigned carry
        CPUI_INT_CARRY = 21,
        /// Test for signed carry
        CPUI_INT_SCARRY = 22,
        /// Test for signed borrow
        CPUI_INT_SBORROW = 23,
        /// Twos complement
        CPUI_INT_2COMP = 24,
        /// Logical/bitwise negation (~)
        CPUI_INT_NEGATE = 25,
        /// Logical/bitwise exclusive-or (^)
        CPUI_INT_XOR = 26,
        /// Logical/bitwise and (&)
        CPUI_INT_AND = 27,
        /// Logical/bitwise or (|)
        CPUI_INT_OR = 28,
        /// Left shift (<<)
        CPUI_INT_LEFT = 29,
        /// Right shift, logical (>>)
        CPUI_INT_RIGHT = 30,
        /// Right shift, arithmetic (>>)
        CPUI_INT_SRIGHT = 31,
        /// Integer multiplication, signed and unsigned (*)
        CPUI_INT_MULT = 32,
        /// Integer division, unsigned (/)
        CPUI_INT_DIV = 33,
        /// Integer division, signed (/)
        CPUI_INT_SDIV = 34,
        /// Remainder/modulo, unsigned (%)
        CPUI_INT_REM = 35,
        /// Remainder/modulo, signed (%)
        CPUI_INT_SREM = 36,
        /// Boolean negate (!)
        CPUI_BOOL_NEGATE = 37,
        /// Boolean exclusive-or (^^)
        CPUI_BOOL_XOR = 38,
        /// Boolean and (&&)
        CPUI_BOOL_AND = 39,
        /// Boolean or (||)
        CPUI_BOOL_OR = 40,
        /// Floating-point comparison, equality (==)
        CPUI_FLOAT_EQUAL = 41,
        /// Floating-point comparison, in-equality (!=)
        CPUI_FLOAT_NOTEQUAL = 42,
        /// Floating-point comparison, less-than (<)
        CPUI_FLOAT_LESS = 43,
        /// Floating-point comparison, less-than-or-equal (<=)
        CPUI_FLOAT_LESSEQUAL = 44,
        /// Not-a-number test (NaN)
        CPUI_FLOAT_NAN = 46,
        /// Floating-point addition (+)
        CPUI_FLOAT_ADD = 47,
        /// Floating-point division (/)
        CPUI_FLOAT_DIV = 48,
        /// Floating-point multiplication (*)
        CPUI_FLOAT_MULT = 49,
        /// Floating-point subtraction (-)
        CPUI_FLOAT_SUB = 50,
        /// Floating-point negation (-)
        CPUI_FLOAT_NEG = 51,
        /// Floating-point absolute value (abs)
        CPUI_FLOAT_ABS = 52,
        /// Floating-point square root (sqrt)
        CPUI_FLOAT_SQRT = 53,
        /// Convert an integer to a floating-point
        CPUI_FLOAT_INT2FLOAT = 54,
        /// Convert between different floating-point sizes
        CPUI_FLOAT_FLOAT2FLOAT = 55,
        /// Round towards zero
        CPUI_FLOAT_TRUNC = 56,
        /// Round towards +infinity
        CPUI_FLOAT_CEIL = 57,
        /// Round towards -infinity
        CPUI_FLOAT_FLOOR = 58,
        /// Round towards nearest     
        CPUI_FLOAT_ROUND = 59,
        //Internal opcodes for simplification. Not typically generated in a direct translation
        // Data-flow operations
        /// Phi-node operator
        CPUI_MULTIEQUAL = 60,
        /// Copy with an indirect effect
        CPUI_INDIRECT = 61,
        /// Concatenate
        CPUI_PIECE = 62,
        /// Truncate
        CPUI_SUBPIECE = 63,
        /// Cast from one data-type to another
        CPUI_CAST = 64,
        /// Index into an array ([])
        CPUI_PTRADD = 65,
        /// Drill down to a sub-field  (->)
        CPUI_PTRSUB = 66,
        /// Look-up a \e segmented address
        CPUI_SEGMENTOP = 67,
        /// Recover a value from the \e constant \e pool
        CPUI_CPOOLREF = 68,
        /// Allocate a new object (new)
        CPUI_NEW = 69,
        /// Insert a bit-range
        CPUI_INSERT = 70,
        /// Extract a bit-range
        CPUI_EXTRACT = 71,
        /// Count the 1-bits
        CPUI_POPCOUNT = 72,
        /// Count the leading 0-bits
        CPUI_LZCOUNT = 73,
        /// Value indicating the end of the op-code values
        CPUI_MAX = 74,
    }

    unsafe extern "C++" {
        include!("jingle_sleigh/src/ffi/cpp/sleigh/opcodes.hh");

        #[namespace = "ghidra"]
        type OpCode;

    }
}
