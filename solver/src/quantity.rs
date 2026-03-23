use std::ops::{AddAssign, Mul, Sub, SubAssign};

use good_lp::{Expression, Variable};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Quantity(pub f64);

impl From<f64> for Quantity {
    fn from(value: f64) -> Self {
        Quantity(value)
    }
}

impl From<Quantity> for f64 {
    fn from(value: Quantity) -> Self {
        value.0
    }
}

impl Mul<Variable> for Quantity {
    type Output = Expression;
    fn mul(self, rhs: Variable) -> Self::Output {
        self.0 * rhs
    }
}

impl SubAssign<Quantity> for Expression {
    fn sub_assign(&mut self, rhs: Quantity) {
        *self -= rhs.0;
    }
}

impl From<Quantity> for Expression {
    fn from(value: Quantity) -> Self {
        Expression::from(value.0)
    }
}

impl SubAssign for Quantity {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl AddAssign for Quantity {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Mul<f64> for Quantity {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<f64> for &Quantity {
    type Output = Quantity;
    fn mul(self, rhs: f64) -> Self::Output {
        Quantity(self.0 * rhs)
    }
}

impl Sub<Quantity> for Variable {
    type Output = Expression;
    fn sub(self, rhs: Quantity) -> Self::Output {
        self - rhs.0
    }
}
