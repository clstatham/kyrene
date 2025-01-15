use crate::{prelude::Component, util::TypeInfo};

pub trait Bundle: Sized + 'static {
    fn into_dyn_components(self) -> Vec<(TypeInfo, Box<dyn Component>)>;
}

macro_rules! impl_bundle_tuple {
    ($($t:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($t: Component),*> Bundle for ($($t,)*) {
            fn into_dyn_components(self) -> Vec<(TypeInfo, Box<dyn Component>)> {
                let ($($t,)*) = self;
                vec![$(
                    (TypeInfo::of::<$t>(), Box::new($t))
                ),*]
            }
        }
    };
}

impl_bundle_tuple!(A);
impl_bundle_tuple!(A, B);
impl_bundle_tuple!(A, B, C);
impl_bundle_tuple!(A, B, C, D);
impl_bundle_tuple!(A, B, C, D, E);
