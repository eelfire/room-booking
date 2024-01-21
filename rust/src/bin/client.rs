use std::fs::File;
use std::io::Write;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[derive(Debug)]
struct RequestData {
    data: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("hello from the client!");
    let input_file_arg = std::env::args().nth(1).unwrap();
    let number_idx = input_file_arg.find(|c: char| c.is_numeric()).unwrap();
    println!("number_idx: {}", number_idx);
    let number = input_file_arg.chars().nth(number_idx).unwrap();
    println!("number: {}", number);

    let mut stream = TcpStream::connect("0.0.0.0:1234").await.unwrap();
    let (tx, mut rx) = mpsc::channel::<RequestData>(100);

    let sender_handle = tokio::spawn(async move {
        let req_file = tokio::fs::File::open(input_file_arg).await.unwrap();
        let mut reader = tokio::io::BufReader::new(req_file);
        let mut buffer = Vec::new();

        // Read the entire file into a buffer
        reader.read_to_end(&mut buffer).await.unwrap();

        // Split the buffer into lines and send each line as a request
        for (idx, line) in buffer.split(|&c| c == b'\n').enumerate() {
            // dbg!(&line);
            if idx == 0 || line.is_empty() {
                continue;
            }
            let mut data = line.to_vec();
            data.push(b'\n');
            let request = RequestData { data };
            tx.send(request).await.unwrap();
        }
    });

    let mut send_count = 0;
    let mut recv_count = 0;
    let receiver_handle = tokio::spawn(async move {
        let mut output_file = File::create(format!("output_{}.csv", number)).unwrap();
        write!(output_file, "Request Time,Type,Room,Timeslot,Status,\r\n").unwrap();

        let mut buffer = [0; 1024];
        while let Some(request) = rx.recv().await {
            stream.write_all(&request.data).await.unwrap();
            send_count += 1;

            let bytes_read = stream.read(&mut buffer).await.unwrap();
            if bytes_read > 0 {
                let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                // print!("Received: {}", response);
                write!(output_file, "{}", response).unwrap();
                recv_count += 1;
            }
        }
        println!("recv_count: {}", recv_count);
        println!("send_count: {}", send_count);
    });

    let _ = tokio::try_join!(sender_handle, receiver_handle);

    Ok(())
}
