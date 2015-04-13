#[doc="

    Module: cvec

    This is a slight modification of Vec from the standard library, but uses
    C-style allocation and reallocation so we can safely construct it from a
    C pointer, or return it as a C pointer. This cannot safely be used on
    zero-sized types, and will panic if you try.

    This CANNOT be used on types that implement Drop, or else we will leak memory
    in the destructor.
"]

extern crate libc;
extern crate core;

use std::ops::Index;
use libc::{c_void, size_t};
use libc::funcs::c95::stdlib::{malloc, realloc, free};
use std::mem;
use self::core::raw::Slice as RawSlice;
use self::core::num::Int;
use std::ptr;
use std::fmt;

const DEFAULT_CVEC_CAPACITY: usize = 8;

pub type Buf = CVec<u8>;

pub struct CVec<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    mutable: bool,
}

impl<T> CVec<T> {

    /// Verify that the T type has a size
    fn check_type_size() {
        if mem::size_of::<T>() == 0 {
            panic!("tried to use a CVec with a zero-size type");
        }
    }

    /// Create a new CVec
    #[allow(dead_code)]
    pub fn new() -> Option<CVec<T>> {
        CVec::<T>::with_capacity(DEFAULT_CVEC_CAPACITY)
    }

    /// Constructs a new CVec with given capacity
    /// returns None if the allocation fails
    pub fn with_capacity(capacity: usize) -> Option<CVec<T>> {
        let capacity = if capacity > 0 { capacity } else { DEFAULT_CVEC_CAPACITY } ;
        CVec::<T>::check_type_size();
        let size = capacity.checked_mul(mem::size_of::<T>() as usize);
        if size.is_none() {
            return None;
        }
        let ptr = unsafe { malloc(size.unwrap() as size_t) } as *mut T;
        if ptr.is_null() {
            None
        } else {
            Some(CVec {
                ptr: ptr,
                len: 0,
                cap: capacity,
                mutable: true
            })
        }
    }

    /// Constructs a new CVec around a given buffer in memory, without copying
    /// If the input pointer is null or buf_size is 0, then None is returned
    /// The returned CVec CANNOT be modified!
    pub unsafe fn from_raw_buf(ptr: *const T, buf_size: usize) -> Option<CVec<T>> {
        if ptr.is_null() || buf_size == 0 {
            None
        } else {
            Some(CVec {
                ptr: ptr as *mut T,
                len: buf_size,
                cap: buf_size,
                mutable: false
            })
        }
    }

    /// Converts this CVec to a raw pointer. The CVec cannot be used after this
    /// is called. The raw pointer must be freed by the caller.
    pub fn into_raw_buf(self) -> (*mut T, usize) {
        let ret = (self.ptr, self.len);
        unsafe { mem::forget(self); }
        ret
    }

    /// Return the length of the CVec
    pub fn len(&self) -> usize {
        self.len
    }

    /// Effect: doubles the CVec's capacity
    /// returns None if the allocation failed
    pub fn double_capacity(&mut self) -> Option<()> {
        assert!(self.mutable);
        let old_size = self.cap * mem::size_of::<T>();
        let size = old_size * 2;
        if old_size > size {
            mem::drop(self);
            return None;
        }
        unsafe {
            let new_ptr = realloc(self.ptr as *mut c_void, size as size_t);
            if new_ptr.is_null() {
                return None;
            }
            self.ptr = new_ptr as *mut T;
        }
        self.cap = self.cap * 2;
        Some(())
    }

    /// Add a new element to the CVec
    /// returns None if we had to reallocate and it failed
    pub fn push(&mut self, value: T) -> Option<()> {
        assert!(self.mutable);
        if self.len == self.cap {
            try_opt!(self.double_capacity());
        }
        assert!(self.cap > self.len);
        unsafe {
            let end = self.ptr.offset(self.len as isize);
            ptr::write(&mut *end, value);
            self.len += 1;
        }
        Some(())
    }

    /// Remove and return the last element of the CVec
    #[allow(dead_code)]
    pub fn pop(&mut self) -> Option<T> {
        assert!(self.mutable);
        if self.len == 0 {
            None
        } else {
            unsafe {
                self.len -= 1;
                Some(ptr::read(self.as_slice().get_unchecked(self.len())))
            }
        }
    }

