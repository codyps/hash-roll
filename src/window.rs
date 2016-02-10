use std::ops::Deref;

#[derive(Clone,Debug)]
pub struct Buf<T> {
    inner: Vec<T>,
    window_len: usize,
}

impl<T> Buf<T> {
    pub fn new(window_len: usize) -> Self {
        Buf {
            inner: Vec::with_capacity(window_len),
            window_len: window_len,
        }
    }

    pub fn push(&mut self, new: T) -> Option<&T> {
        if self.inner.len() < self.window_len {
            self.inner.push(new);
            None
        } else {
            /* full buffer */
            self.inner.push(new);
            Some(&self.inner[self.inner.len() - self.window_len - 1])
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn window(&self) -> &[T] {
        &self.inner[(self.inner.len() - self.window_len)..]
    }

    pub fn into_vec(self) -> Vec<T>
    {
        self.inner
    }
}

impl<T> Deref for Buf<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[test]
fn test_buf() {
    let mut b = Buf::new(3);

    assert_eq!(b.push(1), None);
    assert_eq!(b.push(2), None);
    assert_eq!(b.push(3), None);
    assert_eq!(b.push(4), Some(1).as_ref());

    assert_eq!(&b[..], &[1, 2, 3, 4][..]);
    assert_eq!(b.window(), [2,3,4]);
}

