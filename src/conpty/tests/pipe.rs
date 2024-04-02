#![cfg(windows)]
use std::io::{Read,Write};
use test_log::test;
use conpty::pipe::create_pipe;

const DEFAULT_BUFFER_SIZE: usize = 1024;

fn hash_index(i: usize) -> u8 {
    let d = i ^ (i >> 2) ^ (i >> 4);
    d as u8
}

#[test]
fn simple_create_pipe() {
    let (read_pipe, write_pipe) = create_pipe(None).unwrap();
    drop(read_pipe);
    drop(write_pipe);
}

#[test]
fn simple_read_write_pipe() {
    let (mut read_pipe, mut write_pipe) = create_pipe(None).unwrap();
    let mut write_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    let mut read_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    write_buffer.iter_mut().enumerate().for_each(|(i,v)| *v = hash_index(i));
    write_pipe.write_all(write_buffer.as_slice()).unwrap();
    read_pipe.read_exact(read_buffer.as_mut_slice()).unwrap();
    assert!(write_buffer == read_buffer);
}

#[test]
fn clone_write_pipe() {
    let (mut read_pipe, write_pipe) = create_pipe(Some(DEFAULT_BUFFER_SIZE as u32)).unwrap();
    let total_pipes = 8;
    let total_size = DEFAULT_BUFFER_SIZE*total_pipes;
    let mut write_buffer = vec![0u8; total_size];
    let mut read_buffer = vec![0u8; total_size];

    let mut write_pipes = Vec::new();
    for _ in 0..(total_pipes-1) {
        let cloned_pipe = write_pipe.try_clone().unwrap();
        write_pipes.push(cloned_pipe)
    }
    write_pipes.push(write_pipe);

    let write_thread = std::thread::spawn(move || {
        write_buffer.iter_mut().enumerate().for_each(|(i,v)| *v = hash_index(i));
        for (data, write_pipe) in write_buffer.as_slice().chunks(DEFAULT_BUFFER_SIZE).zip(write_pipes.iter_mut()) {
            write_pipe.write_all(data).unwrap();
        }
    });
    read_pipe.read_exact(read_buffer.as_mut_slice()).unwrap();
    write_thread.join().unwrap();
    let is_equal = read_buffer.iter().enumerate().all(|(i,v)| *v == hash_index(i));
    assert!(is_equal);
}

#[test]
fn clone_read_pipe() {
    let (read_pipe, mut write_pipe) = create_pipe(Some(DEFAULT_BUFFER_SIZE as u32)).unwrap();
    let total_pipes = 8;
    let total_size = DEFAULT_BUFFER_SIZE*total_pipes;
    let mut write_buffer = vec![0u8; total_size];
    let mut read_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];

    let mut read_pipes = Vec::new();
    for _ in 0..(total_pipes-1) {
        let cloned_pipe = read_pipe.try_clone().unwrap();
        read_pipes.push(cloned_pipe)
    }
    read_pipes.push(read_pipe);

    let write_thread = std::thread::spawn(move || {
        write_buffer.iter_mut().enumerate().for_each(|(i,v)| *v = hash_index(i));
        write_pipe.write_all(write_buffer.as_slice()).unwrap();
    });

    for (i, read_pipe) in read_pipes.iter_mut().enumerate() {
        let offset = i*read_buffer.len();
        read_pipe.read_exact(read_buffer.as_mut_slice()).unwrap();
        let is_equal = read_buffer.iter().enumerate().all(|(i,v)| *v == hash_index(i+offset));
        assert!(is_equal);
    }
    write_thread.join().unwrap();
}

#[test]
fn clone_and_drop_read_pipe() {
    let (read_pipe_0, mut write_pipe) = create_pipe(None).unwrap();
    let mut read_pipe = read_pipe_0.try_clone().unwrap();
    drop(read_pipe_0);
    let mut write_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    let mut read_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    write_buffer.iter_mut().enumerate().for_each(|(i,v)| *v = hash_index(i));
    write_pipe.write_all(write_buffer.as_slice()).unwrap();
    read_pipe.read_exact(read_buffer.as_mut_slice()).unwrap();
    assert!(write_buffer == read_buffer);
}

