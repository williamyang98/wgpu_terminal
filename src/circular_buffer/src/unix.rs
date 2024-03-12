use nix::errno::Errno;
use nix::sys::mman::{
    mmap_anonymous, mremap, munmap,
    MapFlags, MRemapFlags, ProtFlags,
};
use nix::unistd::{
    sysconf, 
    SysconfVar,
};
use core::ffi::c_void;
use core::num::NonZeroUsize;
use std::ptr::NonNull;
use crate::circular_buffer::CircularBuffer;

pub fn get_allocation_granularity() -> usize {
    let page_size = sysconf(SysconfVar::PAGE_SIZE).expect("Expected page size to be available at runtime");
    let page_size = page_size.expect("Page size must be defined to allocate circular buffer");
    if page_size <= 0 {
        panic!("Page size must be greater than 0 but got {}", page_size);
    }
    page_size as usize
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
    FailedMemoryMap(Errno),
    FailedMemoryRemap(Errno),
    NonAdjacentViews,
}

#[derive(Clone,Copy)]
struct VirtualMapping {
    address: NonNull<c_void>,
    size: usize,
}

#[derive(Default)]
struct CreateContext {
    view_address_0: Option<VirtualMapping>,
    view_address_1: Option<VirtualMapping>,
}

impl Drop for CreateContext {
    fn drop(&mut self) {
        if let Some(mapping) = self.view_address_1 {
            let res = unsafe { munmap(mapping.address, mapping.size) };
            if let Err(err) = res {
                log::error!("munmap(self.view_address_1) failed with {}", err);
            }
        }
        if let Some(mapping) = self.view_address_0 {
            let res = unsafe { munmap(mapping.address, mapping.size) };
            if let Err(err) = res {
                log::error!("munmap(self.view_address_0) failed with {}", err);
            }
        }
    }
}

impl<T> CircularBuffer<T> 
where T: Sized + Clone + Default
{
    pub fn new(total_elements: usize) -> Result<Self, CreateError> {
        let allocation_granularity = get_allocation_granularity();
        let element_size = std::mem::size_of::<T>();
        let size_bytes = element_size * total_elements;
        // determine number of blocks to allocate aligned to page size
        let allocation_multiple = (size_bytes / allocation_granularity).max(1);
        let size_bytes = allocation_multiple * allocation_granularity;
        // determine if resulting allocation will wrap nicely to align with type size
        let total_elements = size_bytes / element_size;
        if size_bytes % element_size != 0 {
            let error = CreateAlignError { element_size, total_elements, size_bytes, allocation_granularity };
            return Err(CreateError::AlignmentError(error));
        }
        assert!(size_bytes % allocation_granularity == 0);
        assert!(size_bytes % total_elements == 0);
        assert!(size_bytes > 0);
        assert!(total_elements > 0);

        let mut context = CreateContext::default();
        context.view_address_0 = unsafe {
            let address = None;
            let length = NonZeroUsize::new(size_bytes*2).unwrap();
            let protection_flags = ProtFlags::PROT_READ | ProtFlags::PROT_WRITE;
            let mapping_flags = MapFlags::MAP_SHARED | MapFlags::MAP_ANONYMOUS;
            let address = mmap_anonymous(
                address,
                length,
                protection_flags,
                mapping_flags,
            ).map_err(CreateError::FailedMemoryMap)?;
            Some(VirtualMapping {
                address,
                size: length.get(),
            })
        };

        // second half of virtual mapping should refer to first half of physical memory
        // this should let the second half of physical memory be freed
        context.view_address_1 = unsafe {
            let view_address_0 = context.view_address_0.unwrap();
            let old_address = view_address_0.address;
            // Source: https://man7.org/linux/man-pages/man2/mremap.2.html
            // if we specify old size as 0 it will create a new mapping
            // we also need to provide MREMAP_MAYMOVE when creating a new mapping
            // we provide MREMAP_FIXED to override the original mapping
            let old_size = 0;
            let new_size = size_bytes;
            let remapping_flags = MRemapFlags::MREMAP_MAYMOVE | MRemapFlags::MREMAP_FIXED;
            let new_address = old_address.as_ptr().wrapping_byte_add(size_bytes);
            let new_address = Some(NonNull::new(new_address).unwrap());
            let address = mremap(
                old_address,
                old_size,
                new_size,
                remapping_flags,
                new_address,
            ).map_err(CreateError::FailedMemoryRemap)?;
            Some(VirtualMapping {
                address,
                size: size_bytes,
            })
        };
        context.view_address_0.unwrap().size = size_bytes;
 
        // verify the virtual memory locations are actually adjacent
        let address_0 = context.view_address_0.unwrap().address.as_ptr();
        let address_1 = context.view_address_1.unwrap().address.as_ptr();
        if address_0.wrapping_byte_add(size_bytes) != address_1 {
            return Err(CreateError::NonAdjacentViews);
        }

        // transfer ownership to buffer
        context.view_address_0 = None;
        context.view_address_1 = None;
        let buffer = address_0 as *mut T;
        let buffer = unsafe { NonNull::new_unchecked(buffer) };
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
        let address = self.ptr.as_ptr() as *mut c_void;
        let address = unsafe { NonNull::new_unchecked(address) };
        let total_bytes = self.len()*std::mem::size_of::<T>();
        // unmap both halves of virtual memory mapping
        if let Err(err) = unsafe { munmap(address, total_bytes*2) } {
            log::error!("munmap(self.buffer) failed with: {:?}", err);
        }
    }
}
