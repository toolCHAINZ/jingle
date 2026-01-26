use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::OnceLock,
};

use crate::analysis::cpa::state::AbstractState;

type StrengthenFn = fn(&mut dyn Any, &dyn Any);

/// A factory wrapper used with `inventory` so each registration can provide
/// the pair `(TypeId of target, TypeId of other, StrengthenFn)` at runtime.
pub struct StrengthenFactory(pub fn() -> (TypeId, TypeId, StrengthenFn));

inventory::collect!(StrengthenFactory);

#[macro_export]
macro_rules! register_strengthen {
    ($From:ty, $To:ty, $func:path) => {
        const _: () = {
            // wrapper used to adapt the concrete fn signature `fn(&$From, &$To) -> Option<$From>`
            // into the registry-required `fn(&dyn Any, &dyn Any) -> Option<Box<dyn Any>>`.
            fn wrapper(a: &mut dyn std::any::Any, b: &dyn std::any::Any) {
                let a = a.downcast_ref::<$From>();
                let b = b.downcast_ref::<$To>();
                if let Some(a) = a
                    && let Some(b) = b
                {
                    ($func)(a, b);
                }
            }

            // factory function (concrete fn pointer) that returns the triple the registry expects.
            fn factory() -> (
                std::any::TypeId,
                std::any::TypeId,
                fn(&mut dyn std::any::Any, &dyn std::any::Any),
            ) {
                (
                    std::any::TypeId::of::<$From>(),
                    std::any::TypeId::of::<$To>(),
                    wrapper,
                )
            }

            // Submit the factory to inventory so it is discovered at link time.
            inventory::submit! {
                $crate::analysis::compound::strengthen::StrengthenFactory(factory)
            }
        };
    };
}

static STRENGTHEN_REGISTRY: OnceLock<HashMap<(TypeId, TypeId), StrengthenFn>> = OnceLock::new();

fn build_strengthen_registry() -> HashMap<(TypeId, TypeId), StrengthenFn> {
    let mut m = HashMap::new();
    for f in inventory::iter::<StrengthenFactory> {
        let (a, b, fun) = (f.0)();
        m.insert((a, b), fun);
    }
    m
}

fn register_lookup(src: TypeId, other: TypeId) -> Option<StrengthenFn> {
    let map = STRENGTHEN_REGISTRY.get_or_init(build_strengthen_registry);
    map.get(&(src, other)).copied()
}

pub trait ComponentStrengthen: 'static {
    /// Attempt to strengthen `self` using information from `other`.
    ///
    /// The default implementation performs a lookup in the inventory-backed
    /// registry of strengthening functions keyed by `(TypeId of Self, TypeId of other)`.
    /// Registered functions have signature `fn(&dyn Any, &dyn Any) -> Option<Box<dyn Any>>`
    /// and are expected to return a boxed concrete `Self` on success.
    fn try_strengthen<'a, 'b>(&'a mut self, other: &'b dyn Any)
    where
        Self: Sized,
    {
        if let Some(func) = register_lookup(TypeId::of::<Self>(), other.type_id()) {
            func(self as &mut dyn Any, other)
        }
    }
}

impl<T: AbstractState> ComponentStrengthen for T where T: 'static {}
