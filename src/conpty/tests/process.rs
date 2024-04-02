#![cfg(windows)]
use std::io::{Read,Write};
use test_log::test;
use conpty::process::{ConptyProcess, SpawnError, Size as ProcessSize};
use std::process::Command;

fn assert_exit_code(given: u32, expected: u32) {
    if given != expected {
        log::error!("exit code expected to be {} but given {}", expected, given);
    }
    assert!(given == expected);
}

#[test]
fn spawn_process() {
    let command = Command::new("cmd.exe");
    let process = ConptyProcess::spawn(command, None).unwrap();
    assert!(process.is_alive());

    let mut write_pipe = process.get_write_pipe().try_clone().unwrap();
    let mut read_pipe = process.get_read_pipe().try_clone().unwrap();
    let read_thread = std::thread::spawn(move || {
        let mut read_data = Vec::new();
        let mut read_buffer = vec![0u8; 1024];
        loop {
            match read_pipe.read(read_buffer.as_mut_slice()) {
                Ok(0) => {
                    log::info!("Read pipe closing on 0 byte read");
                    break;
                },
                Ok(total) => read_data.extend_from_slice(&read_buffer[..total]),
                Err(err) => {
                    log::info!("Read pipe closing on error: {:?}", err);
                    break;
                },
            }
        }
        read_data
    });
    write_pipe.write_all(b"echo hello world\x0d").unwrap();
    write_pipe.write_all(b"exit\x0d").unwrap();
    let exit_code = process.wait(None).unwrap();
    assert!(!process.is_alive());
    drop(process);

    let read_buffer = read_thread.join().unwrap();
    log::info!("process.read_pipe: {:?}", std::str::from_utf8(read_buffer.as_slice()));
    assert_exit_code(exit_code, 0);
    assert!(!read_buffer.is_empty());
}

#[test]
fn spawn_invalid_process() {
    let command = Command::new("some_random_madeup_command_that_doesnt_exist");
    let error = ConptyProcess::spawn(command, None).err().expect("Spawn should fail with error");
    let SpawnError::FailedCreateProcess(error) = error else {
        panic!("Expected error when creating process not: {:?}", error);
    };
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND as EXPECTED_ERROR;
    let given_code = error.code();
    let expected_code = EXPECTED_ERROR.to_hresult();
    if given_code != expected_code {
        log::error!("given_code: {:?} differs from expected_code: {:?} for error: {:?}", given_code, expected_code, error); 
    }
    assert!(given_code == expected_code);
}

#[test]
fn resize_console() {
    let command = Command::new("cmd.exe");
    let mut process = ConptyProcess::spawn(command, None).unwrap();
    assert!(process.is_alive());

    let mut write_pipe = process.get_write_pipe().try_clone().unwrap();
    let mut read_pipe = process.get_read_pipe().try_clone().unwrap();
    let read_thread = std::thread::spawn(move || {
        let mut read_data = Vec::new();
        let mut read_buffer = vec![0u8; 1024];
        loop {
            match read_pipe.read(read_buffer.as_mut_slice()) {
                Ok(0) => {
                    log::info!("Read pipe closing on 0 byte read");
                    break;
                },
                Ok(total) => read_data.extend_from_slice(&read_buffer[..total]),
                Err(err) => {
                    log::info!("Read pipe closing on error: {:?}", err);
                    break;
                },
            }
        }
        read_data
    });
    write_pipe.write_all(b"dir\x0d").unwrap();
 
    let new_size = ProcessSize::new(32,8);
    process.set_size(new_size).expect("Resize should have worked correctly");
    assert!(process.get_size() == new_size);

    write_pipe.write_all(b"exit\x0d").unwrap();
    let exit_code = process.wait(None).unwrap();
    assert!(!process.is_alive());
    drop(process);

    let read_buffer = read_thread.join().unwrap();
    log::info!("process.read_pipe: {:?}", std::str::from_utf8(read_buffer.as_slice()));
    assert_exit_code(exit_code, 0);
    assert!(!read_buffer.is_empty());
}
