use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use ecow::{EcoString, eco_format};
use typst_utils::Numeric;

use crate::foundations::{Fold, Repr, Resolve, StyleChain, cast, ty};
use crate::layout::{Abs, Em, Length, Ratio};

/// A length in relation to some known length.
///
/// This type is a combination of a [length] with a [ratio]. It results from
/// addition and subtraction of a length and a ratio. Wherever a relative length
/// is expected, you can also use a bare length or ratio.
///
/// # Relative to the page
/// A common use case is setting the width or height of a layout element (e.g.,
/// [block], [rect], etc.) as a certain percentage of the width of the page.
/// Here, the rectangle's width is set to `{25%}`, so it takes up one fourth of
/// the page's _inner_ width (the width minus margins).
///
/// ```example
/// #rect(width: 25%)
/// ```
///
/// Bare lengths or ratios are always valid where relative lengths are expected,
/// but the two can also be freely mixed:
/// ```example
/// #rect(width: 25% + 1cm)
/// ```
///
/// If you're trying to size an element so that it takes up the page's _full_
/// width, you have a few options (this highly depends on your exact use case):
///
/// 1. Set page margins to `{0pt}` (`[#set page(margin: 0pt)]`)
/// 2. Multiply the ratio by the known full page width (`{21cm * 69%}`)
/// 3. Use padding which will negate the margins (`[#pad(x: -2.5cm, ...)]`)
/// 4. Use the page [background](page.background) or
///    [foreground](page.foreground) field as those don't take margins into
///    account (note that it will render the content outside of the document
///    flow, see [place] to control the content position)
///
/// # Relative to a container
/// When a layout element (e.g. a [rect]) is nested in another layout container
/// (e.g. a [block]) instead of being a direct descendant of the page, relative
/// widths become relative to the container:
///
/// ```example
/// #block(
///   width: 100pt,
///   fill: aqua,
///   rect(width: 50%),
/// )
/// ```
///
/// # Scripting
/// You can multiply relative lengths by [ratios]($ratio), [integers]($int), and
/// [floats]($float).
///
/// A relative length has the following fields:
/// - `length`: Its length component.
/// - `ratio`: Its ratio component.
///
/// ```example
/// #(100% - 50pt).length \
/// #(100% - 50pt).ratio
/// ```
#[ty(cast, name = "relative", title = "Relative Length")]
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rel<T: Numeric = Length> {
    /// The relative part.
    pub rel: Ratio,
    /// The absolute part.
    pub abs: T,
}

impl<T: Numeric> Rel<T> {
    /// The zero relative.
    pub fn zero() -> Self {
        Self { rel: Ratio::zero(), abs: T::zero() }
    }

    /// A relative with a ratio of `100%` and no absolute part.
    pub fn one() -> Self {
        Self { rel: Ratio::one(), abs: T::zero() }
    }

    /// Create a new relative from its parts.
    pub fn new(rel: Ratio, abs: T) -> Self {
        Self { rel, abs }
    }

    /// Whether both parts are zero.
    pub fn is_zero(self) -> bool {
        self.rel.is_zero() && self.abs == T::zero()
    }

    /// Whether the relative part is one and the absolute part is zero.
    pub fn is_one(self) -> bool {
        self.rel.is_one() && self.abs == T::zero()
    }

    /// Evaluate this relative to the given `whole`.
    pub fn relative_to(self, whole: T) -> T {
        self.rel.of(whole) + self.abs
    }

    /// Map the absolute part with `f`.
    pub fn map<F, U>(self, f: F) -> Rel<U>
    where
        F: FnOnce(T) -> U,
        U: Numeric,
    {
        Rel { rel: self.rel, abs: f(self.abs) }
    }
}

impl Rel<Length> {
    /// Try to divide two relative lengths.
    pub fn try_div(self, other: Self) -> Option<f64> {
        if self.rel.is_zero() && other.rel.is_zero() {
            self.abs.try_div(other.abs)
        } else if self.abs.is_zero() && other.abs.is_zero() {
            Some(self.rel / other.rel)
        } else {
            None
        }
    }
}

