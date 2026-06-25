use crate::{NodeFFI, RecExprFFI};

pub mod bridge;
pub mod egg;
pub mod print;
pub mod slotted;

pub trait FFI {
    type EG;

    fn to_ffi(&self, egraph: Option<&Self::EG>) -> RecExprFFI;
}

pub trait FFIInner {
    type EG;

    fn to_ffi(&self, _type_: Option<RecExprFFI>) -> NodeFFI {
        Default::default()
    }
    fn to_ffi_with_childs(&self, _children: &[usize], _egraph: Option<&Self::EG>) -> NodeFFI {
        Default::default()
    }
}