    /// Get a refrence to the element at the given index
    /// Does not do any bounds checking!
    unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.as_slice().get_unchecked(index)
    }

    /// Get a reference to the element at the given index
    /// Does bounds checking to ensure that index < len
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    /// gets sizeof(U) bytes at the given index into the CVec
    pub fn get_wide<U>(&self, index: usize) -> Option<U> {
        let size = mem::size_of::<U>();
        let end = index + size;
        if end > self.len {
            None
        } else {
            let ptr = unsafe { self.get_raw_pointer_to_item(index) };
            Some(unsafe { ptr::read(ptr as *const U)} )
        }
    }

    /// Return an iterator over the CVec's contents
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }

    /// Return an iterator over a slice of the CVec
    pub fn limit_iter(&self, index: usize, limit: usize) -> Iter<T> {
        Iter::limit_new(self, index, limit)
    }

    /// Get a raw pointer to the item at the given index
    pub unsafe fn get_raw_pointer_to_item(&self, index: usize) -> *const T {
        if index >= self.len {
            ptr::null::<T>()
        } else {
            self.as_slice().as_ptr().offset(index as isize)
        }
    }

    /// Clear the contents of the CVec
    pub fn clear(&mut self) {
        unsafe {
            while self.len > 0 {
                self.len -= 1;
                ptr::read(self.get_unchecked(self.len));
            }
        }
    }
}

impl<T: Clone> CVec<T> {
    /// Add to the CVec length bytes from distance bytes from the end
    pub fn copy_back_pointer(&mut self, distance: usize, length: usize) {
        let mut back_ptr  = self.len - distance - 1;
        let mut length = length;
        let mut c;
        while length > 0 {
            c = self[back_ptr].clone();
            self.push(c);
            back_ptr += 1;
            length -= 1;
        }
    }
}

impl<T> Index<usize> for CVec<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: &usize) -> &T {
        assert!(*index < self.len);
        unsafe { self.get_unchecked(*index) }
    }

}

#[unsafe_destructor]
impl<T> Drop for CVec<T> {
    fn drop(&mut self) {
        if self.mutable {
            self.clear();
            unsafe { free(self.ptr as *mut c_void); }
        }
    }
}

impl<T> AsSlice<T> for CVec<T> {
    fn as_slice<'a>(&'a self) -> &'a [T] {
        unsafe {
            mem::transmute(RawSlice {
                data: self.ptr as *const T,
                len: self.len
            })
        }
    }
}

impl<T: fmt::Show> fmt::Show for CVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Show::fmt(self.as_slice(), f)
    }
}

/////////////////////////////////////////////////////////////////////
//                            Iterator                             //
/////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Show)]
pub struct Iter<'a, T: 'a> {
    cvec: &'a CVec<T>,
    index: usize,
    limit: Option<usize>
}

impl<'a, T> Iter<'a, T> {
    fn new(vec: &'a CVec<T>) -> Iter<'a, T> {
        Iter {
            cvec: vec,
            index: 0,
            limit: None,
        }
    }

    fn limit_new(vec: &'a CVec<T>, index: usize, limit: usize) -> Iter<'a, T> {
        Iter {
            cvec: vec,
            index: index,
            limit: Some(limit),
        }
    }

    pub fn next_wide<F>(&mut self) -> Option<F> {
        let index = self.index;
        let size = mem::size_of::<F>();
        self.index += size;
        if self.limit.is_some() && self.index > self.limit.unwrap() {
            None
        } else {
            self.cvec.get_wide::<F>(index)
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    #[allow(dead_code)]
    pub fn skip(&self, n: usize) -> Iter<'a, T> {
        Iter {
            cvec: self.cvec,
            index: self.index + n,
            limit: self.limit
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        let index = self.index;
        self.index += 1;
        if self.limit.is_some() && self.index > self.limit.unwrap() {
            None
        } else {
            self.cvec.get(index)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.cvec.len();
        (len, Some(len))
    }
}


#[cfg(test)]
mod cvec_tests {
    use super::CVec;

    fn setup() -> CVec<u8> {
        let mut v: CVec<u8> = CVec::new().unwrap();
        for i in 1..10 {
            v.push(i);
        }
        v
    }

    #[test]
    fn test_iterator() {
        let mut expect = 1;
        for &el in setup().iter() {
            assert_eq!(expect, el);
            expect += 1;
        }
    }

    #[test]
    fn test_skip() {
        let mut expect = 4;
        for &el in setup().iter().skip(3) {
            assert_eq!(expect, el);
            expect += 1;
        }
    }

    #[test]
    fn test_pop() {
        let mut v = setup();
        let mut expect = 9;
        while let Some(el) = v.pop() {
            assert_eq!(el, expect);
            expect -= 1;
        }
    }

    #[test]
    fn test_push() {
        let mut v = setup();
        v.push(5);
        assert_eq!(v.pop().unwrap(), 5);
    }

    #[test]
    fn test_index() {
        let mut v = setup();
        for i in 0 .. v.len() {
            assert_eq!(v[i], (i + 1) as u8);
        }
        v.push(42);
        assert_eq!(v[v.len() - 1], 42);
    }
}















