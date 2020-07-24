/// Rapid Asymmetric Maximum (RAM) is a fast chunking algorithm
///
/// - Has a minimum block size (it's "window" size)
/// - Does not provide an upper bound on block size (though paper discusses a RAML variant that
///   does).
///
/// doi:10.1016/j.future.2017.02.013
#[derive(Debug)]
pub struct Ram {
    /// window size
    ///
    /// fixed data
    w: u64,

    /// global index (number of processed bytes since split)
    i: u64,

    ///
    max_val: u8,
}

impl Ram {
    /// Construct a RAM instance with window size `w`
    ///
    /// `w` is also the minimum block size
    pub fn with_w(w: u64) -> Self {
        Self {
            w,
            max_val: 0,
            i: 0,
        }
    }

    pub fn feed(&mut self, input: &[u8]) -> Option<usize> {
        let i = self.i;

        for (l_i, b) in input.iter().cloned().enumerate() {
            if b >= self.max_val {
                // minimum block size
                let ri = l_i as u64 + i;
                if ri > self.w {
                    self.i = 0;
                    self.max_val = 0;
                    return Some(l_i);
                }

                self.max_val = b;
            }
        }

        self.i += input.len() as u64;
        None
    }
}