#[test]
fn clone_and_drop_write_pipe() {
    let (mut read_pipe, write_pipe_0) = create_pipe(None).unwrap();
    let mut write_pipe = write_pipe_0.try_clone().unwrap();
    drop(write_pipe_0);
    let mut write_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    let mut read_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    write_buffer.iter_mut().enumerate().for_each(|(i,v)| *v = hash_index(i));
    write_pipe.write_all(write_buffer.as_slice()).unwrap();
    read_pipe.read_exact(read_buffer.as_mut_slice()).unwrap();
    assert!(write_buffer == read_buffer);
}

#[test]
fn drop_read_pipe_fail() {
    let (read_pipe, mut write_pipe) = create_pipe(None).unwrap();
    drop(read_pipe);
    let write_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    let error = write_pipe.write_all(write_buffer.as_slice()).err().unwrap();
    use windows::Win32::Foundation::ERROR_NO_DATA as EXPECTED_ERROR;
    let given_code = error.raw_os_error().unwrap();
    let expected_code = EXPECTED_ERROR.to_hresult().0;
    if given_code != expected_code {
        log::error!("given_code: {:?} differs from expected_code: {:?} for error: {:?}", given_code, expected_code, error); 
    }
    assert!(given_code == expected_code);
}

#[test]
fn drop_write_pipe_fail() {
    let (mut read_pipe, write_pipe) = create_pipe(None).unwrap();
    drop(write_pipe);
    let mut read_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
    let error = read_pipe.read_exact(read_buffer.as_mut_slice()).err().unwrap();
    use windows::Win32::Foundation::ERROR_BROKEN_PIPE as EXPECTED_ERROR;
    let given_code = error.raw_os_error().unwrap();
    let expected_code = EXPECTED_ERROR.to_hresult().0;
    if given_code != expected_code {
        log::error!("given_code: {:?} differs from expected_code: {:?} for error: {:?}", given_code, expected_code, error); 
    }
    assert!(given_code == expected_code);
}

#[test]
fn clone_write_and_read_pipe() {
    let (read_pipe, write_pipe) = create_pipe(Some(DEFAULT_BUFFER_SIZE as u32)).unwrap();
    let total_pipes = 8;

    let mut write_pipes = Vec::new();
    for _ in 0..(total_pipes-1) {
        let cloned_pipe = write_pipe.try_clone().unwrap();
        write_pipes.push(cloned_pipe)
    }
    write_pipes.push(write_pipe);

    let mut read_pipes = Vec::new();
    for _ in 0..(total_pipes-1) {
        let cloned_pipe = read_pipe.try_clone().unwrap();
        read_pipes.push(cloned_pipe)
    }
    read_pipes.push(read_pipe);

    let write_thread = std::thread::spawn(move || {
        let mut write_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
        for (i, write_pipe) in write_pipes.iter_mut().enumerate() {
            let offset = i*write_buffer.len();
            write_buffer.iter_mut().enumerate().for_each(|(i,v)| *v = hash_index(i+offset));
            write_pipe.write_all(write_buffer.as_slice()).unwrap();
        }
    });

    let read_thread = std::thread::spawn(move || {
        let mut read_buffer = vec![0u8; DEFAULT_BUFFER_SIZE];
        for (i, read_pipe) in read_pipes.iter_mut().enumerate() {
            let offset = i*read_buffer.len();
            read_pipe.read_exact(read_buffer.as_mut_slice()).unwrap();
            let is_equal = read_buffer.iter().enumerate().all(|(i,v)| *v == hash_index(i+offset));
            assert!(is_equal);
        }
    });

    write_thread.join().unwrap();
    read_thread.join().unwrap();
}
