use windows::Win32::{
    System::SystemInformation::{
        SYSTEM_INFO,
        GetSystemInfo,
    },
    System::Memory::{
        MEM_RESERVE, MEM_RESERVE_PLACEHOLDER, MEM_REPLACE_PLACEHOLDER,
        MEM_RELEASE, MEM_PRESERVE_PLACEHOLDER, VIRTUAL_FREE_TYPE,
        PAGE_NOACCESS, PAGE_READWRITE,
        MEMORY_MAPPED_VIEW_ADDRESS,
        VirtualAlloc2, VirtualFree, CreateFileMappingA, MapViewOfFile3, UnmapViewOfFile,
    },
    Foundation::{
        HANDLE, INVALID_HANDLE_VALUE, CloseHandle, GetLastError,
    },
};
use windows::core::{
    PCSTR,
    Error as WinError,
};
use core::ffi::{
    c_void,
};

#[derive(Debug)]
pub struct CircularBuffer<T> {
    ptr: *mut T,
    len: usize,
}

unsafe impl<T> Send for CircularBuffer<T> {}

impl<T> CircularBuffer<T> {
    // Cannot allocate a circular buffer without virtual memory backing it
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { &*std::ptr::slice_from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *std::ptr::slice_from_raw_parts_mut(self.ptr, self.len) }
    }

    pub fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
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
                let slice = unsafe { &*std::ptr::slice_from_raw_parts(self.ptr, 2*self.len) };
                let slice = &slice[index];
                let end_index = slice.len().min(self.len);
                let slice = &slice[..end_index];
                slice
            }
        }

        impl<T> std::ops::IndexMut<$I> for CircularBuffer<T> {
            fn index_mut(&mut self, index: $I) -> &mut [T] {
                let slice = unsafe { &mut *std::ptr::slice_from_raw_parts_mut(self.ptr, 2*self.len) };
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
        unsafe { &*self.ptr.wrapping_add(index) }
    }
}

impl<T> std::ops::IndexMut<usize> for CircularBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(index < (self.len*2));
        unsafe { &mut *self.ptr.wrapping_add(index) }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CreateAlignError {
    pub element_size: usize, 
    pub total_elements: usize, 
    pub size_bytes: usize, 
    pub allocation_granularity: usize,
}

// create circular buffer
#[derive(Clone,Debug,PartialEq)]
pub enum CreateError {
    AlignmentError(CreateAlignError),
    FailedVirtualAlloc(Option<WinError>),
    FailedVirtualSplit(WinError),
    FailedCreateFileMapping(WinError),
    InvalidFileMapping(Option<WinError>),
    FailedMapView(usize),
    NonAdjacentViews,
}

