mod circular_buffer;
pub use circular_buffer::CircularBuffer;

#[cfg(windows)]
mod win32;
#[cfg(windows)]
pub use win32::{
    CreateAlignError,
    CreateError,
    get_allocation_granularity,
};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{
    CreateAlignError,
    CreateError,
    get_allocation_granularity,
};

#[cfg(test)]
mod test {
    use crate::{
        CircularBuffer, 
        CreateError,
        CreateAlignError,
        get_allocation_granularity,
    };
    use test_log::test;

    #[test]
    fn valid_create() {
        let block_size = get_allocation_granularity();
        let size = block_size*4;
        let buffer = CircularBuffer::<u8>::new(size).unwrap();
        assert!(buffer.len() == size);
    }

    #[test]
    fn valid_create_default() {
        let block_size = get_allocation_granularity();
        let size = block_size*4;
        let buffer = CircularBuffer::<u8>::new(size).unwrap();
        let is_all_default = buffer.as_slice().iter().all(|v| *v == u8::default());
        assert!(is_all_default);
    }

    #[test]
    fn valid_drop() {
        let block_size = get_allocation_granularity();
        let size = block_size*4;
        let buffer = CircularBuffer::<u8>::new(size).unwrap();
        drop(buffer);
    }

    #[test]
    fn valid_get_slice() {
        let block_size = get_allocation_granularity();
        let size = block_size*4;
        let mut buffer = CircularBuffer::<u8>::new(size).unwrap();
        assert!(buffer.as_slice().len() == size);
        assert!(buffer.as_mut_slice().len() == size);
    }

    #[test]
    fn valid_clone() {
        let block_size = get_allocation_granularity();
        let mut buffer_0 = CircularBuffer::<u8>::new(block_size*10).unwrap();
        for (i, b) in buffer_0.as_mut_slice().iter_mut().enumerate() {
            *b = i as u8; 
        }
        let buffer_1 = buffer_0.clone(); 
        assert!(buffer_0.len() == buffer_1.len());
        assert!(buffer_0.as_slice() == buffer_1.as_slice());

        for (i, b) in buffer_0.as_mut_slice().iter_mut().enumerate() {
            *b = (i+10) as u8; 
        }
        assert!(buffer_0.as_slice() != buffer_1.as_slice());
    }

    #[test]
    fn valid_wrapped_slice_write() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let write_amount = block_size*2;
        for (i, b) in &mut buffer[(size+offset)..(size+offset+write_amount)].iter_mut().enumerate() {
            *b = i as u8;
        }

