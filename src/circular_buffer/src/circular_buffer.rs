#[derive(Debug,PartialEq,Eq)]
pub struct CircularBuffer<T> {
    pub(crate) ptr: std::ptr::NonNull<T>,
    pub(crate) len: usize,
}

unsafe impl<T> Send for CircularBuffer<T> {}

impl<T> CircularBuffer<T> {
    // Cannot allocate a circular buffer without virtual memory backing it
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { &*std::ptr::slice_from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *std::ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr() as *const T
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T> Clone for CircularBuffer<T> 
where T: Clone + Copy + Default
{
    fn clone(&self) -> Self {
        let mut dest = Self::new(self.len).expect("Failed to allocate memory mapped circular buffer");
        assert!(dest.len() == self.len());
        dest.as_mut_slice().copy_from_slice(self.as_slice());
        dest
    }
}

impl<T> std::borrow::Borrow<[T]> for CircularBuffer<T> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> std::borrow::BorrowMut<[T]> for CircularBuffer<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

// NOTE: We cannot implement a blanket implementation for SliceIndex<[T], Output = [T]>
//      the following implementations will conflict:
//       - impl<T,I> Index<I> for CircularBuffer<T> where I: SliceIndex<[T], Output = [T]>
//       - impl<T> Index<usize> for CircularBuffer<T>
//       due to associated trait bound <Output = T> not being considered:
//       - impl<T> SliceIndex<[T], Output = T> for usize 
//       - impl<T> Index<I> where I: SliceIndex<[T]>
//       because of:
//       - inability to declare non-overlapping blanket implementations
//       - https://github.com/rust-lang/rust/issues/20400
//       therefore:
//       - we implement them using a macro explicitly
macro_rules! impl_slice_index {
    ($I:ty) => {
        impl<T> std::ops::Index<$I> for CircularBuffer<T> {
            type Output = [T];
            fn index(&self, index: $I) -> &Self::Output {
                let slice = unsafe { &*std::ptr::slice_from_raw_parts(self.ptr.as_ptr(), 2*self.len) };
                let slice = &slice[index];
                let end_index = slice.len().min(self.len);
                let slice = &slice[..end_index];
                slice
            }
        }

        impl<T> std::ops::IndexMut<$I> for CircularBuffer<T> {
            fn index_mut(&mut self, index: $I) -> &mut [T] {
                let slice = unsafe { &mut *std::ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), 2*self.len) };
                let slice = &mut slice[index];
                let end_index = slice.len().min(self.len);
                let slice = &mut slice[..end_index];
                slice
            }
        }
    }
}
impl_slice_index!(std::ops::Range::<usize>);
impl_slice_index!(std::ops::RangeFrom::<usize>);
impl_slice_index!(std::ops::RangeFull);
impl_slice_index!(std::ops::RangeInclusive::<usize>);
impl_slice_index!(std::ops::RangeTo::<usize>);
impl_slice_index!(std::ops::RangeToInclusive::<usize>);

impl<T> std::ops::Index<usize> for CircularBuffer<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < (self.len*2));
        unsafe { &*self.ptr.as_ptr().wrapping_add(index) }
    }
}

impl<T> std::ops::IndexMut<usize> for CircularBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(index < (self.len*2));
        unsafe { &mut *self.ptr.as_ptr().wrapping_add(index) }
    }
}
