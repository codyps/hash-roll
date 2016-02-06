#[derive(Clone,Debug)]
pub struct Buf<T> {
    inner: Vec<T>,
    limit: usize,
    first: usize,
}

impl<T> Buf<T> {
    pub fn new(limit: usize) -> Self {
        CircBuf {
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

    pub fn iter(&'a self) -> Iter<'a> {
        Iter::new(&self)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, 'b, A, B> PartialEq<&'b [B]> for Buf<A> where A: PartialEq<B> {
    fn eq(&self, other: &'b [B]) -> bool
    {
        if other.len() != self.len() {
            return false;
        }
        
        /* FIXME: might be more efficient to generate 2 slices and call those eq()s */
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
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
    type Output = T;
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.inner.len());
        &mut self.inner[(index + self.first) & self.limit]
    }
}

/*
 * We could fake this with cycle().skip().take(), but that is (probably) less efficient
 */
#[derive(Clone,Debug)]
pub struct Iter<'a, T> {
    inner: &'a Buf<T>,
    pos: usize,
}

impl<T> Iterator for Iter<T> {
    type Item = &T;

    fn next(&mut self) -> Option<Self::Item>
    {
        let p = self.pos;
        if self.pos < self.inner.len() {
            self.pos = p + 1;
            Some(self.inner[p])
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


    assert_eq!(b, &[2, 3, 4]);

    {
        let i = b.iter();
        assert_eq!(i.next(), Some(2));
        assert_eq!(i.next(), Some(3));
        assert_eq!(i.next(), Some(4));
        assert_eq!(i.next(), None);
        assert_eq!(i.next(), None);
    }

    assert_eq!(b, &[2, 3, 4]);
}