struct CreateContext {
    virtual_address_0: *mut c_void,
    virtual_address_1: *mut c_void,
    file_view: HANDLE,
    view_address_0: MEMORY_MAPPED_VIEW_ADDRESS,
    view_address_1: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl Default for CreateContext {
    fn default() -> Self {
        Self {
            virtual_address_0: std::ptr::null_mut(),
            virtual_address_1: std::ptr::null_mut(),
            file_view: INVALID_HANDLE_VALUE,
            view_address_0: MEMORY_MAPPED_VIEW_ADDRESS { Value: std::ptr::null_mut() },
            view_address_1: MEMORY_MAPPED_VIEW_ADDRESS { Value: std::ptr::null_mut() },
        }
    }
}

impl Drop for CreateContext {
    fn drop(&mut self) {
        if !self.view_address_1.Value.is_null() {
            if let Err(err) = unsafe { UnmapViewOfFile(self.view_address_1) } {
                log::error!("UnmapViewOfFile(self.view_address_1) failed with: {:?}", err);
            }
        }
        if !self.view_address_0.Value.is_null() {
            if let Err(err) = unsafe { UnmapViewOfFile(self.view_address_0) } {
                log::error!("UnmapViewOfFile(self.view_address_0) failed with: {:?}", err);
            }
        }
        if self.file_view != INVALID_HANDLE_VALUE {
            if let Err(err) =  unsafe { CloseHandle(self.file_view) } {
                log::error!("CloseHandle(self.file_view) failed with: {:?}", err);
            }
        }
        if !self.virtual_address_1.is_null() {
            if let Err(err) = unsafe { VirtualFree(self.virtual_address_1, 0, MEM_RELEASE) } {
                log::error!("VirtualFree(self.virtual_address_1) failed with: {:?}", err);
            }
        }
        if !self.virtual_address_0.is_null() {
            if let Err(err) = unsafe { VirtualFree(self.virtual_address_0, 0, MEM_RELEASE) } {
                log::error!("VirtualFree(self.virtual_address_0) failed with: {:?}", err);
            }
        }
    }
}

pub fn get_allocation_granularity() -> usize {
    // determine size that is aligned to page size and allocation granularity
    let mut system_info = SYSTEM_INFO::default();
    unsafe {
        GetSystemInfo(&mut system_info);
    }
    let page_size = system_info.dwPageSize as usize;
    let allocation_granularity = system_info.dwAllocationGranularity as usize;
    if allocation_granularity % page_size != 0 {
        panic!("Allocation granularity ({}) must be a multiple of page size ({})", 
            allocation_granularity, page_size);
    }
    allocation_granularity
}

impl<T> CircularBuffer<T> 
where T: Sized + Clone + Default
{
    #[allow(clippy::field_reassign_with_default)]
    pub fn new(total_elements: usize) -> Result<Self, CreateError> {
        let allocation_granularity = get_allocation_granularity();
        let element_size = std::mem::size_of::<T>();
        let size_bytes = element_size * total_elements;
        // determine number of blocks to allocate
        // Source: https://devblogs.microsoft.com/oldnewthing/20031008-00/?p=42223
        //         We need to force the block size to be aligned to the allocation granularity
        //         This is because VirtualAlloc allocates to 64kB boundaries instead of 4kB boundaries
        let allocation_multiple = (size_bytes / allocation_granularity).max(1);
        let size_bytes = allocation_multiple * allocation_granularity;
        // determine if resulting allocation will wrap nicely to align with type size
        let total_elements = size_bytes / element_size;
        if size_bytes % element_size != 0 {
            let error = CreateAlignError { element_size, total_elements, size_bytes, allocation_granularity };
            return Err(CreateError::AlignmentError(error));
        }
        assert!(size_bytes % total_elements == 0);
        assert!(size_bytes > 0);
        assert!(total_elements > 0);
 
        let mut context = CreateContext::default();
        context.virtual_address_0 = unsafe {
            let process = HANDLE(0);
            let base_address = None;
            let virtual_size = size_bytes*2; // two memory mapped regions side by side
            let allocation_type = MEM_RESERVE | MEM_RESERVE_PLACEHOLDER;
            let page_protection = PAGE_NOACCESS;
            let extended_parameters = None;
            VirtualAlloc2(
                process, base_address, virtual_size,
                allocation_type, page_protection.0, extended_parameters,
            )
        };

        if context.virtual_address_0.is_null() {
            let err = unsafe { GetLastError().err() };
            return Err(CreateError::FailedVirtualAlloc(err));
        }
 
        // Source: https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualfree
        // split region into two for mapping 
        unsafe {
            let free_type = VIRTUAL_FREE_TYPE(MEM_RELEASE.0 | MEM_PRESERVE_PLACEHOLDER.0);
            VirtualFree(context.virtual_address_0, size_bytes, free_type)
                .map_err(CreateError::FailedVirtualSplit)?;
        }
        context.virtual_address_1 = context.virtual_address_0.wrapping_byte_add(size_bytes);

        // Create file views
        context.file_view = unsafe {
            let file_handle = INVALID_HANDLE_VALUE;
            let mapping_attributes = None;
            let page_protection = PAGE_READWRITE;
            let minimum_size: u32 = 0;
            let maximum_size: u32 = size_bytes as u32;
            let file_name = PCSTR::null();
            CreateFileMappingA(
                file_handle,
                mapping_attributes,
                page_protection,
                minimum_size,
                maximum_size,
                file_name,
            ).map_err(CreateError::FailedCreateFileMapping)?
        };

        if context.file_view == HANDLE(0) {
            let err = unsafe { GetLastError().err() };
            return Err(CreateError::InvalidFileMapping(err));
        }

        // Transfer ownership of virtual memory pages to view 0
        context.view_address_0 = unsafe {
            let process = HANDLE(0);
            let base_address = Some(context.virtual_address_0 as *const c_void);
            let offset: u64 = 0;
            let view_size = size_bytes;
            let allocation_type = MEM_REPLACE_PLACEHOLDER;
            let page_protection = PAGE_READWRITE;
            let extended_parameters = None;
            MapViewOfFile3(
                context.file_view, process,
                base_address, offset, view_size,
                allocation_type, page_protection.0,
                extended_parameters,
            )
        };
        if context.view_address_0.Value.is_null() {
            return Err(CreateError::FailedMapView(0));
        }
        context.virtual_address_0 = std::ptr::null_mut();

        // Transfer ownership of virtual memory pages to view 1
        context.view_address_1 = unsafe {
            let process = HANDLE(0);
            let base_address = Some(context.virtual_address_1 as *const c_void);
            let offset: u64 = 0;
            let view_size = size_bytes;
            let allocation_type = MEM_REPLACE_PLACEHOLDER;
            let page_protection = PAGE_READWRITE;
            let extended_parameters = None;
            MapViewOfFile3(
                context.file_view, process,
                base_address, offset, view_size,
                allocation_type, page_protection.0,
                extended_parameters,
            )
        };
        if context.view_address_1.Value.is_null() {
            return Err(CreateError::FailedMapView(1));
        }
        context.virtual_address_1 = std::ptr::null_mut();

        // Guarantee that views are adjacent
        if context.view_address_0.Value.wrapping_byte_add(size_bytes) != context.view_address_1.Value {
            return Err(CreateError::NonAdjacentViews);
        }

        // Views now point to circular buffer
        let buffer = context.view_address_0.Value as *mut T;
        context.view_address_0.Value = std::ptr::null_mut();
        context.view_address_1.Value = std::ptr::null_mut();

        let mut res = Self {
            ptr: buffer,
            len: total_elements,
        };
        // uninitialised memory is considered unsafe in rust
        res.as_mut_slice().fill(T::default());
        Ok(res)
    }
}

impl<T> Drop for CircularBuffer<T> 
where T: Sized
{
    fn drop(&mut self) {
        let view_address_0 = self.ptr as *mut c_void;
        let view_address_1 = self.ptr.wrapping_add(self.len) as *mut c_void;
        let view_address_0 = MEMORY_MAPPED_VIEW_ADDRESS { Value: view_address_0 };
        let view_address_1 = MEMORY_MAPPED_VIEW_ADDRESS { Value: view_address_1 };
        if let Err(err) = unsafe { UnmapViewOfFile(view_address_0) } {
            log::error!("UnmapViewOfFile(self.view_address_0) failed with: {:?}", err);
        }
        if let Err(err) = unsafe { UnmapViewOfFile(view_address_1) } {
            log::error!("UnmapViewOfFile(self.view_address_1) failed with: {:?}", err);
        }
    }
}