        let is_all_equal = &buffer[offset..(offset+write_amount)]
            .iter()
            .enumerate()
            .all(|(i,b)| *b == i as u8);
        assert!(is_all_equal);
    }

    #[test]
    fn valid_partial_overhang_slice_write() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let write_amount = block_size*3;
        for (i, b) in &mut buffer[(size-offset)..(size-offset+write_amount)].iter_mut().enumerate() {
            *b = i as u8;
        }
        let is_all_equal = buffer[(size-offset)..size]
            .iter()
            .enumerate()
            .all(|(i,b)| *b == i as u8);
        assert!(is_all_equal);
        let is_all_equal = buffer[..(write_amount-offset)]
            .iter()
            .enumerate()
            .all(|(i,b)| *b == ((i + offset) as u8));
        assert!(is_all_equal);
    }

    #[test]
    fn valid_wrapped_slice_read() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let write_amount = block_size*2;
        for (i, b) in &mut buffer[offset..(offset+write_amount)].iter_mut().enumerate() {
            *b = i as u8;
        }
        let is_all_equal = &buffer[(size+offset)..(size+offset+write_amount)]
            .iter()
            .enumerate()
            .all(|(i,b)| *b == i as u8);
        assert!(is_all_equal);
    }

    #[test]
    fn valid_partial_overhang_slice_read() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let write_amount = block_size*3;
        for (i,b) in &mut buffer[(size-offset)..size].iter_mut().enumerate() {
            *b = i as u8;
        }
        for (i,b) in &mut buffer[..(write_amount-offset)].iter_mut().enumerate() {
            *b = (i + offset) as u8;
        }
        let is_all_equal = buffer[(size-offset)..(size-offset+write_amount)]
            .iter_mut()
            .enumerate()
            .all(|(i,b)| *b == i as u8);
        assert!(is_all_equal);
    }

    #[test]
    fn valid_wrapped_single_write() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let test_value = 0x11;
        buffer[size+offset] = test_value;
        assert!(buffer[offset] == test_value);
    }

    #[test]
    fn valid_wrapped_single_read() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let test_value = 0x11;
        buffer[offset] = test_value;
        assert!(buffer[size+offset] == test_value);
    }

    #[test]
    fn valid_length_of_overhang_slice() {
        let block_size = get_allocation_granularity();
        let mut buffer = CircularBuffer::<u8>::new(block_size*10).unwrap();
        let size = buffer.len();
        let offset = block_size;
        let slice = &buffer[(size-offset)..];
        assert!(slice.len() <= size);
        let slice = &mut buffer[(size-offset)..];
        assert!(slice.len() <= size);
    }

    #[test]
    fn invalid_type_alignment() {
        let allocation_granularity = get_allocation_granularity();
        const ELEMENT_SIZE: usize = 13;
        type T = [u8; ELEMENT_SIZE];
        let size = 1024;
        let buffer = CircularBuffer::<T>::new(size);
        if let Err(err) = buffer {
            let size_bytes = ELEMENT_SIZE * size;
            let allocation_multiple = (size_bytes / allocation_granularity).max(1);
            let size_bytes = allocation_multiple * allocation_granularity;
            let total_elements = size_bytes / ELEMENT_SIZE;
            let error = CreateAlignError {
                element_size: ELEMENT_SIZE,
                total_elements,
                size_bytes,
                allocation_granularity,
            };
            assert!(err == CreateError::AlignmentError(error));
        } else {
            panic!("Circular buffer should have failed creation");
        }
    }

    #[test]
    fn valid_create_with_badly_aligned_type() {
        let allocation_granularity = get_allocation_granularity();
        const ELEMENT_SIZE: usize = 13;
        type T = [u8; ELEMENT_SIZE];
        let size = allocation_granularity * ELEMENT_SIZE;
        let buffer = CircularBuffer::<T>::new(size).unwrap();
        assert!(buffer.len() == size);
    }

    #[derive(Clone,Copy,Debug,PartialEq,Eq)]
    struct ComplexType { // pack into 32 bytes
        a: usize,   // 8
        b: u32,     // 4
        c: i32,     // 4
        d: [u8; 3], // 3
        e: u8,      // 1
        f: i64,     // 8
        g: char,    // 4
    }

    impl ComplexType {
        fn new(index: usize) -> Self {
            Self {
                a: index,
                b: (index as u32) / 4,
                c: (index as i32) - (index*10) as i32,
                d: [index as u8, (index+2) as u8, (index+15) as u8],
                e: ((index + 10) % 255) as u8,
                f: (index as i64) - 1024i64,
                g: char::from_u32(index as u32).unwrap_or('😩'),
            }
        }
    }

    impl Default for ComplexType {
        fn default() -> Self {
            Self {
                a: 1024,
                b: 68,
                c: -20,
                d: [0, 4, 13],
                e: 24,
                f: -123219,
                g: '😩',
            }

        }
    }

    #[test]
    fn valid_complex_type_create() {
        let buffer = CircularBuffer::<ComplexType>::new(1024).unwrap();
        let size = buffer.len();
        assert!(size > 0);
    }

    #[test]
    fn valid_complex_type_with_default() {
        let buffer = CircularBuffer::<ComplexType>::new(1024).unwrap();
        let default_value = ComplexType::default();
        let is_all_default = buffer.as_slice().iter().all(move |v| *v == default_value);
        assert!(is_all_default);
    }

    #[test]
    fn valid_complex_type_write_overhang() {
        let mut buffer = CircularBuffer::<ComplexType>::new(1024).unwrap();
        let size = buffer.len();
        let offset = 20;
        let total = 50;
        assert!(size > offset);
        for (i,b) in &mut buffer[(size-offset)..(size-offset+total)].iter_mut().enumerate() {
            *b = ComplexType::new(i);
        }
        // check we have written to it
        let is_all_equal = &buffer[(size-offset)..size]
            .iter()
            .enumerate()
            .all(|(i,b)| {
                *b == ComplexType::new(i)
            });
        assert!(is_all_equal);
        let is_all_equal = &buffer[..(total-offset)]
            .iter()
            .enumerate()
            .all(|(i,b)| {
                *b == ComplexType::new(i+offset)
            });
        assert!(is_all_equal);
        // check non written areas are still at default value
        let is_all_default_equal = &buffer[(total-offset)..(size-offset)]
            .iter()
            .all(|b| {
                *b == ComplexType::default()
            });
        assert!(is_all_default_equal);
    }

    #[derive(Clone,Copy,Debug,PartialEq,Eq)]
    struct BadlyAlignedType {
        data: [u8; 13],
    }

    impl BadlyAlignedType {
        fn new(index: usize) -> Self {
            let mut data = Self::default();
            for (i,b) in data.data.iter_mut().enumerate() {
                *b = (2*i + index) as u8;
            }
            data
        }
    }

    impl Default for BadlyAlignedType {
        fn default() -> Self {
            Self {
                data: [
                    1,2,126,5,
                    1,5,1,23,
                    40,0,128,252,
                    255,
                ],
            }
        }
    }

    #[test]
    fn valid_badly_aligned_type_create() {
        let allocation_granularity = get_allocation_granularity();
        let element_size = std::mem::size_of::<BadlyAlignedType>();
        assert!(element_size == 13);
        let size = allocation_granularity * element_size; // dirty way to get wrapping alignment
        let buffer = CircularBuffer::<BadlyAlignedType>::new(size).unwrap();
        assert!(buffer.len() == size);
    }

    #[test]
    fn valid_badly_aligned_type_with_default() {
        let allocation_granularity = get_allocation_granularity();
        let size = allocation_granularity * std::mem::size_of::<BadlyAlignedType>();
        let buffer = CircularBuffer::<BadlyAlignedType>::new(size).unwrap();
        let default_value = BadlyAlignedType::default();
        let is_all_default = buffer.as_slice().iter().all(move |v| *v == default_value);
        assert!(is_all_default);
    }

    #[test]
    fn valid_badly_aligned_type_write_overhang() {
        let allocation_granularity = get_allocation_granularity();
        let size = allocation_granularity * std::mem::size_of::<BadlyAlignedType>();
        let mut buffer = CircularBuffer::<BadlyAlignedType>::new(size).unwrap();
        let offset = 20;
        let total = 50;
        assert!(size > offset);
        for (i,b) in &mut buffer[(size-offset)..(size-offset+total)].iter_mut().enumerate() {
            *b = BadlyAlignedType::new(i);
        }
        // check we have written to it
        let is_all_equal = &buffer[(size-offset)..size]
            .iter()
            .enumerate()
            .all(|(i,b)| {
                *b == BadlyAlignedType::new(i)
            });
        assert!(is_all_equal);
        let is_all_equal = &buffer[..(total-offset)]
            .iter()
            .enumerate()
            .all(|(i,b)| {
                *b == BadlyAlignedType::new(i+offset)
            });
        assert!(is_all_equal);
        // check non written areas are still at default value
        let is_all_default_equal = &buffer[(total-offset)..(size-offset)]
            .iter()
            .all(|b| {
                *b == BadlyAlignedType::default()
            });
        assert!(is_all_default_equal);
    }
}
