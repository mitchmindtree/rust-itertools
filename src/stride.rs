//! Licensed under the Apache License, Version 2.0
//! http://www.apache.org/licenses/LICENSE-2.0 or the MIT license
//! http://opensource.org/licenses/MIT, at your
//! option. This file may not be copied, modified, or distributed
//! except according to those terms.

use std::kinds;
use std::mem;
use std::num;
use std::ptr;
use std::fmt;

/// Similar to the slice iterator, but with a certain number of steps
/// (stride) skipped per iteration.
///
/// Does not support zero-sized `A`.
///
/// Iterator element type is `&'a A`
pub struct Stride<'a, A> {
    // begin is NULL when the iterator is exhausted, because
    // both begin and end are inclusive endpoints.
    begin: *const A,
    // Unlike the slice iterator, end is inclusive and the last
    // pointer we will visit. This makes it possible to have
    // safe stride iterators for columns in matrices etc.
    end: *const A,
    stride: int,
    life: kinds::marker::ContravariantLifetime<'a>,
}

/// Stride with mutable elements
pub struct MutStride<'a, A> {
    begin: *mut A,
    end: *mut A,
    stride: int,
    life: kinds::marker::ContravariantLifetime<'a>,
    nocopy: kinds::marker::NoCopy
}

impl<'a, A> Stride<'a, A>
{
    /// Create Stride iterator from a slice and the element step count
    ///
    /// ## Example
    ///
    /// ```
    /// let xs = [0i, 1, 2, 3, 4, 5];
    /// let mut iter = Stride::from_slice(xs.as_slice(), 2);
    /// ```
    pub fn from_slice(xs: &'a [A], step: uint) -> Stride<'a, A>
    {
        assert!(step != 0);
        assert!(mem::size_of::<A>() != 0);
        let mut begin = ptr::null();
        let mut end = ptr::null();
        let (d, r) = num::div_rem(xs.len(), step);
        let nelem = d + if r > 0 { 1 } else { 0 };
        unsafe {
            if nelem != 0 {
                begin = xs.as_ptr();
                end = begin.offset(((nelem - 1) * step) as int);
            }
            Stride::from_ptrs(begin, end, step as int)
        }
    }

    /// Create Stride iterator from raw pointers from the *inclusive*
    /// pointer range [begin, end].
    ///
    /// **Note:** `end` **must** be a whole number of `stride` steps away
    /// from `begin`
    pub unsafe fn from_ptrs(begin: *const A, end: *const A, stride: int) -> Stride<'a, A>
    {
        Stride {
            begin: begin,
            end: end,
            stride: stride,
            life: kinds::marker::ContravariantLifetime,
        }
    }

    /// Create Stride iterator from an existing Stride iterator
    pub fn from_stride(it: Stride<'a, A>, step: uint) -> Stride<'a, A>
    {
        assert!(step != 0);
        let newstride = it.stride * (step as int);
        unsafe {
            let nelem = ((it.end.to_uint() as int) - (it.begin.to_uint() as int))
                        / (mem::size_of::<A>() as int)
                        / newstride;
            let newend = it.begin.offset(nelem * newstride);
            Stride::from_ptrs(it.begin, newend, newstride)
        }
    }

    /// Swap the being and end pointer and reverse the stride,
    /// in effect reversing the iterator.
    #[inline]
    pub fn swap_ends(&mut self) {
        if !self.begin.is_null() {
            mem::swap(&mut self.begin, &mut self.end);
            self.stride = -self.stride;
        }
    }
}

impl<'a, A> MutStride<'a, A>
{
    /// Create Stride iterator from a slice and the element step count
    ///
    /// ## Example
    ///
    /// ```
    /// let xs = [0i, 1, 2, 3, 4, 5];
    /// let mut iter = Stride::from_slice(xs.as_slice(), 2);
    /// ```
    pub fn from_mut_slice(xs: &'a mut [A], step: uint) -> MutStride<'a, A>
    {
        assert!(step != 0);
        assert!(mem::size_of::<A>() != 0);
        let mut begin = ptr::mut_null();
        let mut end = ptr::mut_null();
        let (d, r) = num::div_rem(xs.len(), step);
        let nelem = d + if r > 0 { 1 } else { 0 };
        unsafe {
            if nelem != 0 {
                begin = xs.as_mut_ptr();
                end = begin.offset(((nelem - 1) * step) as int);
            }
            MutStride::from_ptrs(begin, end, step as int)
        }
    }

    /// Create Stride iterator from raw pointers from the *inclusive*
    /// pointer range [begin, end].
    ///
    /// **Note:** `end` **must** be a whole number of `stride` steps away
    /// from `begin`
    pub unsafe fn from_ptrs(begin: *mut A, end: *mut A, stride: int) -> MutStride<'a, A>
    {
        MutStride {
            begin: begin,
            end: end,
            stride: stride,
            life: kinds::marker::ContravariantLifetime,
            nocopy: kinds::marker::NoCopy
        }
    }

    /// Create MutStride iterator from an existing MutStride iterator
    pub fn from_mut_stride(it: MutStride<'a, A>, step: uint) -> MutStride<'a, A>
    {
        assert!(step != 0);
        let newstride = it.stride * (step as int);
        unsafe {
            let nelem = ((it.end.to_uint() as int) - (it.begin.to_uint() as int))
                        / (mem::size_of::<A>() as int)
                        / newstride;
            let newend = it.begin.offset(nelem * newstride);
            MutStride::from_ptrs(it.begin, newend, newstride)
        }
    }

    /// Swap the being and end pointer and reverse the stride,
    /// in effect reversing the iterator.
    #[inline]
    pub fn swap_ends(&mut self) {
        if !self.begin.is_null() {
            mem::swap(&mut self.begin, &mut self.end);
            self.stride = -self.stride;
        }
    }
}
macro_rules! stride_iterator {
    (struct $name:ident -> $ptr:ty, $elem:ty, $null:expr) => {
        impl<'a, A> Iterator<$elem> for $name<'a, A>
        {
            #[inline]
            fn next(&mut self) -> Option<$elem>
            {
                if self.begin.is_null() {
                    None
                } else {
                    unsafe {
                        let elt: $elem = mem::transmute(self.begin);
                        if self.begin == self.end {
                            self.begin = $null;
                        } else {
                            self.begin = self.begin.offset(self.stride);
                        }
                        Some(elt)
                    }
                }
            }

            fn size_hint(&self) -> (uint, Option<uint>)
            {
                let len;
                if self.begin.is_null() {
                    len = 0;
                } else {
                    len = (self.end as uint - self.begin as uint) as int / self.stride
                        / mem::size_of::<A>() as int + 1;
                }

                (len as uint, Some(len as uint))
            }
        }

        impl<'a, A> DoubleEndedIterator<$elem> for $name<'a, A>
        {
            #[inline]
            fn next_back(&mut self) -> Option<$elem>
            {
                if self.begin.is_null() {
                    None
                } else {
                    unsafe {
                        let elt: $elem = mem::transmute(self.end);
                        if self.begin == self.end {
                            self.begin = $null;
                        } else {
                            self.end = self.end.offset(-self.stride);
                        }
                        Some(elt)
                    }
                }
            }
        }

        impl<'a, A> ExactSize<$elem> for $name<'a, A> { }

        impl<'a, A> Index<uint, A> for $name<'a, A>
        {
            fn index<'b>(&'b self, i: &uint) -> &'b A
            {
                assert!(*i < self.size_hint().val0());
                unsafe {
                    let ptr = self.begin.offset(self.stride * (*i as int));
                    mem::transmute(ptr)
                }
            }
        }
    }
}

stride_iterator!{struct Stride -> *const A, &'a A, ptr::null()}
stride_iterator!{struct MutStride -> *mut A, &'a mut A, ptr::mut_null()}

impl<'a, A: fmt::Show> fmt::Show for Stride<'a, A>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let it = *self;
        try!(write!(f, "["));
        for (i, elt) in it.enumerate() {
            if i != 0 {
                try!(write!(f, ", "));
            }
            try!(write!(f, "{}", *elt));
        }
        write!(f, "]")
    }
}

impl<'a, A> Clone for Stride<'a, A>
{
    fn clone(&self) -> Stride<'a, A>
    {
        *self
    }
}
