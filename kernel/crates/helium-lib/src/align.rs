pub trait ConstAlign {
    /// Aligns `x` down to the nearest multiple of `N`. If `x` is already a
    /// multiple of `N`, it is returned unchanged.
    fn const_align_down<const N: u64>(self) -> Self;

    /// Aligns `x` up to the nearest multiple of `N`. If `x` is already a
    /// multiple of `N`, it is returned unchanged. If an overflow occurs when
    /// adding `N - 1` to `x`, the result is undefined.
    fn const_align_up<const N: u64>(self) -> Self;
}

pub trait Align {
    /// Aligns `x` down to the nearest multiple of `align`. If `x` is already
    /// a multiple of `align`, it is returned unchanged.
    fn align_down(self, align: Self) -> Self;

    /// Aligns `x` up to the nearest multiple of `align`. If `x` is already a
    /// multiple of `align`, it is returned unchanged.
    /// If an overflow occurs when adding `align - 1` to `x`, the result is
    /// undefined.
    fn align_up(self, align: Self) -> Self;
}

macro_rules! impl_const_align {
    ($($t:ty),*) => {
        $(
            impl ConstAlign for $t {
                fn const_align_down<const N: u64>(self) -> Self {
                    down::<N>(self as u64) as $t
                }
                fn const_align_up<const N: u64>(self) -> Self {
                    up::<N>(self as u64) as $t
                }
            }
        )*
    };
}

macro_rules! impl_align {
    ($($t:ty),*) => {
        $(
            impl Align for $t {
                fn align_down(self, align: Self) -> Self {
                    assert!(align.is_power_of_two());
                    self & !(align - 1)
                }
                fn align_up(self, align: Self) -> Self {
                    assert!(align.is_power_of_two());
                    (self + align - 1) & !(align - 1)
                }
            }
        )*
    };
}

impl_const_align!(u8, u16, u32, u64, usize);
impl_align!(u8, u16, u32, u64, usize);

/// Aligns `x` down to the nearest multiple of `N`. If `x` is already a
/// multiple of `N`, it is returned unchanged.
pub const fn down<const N: u64>(x: u64) -> u64 {
    assert!(N.is_power_of_two());
    x & !(N - 1)
}

/// Aligns `x` up to the nearest multiple of `N`. If `x` is already a
/// multiple of `N`, it is returned unchanged. If an overflow occurs when
/// adding `N - 1` to `x`, the result is undefined.
pub const fn up<const N: u64>(x: u64) -> u64 {
    assert!(N.is_power_of_two());
    down::<N>(x + N - 1)
}

/// Verifies that `x` is a multiple of `N`.
pub const fn aligned<const N: u64>(x: u64) -> bool {
    assert!(N.is_power_of_two());
    x & (N - 1) == 0
}
