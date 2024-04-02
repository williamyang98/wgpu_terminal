pub mod process;
pub mod master_pty;

#[cfg(test)]
mod test {
    use crate::process::PtyProcess;
    use std::process::Command;
    use test_log::test;
    use std::io::{Read,Write};
    use cgmath::Vector2;

    fn assert_value<T: PartialEq + std::fmt::Debug>(given: T, expected: T) {
        if given != expected {
            panic!("given: {:?}, expected: {:?}", given, expected);
        }
    }

    fn assert_string(given: &[u8], expected: &[u8]) {
        if given != expected {
            panic!("given: {:?}, expected: {:?}", 
                std::str::from_utf8(given), 
                std::str::from_utf8(expected),
            );
        }
    }

    #[test]
    fn simple_spawn() {
        let mut command = Command::new("/usr/bin/echo");
        command.arg("hello world");
        let mut process = PtyProcess::spawn(command, None).unwrap();
        let mut master_pty = process.get_master_pty().try_clone().unwrap();
        assert!(master_pty.is_terminal());

        let status = process.try_wait().unwrap();
        log::info!("process exit status: {:?}", status);

        let mut read_buffer = vec![0u8; 1024];
        let total_read = master_pty.read(read_buffer.as_mut_slice()).unwrap();
        let read_data = &read_buffer[..total_read];
        assert_string(read_data, b"hello world\r\n");

        let status = process.wait().unwrap();
        assert!(status.success());
        assert_value(status.code(), Some(0i32));
    }

    #[test]
    fn simple_sh_shell() {
        let command = Command::new("/usr/bin/sh");
        let mut process = PtyProcess::spawn(command, None).unwrap();
        let mut master_pty = process.get_master_pty().try_clone().unwrap();
        assert!(master_pty.is_terminal());

        let status = process.try_wait().unwrap();
        assert_value(status, None);

        let mut read_buffer = vec![0u8; 1024];
        let total_read = master_pty.read(read_buffer.as_mut_slice()).unwrap();
        let read_data = &read_buffer[..total_read];
        log::info!("process read buffer: {:?}", std::str::from_utf8(read_data));
        master_pty.write_all(b"exit\x0d").unwrap();

        let status = process.wait().unwrap();
        assert!(status.success());
        assert_value(status.code(), Some(0i32));
        assert!(!read_data.is_empty());
    }

    #[test]
    fn simple_set_window_size() {
        let command = Command::new("/usr/bin/sh");
        let mut process = PtyProcess::spawn(command, None).unwrap();
        let mut master_pty = process.get_master_pty().try_clone().unwrap();
        assert!(master_pty.is_terminal());

        let status = process.try_wait().unwrap();
        assert_value(status, None);
 
        let read_thread = std::thread::spawn({
            let mut master_pty = master_pty.try_clone().unwrap();
            move || {
                let mut read_data = Vec::new();
                let mut read_buffer = vec![0u8; 1024];
                loop {
                    match master_pty.read(read_buffer.as_mut_slice()) {
                        Ok(0) => break,
                        Ok(total) => read_data.extend_from_slice(&read_buffer[..total]),
                        Err(err) => {
                            log::info!("Read pipe closed with: {:?}", err);
                            break;
                        },
                    }
                }
                read_data
            }
        });
        // set window size
        let size = Vector2::new(32,16);
        master_pty.set_window_size(size).unwrap();
        assert_value(master_pty.get_window_size(), Ok(size));
        // close shell
        master_pty.write_all(b"exit\x0d").unwrap();
        let status = process.wait().unwrap();
        assert!(status.success());
        assert_value(status.code(), Some(0i32));
        // make sure read thread got data
        let read_data = read_thread.join().unwrap();
        log::info!("process read buffer: {:?}", std::str::from_utf8(read_data.as_slice()));
        assert!(!read_data.is_empty());
    }
}