impl<T: Numeric + Debug> Debug for Rel<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (self.rel.is_zero(), self.abs.is_zero()) {
            (false, false) => write!(f, "{:?} + {:?}", self.rel, self.abs),
            (false, true) => self.rel.fmt(f),
            (true, _) => self.abs.fmt(f),
        }
    }
}

impl<T: Numeric + Repr> Repr for Rel<T> {
    fn repr(&self) -> EcoString {
        eco_format!("{} + {}", self.rel.repr(), self.abs.repr())
    }
}

impl From<Abs> for Rel<Length> {
    fn from(abs: Abs) -> Self {
        Rel::from(Length::from(abs))
    }
}

impl From<Em> for Rel<Length> {
    fn from(em: Em) -> Self {
        Rel::from(Length::from(em))
    }
}

impl<T: Numeric> From<T> for Rel<T> {
    fn from(abs: T) -> Self {
        Self { rel: Ratio::zero(), abs }
    }
}

impl<T: Numeric> From<Ratio> for Rel<T> {
    fn from(rel: Ratio) -> Self {
        Self { rel, abs: T::zero() }
    }
}

impl<T: Numeric + PartialOrd> PartialOrd for Rel<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.rel.is_zero() && other.rel.is_zero() {
            self.abs.partial_cmp(&other.abs)
        } else if self.abs.is_zero() && other.abs.is_zero() {
            self.rel.partial_cmp(&other.rel)
        } else {
            None
        }
    }
}

impl<T: Numeric> Neg for Rel<T> {
    type Output = Self;

    fn neg(self) -> Self {
        Self { rel: -self.rel, abs: -self.abs }
    }
}

impl<T: Numeric> Add for Rel<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl<T: Numeric> Sub for Rel<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self + -other
    }
}

impl<T: Numeric> Mul<f64> for Rel<T> {
    type Output = Self;

    fn mul(self, other: f64) -> Self::Output {
        Self { rel: self.rel * other, abs: self.abs * other }
    }
}

impl<T: Numeric> Mul<Rel<T>> for f64 {
    type Output = Rel<T>;

    fn mul(self, other: Rel<T>) -> Self::Output {
        other * self
    }
}

impl<T: Numeric> Div<f64> for Rel<T> {
    type Output = Self;

    fn div(self, other: f64) -> Self::Output {
        Self { rel: self.rel / other, abs: self.abs / other }
    }
}

impl<T: Numeric + AddAssign> AddAssign for Rel<T> {
    fn add_assign(&mut self, other: Self) {
        self.rel += other.rel;
        self.abs += other.abs;
    }
}

impl<T: Numeric + SubAssign> SubAssign for Rel<T> {
    fn sub_assign(&mut self, other: Self) {
        self.rel -= other.rel;
        self.abs -= other.abs;
    }
}

impl<T: Numeric + MulAssign<f64>> MulAssign<f64> for Rel<T> {
    fn mul_assign(&mut self, other: f64) {
        self.rel *= other;
        self.abs *= other;
    }
}

impl<T: Numeric + DivAssign<f64>> DivAssign<f64> for Rel<T> {
    fn div_assign(&mut self, other: f64) {
        self.rel /= other;
        self.abs /= other;
    }
}

impl<T: Numeric> Add<T> for Ratio {
    type Output = Rel<T>;

    fn add(self, other: T) -> Self::Output {
        Rel::from(self) + Rel::from(other)
    }
}

impl<T: Numeric> Add<T> for Rel<T> {
    type Output = Self;

    fn add(self, other: T) -> Self::Output {
        self + Rel::from(other)
    }
}

impl<T: Numeric> Add<Ratio> for Rel<T> {
    type Output = Self;

    fn add(self, other: Ratio) -> Self::Output {
        self + Rel::from(other)
    }
}

impl<T> Resolve for Rel<T>
where
    T: Resolve + Numeric,
    <T as Resolve>::Output: Numeric,
{
    type Output = Rel<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|abs| abs.resolve(styles))
    }
}

impl<T> Fold for Rel<T>
where
    T: Numeric + Fold,
{
    fn fold(self, outer: Self) -> Self {
        Self { rel: self.rel, abs: self.abs.fold(outer.abs) }
    }
}

cast! {
    Rel<Abs>,
    self => self.map(Length::from).into_value(),
}
