use std::mem;
use std::ops::{Index,IndexMut};

#[derive(Clone,Debug)]
pub struct Buf<T> {
    inner: Vec<T>,
    limit: usize,
    first: usize,
}

impl<T> Buf<T> {
    pub fn new(limit: usize) -> Self {
        Buf {
            inner: Vec::with_capacity(limit),
            limit: limit,
            first: 0
        }
    }

    pub fn push(&mut self, mut new: T) -> Option<T> {
        if self.inner.len() < self.limit {
            self.inner.push(new);
            None
        } else {
            /* full buffer */
            let f = self.first;
            mem::swap(&mut new, &mut self.inner[f]);
            self.first = (f + 1) % self.limit;
            Some(new)
        }
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter::from(&self)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn as_slices(&self) -> (&[T], &[T]) {
        let (a2, a1) = self.inner.split_at(self.first);
        (a1, a2)
    }

    pub fn to_vec(&self) -> Vec<T>
        where T: Clone
    {
        let (a1, a2) = self.as_slices();

        let mut v = Vec::with_capacity(a1.len() + a2.len());
        v.extend_from_slice(a1);
        v.extend_from_slice(a2);
        v
    }
}

impl<'a, A, B> PartialEq<[B]> for Buf<A> where A: PartialEq<B> {
    fn eq(&self, other: &[B]) -> bool
    {
        if other.len() != self.len() {
            return false;
        }

        self.iter().eq(other.iter())
    }
}

impl<'a, 'b, A, B> PartialEq<&'b [B]> for Buf<A> where A: PartialEq<B> {
    fn eq(&self, other: & &'b[B]) -> bool
    {
        if other.len() != self.len() {
            return false;
        }

        self.iter().eq(other.iter())
    }
}

impl<T> Index<usize> for Buf<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.inner.len());
        &self.inner[(index + self.first) % self.limit]
    }
}

impl<T> IndexMut<usize> for Buf<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.inner.len());
        &mut self.inner[(index + self.first) & self.limit]
    }
}

/*
 * We could fake this with cycle().skip().take(), but that is (probably) less efficient
 */
#[derive(Clone,Debug)]
pub struct Iter<'a, T: 'a> {
    inner: &'a Buf<T>,
    pos: usize,
}

impl<'a, T> Iter<'a, T> {
    pub fn from(inner: &'a Buf<T>) -> Self {
        Iter {
            inner: inner,
            pos: 0,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item>
    {
        let p = self.pos;
        if self.pos < self.inner.len() {
            self.pos = p + 1;
            Some(&self.inner[p])
        } else {
            None
        }
    }
}

#[test]
fn test_buf() {
    let mut b = Buf::new(3);

    assert_eq!(b.push(1), None);
    assert_eq!(b.push(2), None);
    assert_eq!(b.push(3), None);
    assert_eq!(b.push(4), Some(1));


    assert_eq!(b, &[2, 3, 4][..]);

    {
        let mut i = b.iter().cloned();
        assert_eq!(i.next(), Some(2));
        assert_eq!(i.next(), Some(3));
        assert_eq!(i.next(), Some(4));
        assert_eq!(i.next(), None);
        assert_eq!(i.next(), None);
    }

    assert_eq!(b.as_slices(), (&[2, 3][..], &[4][..]));
    assert_eq!(b.to_vec(), &[2, 3, 4]);

    assert_eq!(b, &[2, 3, 4][..]);
}

