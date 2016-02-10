pub trait SliceExt {
    type Item;
    fn split_on<P>(&self, pred: P) -> SplitOn<Self::Item, P> where P: FnMut(&Self::Item) -> bool;
}

impl<T> SliceExt for [T] {
    type Item = T;

    #[inline]
    fn split_on<P>(&self, pred: P) -> SplitOn<T, P> where P: FnMut(&T) -> bool
    {
        SplitOn {
            v: self,
            pred: pred,
        }
    }
}

#[derive(Clone)]
pub struct SplitOn<'a, T: 'a, P>
    where P: FnMut(&T) -> bool
{
    v: &'a [T],
    pred: P,
}

impl<'a, T, P> SplitOn<'a, T, P> where P: FnMut(&T) -> bool {
    #[inline]
    fn finish(&mut self) -> Option<&'a [T]> {
        if self.v.is_empty() {
            None
        } else {
            let ret = Some(self.v);
            self.v = &[];
            ret
        }
    }
}

impl<'a, T, P> Iterator for SplitOn<'a, T, P> where P: FnMut(&T) -> bool {
    type Item = &'a [T];

    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        if self.v.is_empty() { return None; }

        match self.v.iter().position(|x| (self.pred)(x)) {
            None => self.finish(),
            Some(idx) => {
                let ret = Some(&self.v[..idx + 1]);
                self.v = &self.v[idx + 1..];
                ret
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.v.is_empty() {
            (0, Some(0))
        } else {
            (1, Some(self.v.len() + 1))
        }
    }
}

impl<'a, T, P> DoubleEndedIterator for SplitOn<'a, T, P> where P: FnMut(&T) -> bool {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.v.is_empty() { return None; }

        match self.v.iter().rposition(|x| (self.pred)(x)) {
            None => self.finish(),
            Some(idx) => {
                let ret = Some(&self.v[idx..]);
                self.v = &self.v[..idx];
                ret
            }
        }
    }
}
