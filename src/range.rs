use std::ops::Bound::{self, *};

pub trait RangeExt<T> {
    fn exceeds_max(&self, other: &T) -> bool
    where
        T: PartialOrd<T>;

    fn under_min(&self, item: &T) -> bool
    where
        T: PartialOrd<T>;

    fn contains(&self, item: &T) -> bool
    where
        T: PartialOrd<T>;

    fn into_tuple(self) -> (std::ops::Bound<T>, std::ops::Bound<T>)
    where
        T: Copy + std::marker::Sized;
}

impl<T, R: std::ops::RangeBounds<T>> RangeExt<T> for R {
    fn exceeds_max(&self, item: &T) -> bool
    where
        T: PartialOrd<T>,
    {
        match self.end_bound() {
            Included(ref i) => {
                if item > i {
                    return true;
                }
            }
            Excluded(ref i) => {
                if item >= i {
                    return true;
                }
            }
            Unbounded => {}
        }

        false
    }

    fn under_min(&self, item: &T) -> bool
    where
        T: PartialOrd<T>,
    {
        match self.start_bound() {
            Included(ref i) => {
                if item < i {
                    return true;
                }
            }
            Excluded(ref i) => {
                if item <= i {
                    return true;
                }
            }
            Unbounded => {}
        }

        false
    }

    fn contains(&self, item: &T) -> bool
    where
        T: PartialOrd<T>,
    {
        /* not excluded by lower */
        if self.under_min(item) {
            return false;
        }

        if self.exceeds_max(item) {
            return false;
        }

        true
    }

    fn into_tuple(self) -> (std::ops::Bound<T>, std::ops::Bound<T>)
    where
        T: Copy + std::marker::Sized,
    {
        (
            bound_cloned(self.start_bound()),
            bound_cloned(self.end_bound()),
        )
    }
}

/// Map a `Bound<&T>` to a `Bound<T>` by cloning the contents of the bound.
///
/// # Examples
///
/// ```
/// use std::ops::Bound::*;
/// use std::ops::RangeBounds;
/// use hash_roll::range::bound_cloned;
///
/// assert_eq!((1..12).start_bound(), Included(&1));
/// assert_eq!(bound_cloned((1..12).start_bound()), Included(1));
/// ```
pub fn bound_cloned<T: Clone>(src: std::ops::Bound<&T>) -> std::ops::Bound<T> {
    match src {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(x) => Bound::Included(x.clone()),
        Bound::Excluded(x) => Bound::Excluded(x.clone()),
    }
}
