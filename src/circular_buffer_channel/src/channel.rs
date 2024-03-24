use circular_buffer::{CircularBuffer, CreateError, get_allocation_granularity};
use std::sync::{Arc,Mutex,Condvar};

#[derive(Debug,PartialEq)]
pub struct Buffer<T> {
    data: CircularBuffer<T>,
    write_index: usize,
    read_index: usize,
    total_used: usize,
    total_senders: usize,
    total_receivers: usize,
}

impl<T> Buffer<T> {
    pub fn total_unused(&self) -> usize {
        self.data.len() - self.total_used
    }

    pub fn is_full(&self) -> bool {
        self.total_used == self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_used == 0
    }
}

#[derive(Debug)]
struct Core<T> {
    buffer: Mutex<Buffer<T>>,
    length: usize,
    signal_sender: Condvar,
    signal_receiver: Condvar,
}

#[derive(Debug)]
pub struct Sender<T>(Arc<Core<T>>);

#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum SendError {
    Closed,
    Poisoned,
}

impl<T: Sized + Copy> Sender<T> {
    pub fn send(&self, src_buf: &[T]) -> Result<usize, SendError> {
        let mut buffer = self.0.buffer.lock().map_err(|_| SendError::Poisoned)?;
        loop {
            if buffer.total_receivers == 0 {
                return Err(SendError::Closed);
            }
            if !buffer.is_full() {
                break;
            }
            buffer = self.0.signal_sender.wait(buffer).map_err(|_| SendError::Poisoned)?;
        }
        let total_send = src_buf.len().min(buffer.total_unused());
        let write_index = buffer.write_index;
        let dst_buf = &mut buffer.data[write_index..(write_index+total_send)];
        dst_buf.copy_from_slice(&src_buf[..total_send]);
        buffer.total_used += total_send;
        buffer.write_index += total_send;
        if buffer.write_index > self.0.length {
            buffer.write_index -= self.0.length;
        }
        self.0.signal_receiver.notify_one();
        Ok(total_send)
    }

    pub fn send_all(&self, mut src_buf: &[T]) -> Result<(), SendError> {
        while !src_buf.is_empty() {
            let total_send = self.send(src_buf)?;
            src_buf = &src_buf[total_send..];
        }
        Ok(())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        match self.0.buffer.lock() {
            Ok(ref mut buffer) => {
                buffer.total_senders -= 1;
                if buffer.total_senders == 0 {
                    self.0.signal_receiver.notify_all();
                }
            },
            Err(err) => log::error!("Failed to acquired channel: {:?}", err),
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut buffer = self.0.buffer.lock().unwrap();
        buffer.total_senders += 1;
        Self(self.0.clone())
    }
}

#[derive(Debug)]
pub struct Receiver<T>(Arc<Core<T>>);

#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum ReceiveError {
    Closed,
    Poisoned,
}

impl<T: Sized + Copy> Receiver<T> {
    pub fn receive(&self, dst_buf: &mut [T]) -> Result<usize, ReceiveError> {
        let mut buffer = self.0.buffer.lock().map_err(|_| ReceiveError::Poisoned)?;
        loop {
            if buffer.total_senders == 0 && buffer.is_empty() {
                return Err(ReceiveError::Closed);
            }
            if !buffer.is_empty() {
                break;
            }
            buffer = self.0.signal_receiver.wait(buffer).map_err(|_| ReceiveError::Poisoned)?;
        }
        let total_receive = dst_buf.len().min(buffer.total_used);
        let read_index = buffer.read_index;
        let src_buf = &buffer.data[read_index..(read_index+total_receive)];
        dst_buf[..total_receive].copy_from_slice(src_buf);
        buffer.total_used -= total_receive;
        buffer.read_index += total_receive;
        if buffer.read_index > self.0.length {
            buffer.read_index -= self.0.length;
        }
        self.0.signal_sender.notify_one();
        Ok(total_receive)
    }

    pub fn receive_all(&self, mut dst_buf: &mut [T]) -> Result<(), ReceiveError> {
        while !dst_buf.is_empty() {
            let total_receive = self.receive(dst_buf)?;
            dst_buf = &mut dst_buf[total_receive..];
        }
        Ok(())
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        match self.0.buffer.lock() {
            Ok(ref mut buffer) => {
                buffer.total_receivers -= 1;
                if buffer.total_receivers == 0 {
                    self.0.signal_sender.notify_all();
                }
            },
            Err(err) => log::error!("Failed to acquired buffer: {:?}", err),
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        let mut buffer = self.0.buffer.lock().unwrap();
        buffer.total_receivers += 1;
        Self(self.0.clone())
    }
}

fn greatest_common_denominator(a: usize, b: usize) -> usize {
    if b == 0 {
        return a;
    }
    greatest_common_denominator(b, a % b)
}

fn lowest_common_multiple(a: usize, b: usize) -> usize {
    // try to avoid overflow
    if a > b {
        a / greatest_common_denominator(a, b) * b
    } else {
        b / greatest_common_denominator(a, b) * a
    }
}

#[derive(Debug,Clone)]
pub struct Channel<T>(Arc<Core<T>>);

impl<T: Clone + Sized + Default> Channel<T> {
    pub fn new(minimum_size: usize) -> Result<Self, CreateError> {
        let allocation_granularity = get_allocation_granularity();
        let elem_size = std::mem::size_of::<T>();
        let channel_size_bytes = lowest_common_multiple(allocation_granularity, elem_size);
        let total_elements_aligned = channel_size_bytes / elem_size;
        let multiple = minimum_size.div_ceil(total_elements_aligned).max(1);
        let size = multiple * total_elements_aligned;
        let buffer = CircularBuffer::<T>::new(size)?;
        let length = buffer.len();
        let buffer = Buffer {
            data: buffer,
            write_index: 0,
            read_index: 0,
            total_used: 0,
            total_senders: 0,
            total_receivers: 0,
        };
        let core = Arc::new(Core {
            buffer: Mutex::new(buffer),
            length,
            signal_sender: Condvar::new(),
            signal_receiver: Condvar::new(),
        });
        Ok(Self(core))
    }
}

// Similar api to standard mpsc channel
pub fn channel<T: Clone + Sized + Default>(minimum_size: usize) -> Result<(Sender<T>, Receiver<T>), CreateError> {
    let channel = Channel::<T>::new(minimum_size)?;
    Ok((channel.create_sender(), channel.create_receiver()))
}

impl<T> Channel<T> {
    pub fn create_sender(&self) -> Sender<T> {
        let mut buffer = self.0.buffer.lock().unwrap();
        buffer.total_senders += 1;
        Sender(self.0.clone())
    }

    pub fn create_receiver(&self) -> Receiver<T> {
        let mut buffer = self.0.buffer.lock().unwrap();
        buffer.total_receivers += 1;
        Receiver(self.0.clone())
    }

    pub fn size(&self) -> usize {
        self.0.length
    }
}

impl<T> Sender<T> {
    pub fn get_channel(&self) -> Channel<T> {
        Channel(self.0.clone())
    }
}

impl<T> Receiver<T> {
    pub fn get_channel(&self) -> Channel<T> {
        Channel(self.0.clone())
    }
}
