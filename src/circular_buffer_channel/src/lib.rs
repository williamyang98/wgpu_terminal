mod channel;

pub use channel::{
    channel,
    Channel,
    Sender,
    Receiver,
    SendError,
    ReceiveError,
};

#[cfg(test)]
mod test {
    use crate::{Channel, SendError, ReceiveError};
    use test_log::test;

    #[test]
    fn simple() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total = channel.size();

        let mut tx_buf = vec![0u32; total];
        let mut rx_buf = vec![0u32; total];
        tx_buf.iter_mut().enumerate().for_each(|(i,v)| {
            *v = i as u32;
        });
        tx.send_all(tx_buf.as_slice()).unwrap();
        rx.receive_all(rx_buf.as_mut_slice()).unwrap();
        assert!(tx_buf.as_slice() == rx_buf.as_slice());
    }

    #[test]
    fn with_receiver_in_thread() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total = channel.size();

        let rx_thread = std::thread::spawn(move || {
            let mut rx_buf = vec![0u32; total];
            rx.receive_all(rx_buf.as_mut_slice()).unwrap();
            let is_equal = rx_buf.iter().enumerate().all(|(i,v)| {
                *v == i as u32
            });
            is_equal
        });

        let mut tx_buf = vec![0u32; total];
        tx_buf.iter_mut().enumerate().for_each(|(i,v)| {
            *v = i as u32;
        });
        tx.send_all(tx_buf.as_slice()).unwrap();

        let is_equal = rx_thread.join().unwrap();
        assert!(is_equal);
    }

    #[test]
    fn with_send_larger_than_buffer() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total = channel.size();
        let total = total*10;

        let rx_thread = std::thread::spawn(move || {
            let mut rx_buf = vec![0u32; total];
            rx.receive_all(rx_buf.as_mut_slice()).unwrap();
            let is_equal = rx_buf.iter().enumerate().all(|(i,v)| {
                *v == i as u32
            });
            is_equal
        });

        let mut tx_buf = vec![0u32; total];
        tx_buf.iter_mut().enumerate().for_each(|(i,v)| {
            *v = i as u32;
        });
        tx.send_all(tx_buf.as_slice()).unwrap();

        let is_equal = rx_thread.join().unwrap();
        assert!(is_equal);
    }

    #[test]
    fn close_on_no_receiver() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        drop(rx);
        let res = tx.send(&[10]);
        assert!(res == Err(SendError::Closed));
    }

    #[test]
    fn close_on_no_sender() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        drop(tx);
        let mut buffer: [u32;1] = [0u32;1];
        let res = rx.receive(buffer.as_mut_slice());
        assert!(res == Err(ReceiveError::Closed));
    }

    #[test]
    fn dangling_sender_and_receiver() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total = channel.size();
        drop(channel);

        let mut tx_buf = vec![0u32; total];
        let mut rx_buf = vec![0u32; total];
        tx_buf.iter_mut().enumerate().for_each(|(i,v)| {
            *v = i as u32;
        });
        tx.send_all(tx_buf.as_slice()).unwrap();
        rx.receive_all(rx_buf.as_mut_slice()).unwrap();
        assert!(tx_buf.as_slice() == rx_buf.as_slice());
    }

    #[test]
    fn multiple_coherent_sender() {
        // buffers received will be sent coherently
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total_send = channel.size();
        let total_senders = 32;
        let total_receive = total_send * total_senders;

        let rx_thread = std::thread::spawn(move || {
            let mut rx_buf = vec![0u32; total_receive];
            rx.receive_all(rx_buf.as_mut_slice()).unwrap();
            let is_equal = rx_buf.iter().enumerate().all(|(i,v)| {
                (*v as usize % total_send) == (i % total_send)
            });
            is_equal
        });

        let mut tx_threads = Vec::with_capacity(total_senders);
        for thread_id in 0..total_senders {
            let tx = tx.clone();
            let handle = std::thread::spawn(move || {
                let mut tx_buf = vec![0u32; total_send];
                let offset = thread_id*total_send;
                tx_buf.iter_mut().enumerate().for_each(move |(i,v)| {
                    *v = (i+offset) as u32;
                });
                tx.send_all(tx_buf.as_slice()).unwrap();
            });
            tx_threads.push(Some(handle));
        }
 
        let is_equal = rx_thread.join().unwrap();
        assert!(is_equal);
        tx_threads.iter_mut().for_each(|thread| thread.take().unwrap().join().unwrap());
    }

    #[test]
    fn multiple_coherent_receiver() {
        // buffer sent will be received coherently
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total_receive = channel.size();
        let total_receivers = 32;
        let total_send = total_receive * total_receivers;

        let mut rx_threads = Vec::with_capacity(total_receivers);
        for _ in 0..total_receivers {
            let rx = rx.clone();
            let handle = std::thread::spawn(move || {
                let mut rx_buf = vec![0u32; total_receive];
                rx.receive_all(rx_buf.as_mut_slice()).unwrap();
                let offset = rx_buf[0] as usize;
                let is_equal = rx_buf.iter().enumerate().all(|(i,v)| {
                    *v == (i+offset) as u32
                });
                is_equal
            });
            rx_threads.push(Some(handle));
        }

        let tx_thread = std::thread::spawn(move || {
            let mut tx_buf = vec![0u32; total_send];
            tx_buf.iter_mut().enumerate().for_each(|(i,v)| {
                *v = i as u32;
            });
            tx.send_all(tx_buf.as_mut_slice()).unwrap();
        });
 
        tx_thread.join().unwrap();
        rx_threads.iter_mut().for_each(|thread| {
            let is_equal = thread.take().unwrap().join().unwrap();
            assert!(is_equal);
        });
    }

    #[test]
    fn multiple_incoherent_send() {
        // buffers received will be sent incoherently
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total_send = channel.size();
        let total_senders = 32;
        let total_receive = total_send * total_senders;

        let mut tx_threads = Vec::with_capacity(total_senders);
        for thread_id in 0..total_senders {
            let tx = tx.clone();
            let handle = std::thread::spawn(move || {
                let mut tx_buf = vec![0u32; total_send];
                let offset = thread_id*total_send;
                tx_buf.iter_mut().enumerate().for_each(move |(i,v)| {
                    *v = (i+offset) as u32;
                });
                // data should be received completely out of order 
                for chunk in tx_buf.as_slice().chunks(8) {
                    tx.send_all(chunk).unwrap();
                }
            });
            tx_threads.push(Some(handle));
        }

        let mut counter_buf = vec![0u32; total_receive];
        let mut rx_buf = vec![0u32; total_receive];
        rx.receive_all(rx_buf.as_mut_slice()).unwrap();
        for v in rx_buf.as_slice() {
            counter_buf[*v as usize] += 1;
        }
        let is_all_receive = counter_buf.iter().all(|v| *v == 1);
        assert!(is_all_receive);

        if let Ok(total_threads) = std::thread::available_parallelism() {
            if total_threads.get() > 1 {
                // almost guaranteed that read should be incoherent
                let is_coherent = rx_buf.iter().enumerate().all(|(i,v)| *v == i as u32);
                assert!(!is_coherent);
            }
        }
        tx_threads.iter_mut().for_each(|thread| thread.take().unwrap().join().unwrap());
    }

    #[test]
    fn multiple_incoherent_send_and_receive() {
        let channel = Channel::<u32>::new(0).unwrap();
        let tx = channel.create_sender();
        let rx = channel.create_receiver();
        let total = channel.size();
        let total_threads = 32;

        let mut tx_threads = Vec::with_capacity(total_threads);
        let mut rx_threads = Vec::with_capacity(total_threads);
        for thread_id in 0..total_threads {
            let tx = tx.clone();
            let handle = std::thread::spawn(move || {
                let mut tx_buf = vec![0u32; total];
                let offset = thread_id*total;
                tx_buf.iter_mut().enumerate().for_each(move |(i,v)| {
                    *v = (i+offset) as u32;
                });
                // data should be sent completely out of order
                for chunk in tx_buf.as_slice().chunks(8) {
                    tx.send_all(chunk).unwrap();
                }
            });
            tx_threads.push(Some(handle));

            let rx = rx.clone();
            let handle = std::thread::spawn(move || {
                let mut rx_buf = vec![0u32; total];
                // data should be received completely out of order
                for chunk in rx_buf.as_mut_slice().chunks_mut(8) {
                    rx.receive_all(chunk).unwrap();
                }
                rx_buf
            });
            rx_threads.push(Some(handle));
        }

        let mut counter_buf = vec![0u32; total*total_threads];
        let mut rx_buf = Vec::with_capacity(total*total_threads);
        rx_threads.iter_mut().for_each(|thread| {
            let data = thread.take().unwrap().join().unwrap();
            for d in data.as_slice() {
                counter_buf[*d as usize] += 1;
            }
            rx_buf.extend_from_slice(data.as_slice());
        });
        let is_all_receive = counter_buf.iter().all(|v| *v == 1);
        assert!(is_all_receive);
        if let Ok(total_threads) = std::thread::available_parallelism() {
            if total_threads.get() > 1 {
                // almost guaranteed that read should be incoherent
                let is_coherent = rx_buf.iter().enumerate().all(|(i,v)| *v == i as u32);
                assert!(!is_coherent);
            }
        }
        tx_threads.iter_mut().for_each(|thread| thread.take().unwrap().join().unwrap());
    }
}
