use std::ops::{Deref, DerefMut};

#[macro_export]
macro_rules! define_enum {
    (
        $(#[$attrs:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_attrs:meta])*
                $variant_name:ident
            ),* $(,)?
        }
    ) => {

        $(#[$attrs])*
        #[derive(enum_map::Enum, Clone, Copy, PartialEq, Eq)]
        $vis enum $name {
            $(
                $(#[$variant_attrs])*
                $variant_name
            ),*
        }

        impl $name {
            $vis const COUNT: usize = <[Self]>::len(&[$(Self::$variant_name),*]);
            $vis const ALL: &[Self; Self::COUNT] = &[$(Self::$variant_name),*];

            #[inline]
            $vis const fn from_idx(idx: u8) -> Self {
                Self::try_from_idx(idx).expect(concat!("Index out of range for `", stringify!($name), "`!"))
            }

            #[inline]
            $vis const fn try_from_idx(idx: u8) -> Option<Self> {
                if idx as usize >= Self::COUNT {
                    None
                } else {
                    // SAFETY:
                    // 1. We know that `idx < COUNT`. Since we don't specify discriminants in the enum
                    //    definition, the variants have discriminants `[0, COUNT)`, so `idx` holds a
                    //    valid enum discriminant.
                    // 2. `Self` is #[repr(u8)]. The Rust reference guarantees that we can transmute
                    //    between a field-less primitive representation enum and its backing primitive
                    //    type, so long as we only transmute valid discriminant values.
                    //    (The reference doesn't directly state this (yet?), see <https://github.com/rust-lang/reference/issues/1947>)
                    Some(unsafe { ::core::mem::transmute::<u8, Self>(idx) })
                }
            }

            #[inline]
            $vis const fn idx(self) -> u8 {
                self as u8
            }
        }
    };
}

pub use define_enum;

#[repr(align(128))]
pub struct CachePadded<T>(pub T);

impl<T> Deref for CachePadded<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for CachePadded<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
