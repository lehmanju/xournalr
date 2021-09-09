use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Float(f64);

impl Float {
    pub fn new(from: f64) -> Self {
        if !from.is_finite() {
            panic!("Invalid floating point value: {}", from);
        }
        Self(from)
    }
}

impl From<Float> for f64 {
    fn from(f: Float) -> Self {
        f.0
    }
}

impl Eq for Float {}

impl Ord for Float {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.0.partial_cmp(&rhs.0).unwrap()
    }
}

impl Debug for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <f64 as Debug>::fmt(&self.0, f)
    }
}

impl Display for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <f64 as Display>::fmt(&self.0, f)
    }
}

macro_rules! impl_op {
    ($op:ident, $fn:ident) => {
        impl_op!(@internal $op, $fn, Float, |rhs| rhs.0);
        impl_op!(@internal $op, $fn, &Float, |rhs| rhs.0);
        impl_op!(@internal $op, $fn, f64, |rhs| rhs);
        impl_op!(@internal $op, $fn, &f64, |rhs| *rhs);
    };

    (@internal $op:ident, $fn:ident, $ty:ty, $rhs:expr) => {
        impl $op<$ty> for Float {
            type Output = Self;
            fn $fn(self, rhs: $ty) -> Self {
                fn rhs_access() -> impl FnOnce($ty) -> f64 {
                    $rhs
                }
                Self::new(self.0.$fn(rhs_access()(rhs)))
            }
        }
    };
}

impl_op!(Add, add);
impl_op!(Sub, sub);
impl_op!(Mul, mul);
impl_op!(Div, div);
