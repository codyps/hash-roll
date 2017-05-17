/* TODO: Cyclic polynomial (buzhash)
 *
 * H = s ** (k -1) (h(c_1)) ^ s**(k-2)(h(c_2)) ^ ... ^ s(h(c_(k-1))) ^ h(c_k)
 * where s(x) is a barrel shift of x (ABCDEFG becomes BCDEFGA, where each letter is a bit)
 * s**y(x) is application of s(n) y times.
 *
 * Application:
 *
 *  - H <- s(H)
 *  - c_1 <- s**k(h(c_1))
 *  - H <- s(H) ^ s**k(h(c_1)) ^ h(c_(k+1))
 *
 *  Where c_1 is the character to remove,
 *      c_(k+1) is the character to add
 *
 * Parameters:
 *  - k: number of inputs to contain (can be un-capped?)
 *  - h: a hash function from inputs to integers on [2, 2**L)
 *
 * State:
 *  - every input contained in the hash (if removal is required)
 *  - previous hash result
 */

struct BuzHash {
    _x: ()
}
