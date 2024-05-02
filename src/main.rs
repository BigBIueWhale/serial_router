use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json;
use serialport::available_ports;
use std::io::{self, Write};
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::{net::UdpSocket, signal, sync::mpsc};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting the application...");

    let ports = available_ports()?;
    if ports.is_empty() {
        println!("No serial ports found.");
        return Ok(());
    }

    println!("Available serial ports:");
    for (idx, p) in ports.iter().enumerate() {
        println!("{}: {}", idx + 1, p.port_name);
    }

    println!("Type the numbers of the ports you want to listen on, separated by commas:");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selections: Vec<usize> = input
        .trim()
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .collect();

    let selected_ports: Vec<String> = selections
        .into_iter()
        .filter(|&idx| idx > 0 && idx <= ports.len())
        .filter_map(|idx| ports.get(idx - 1).map(|p| p.port_name.clone()))
        .collect();

    if selected_ports.is_empty() {
        println!("No valid selections made.");
        return Ok(());
    }

    println!("Selected ports: {:?}", selected_ports);

    let listener_task = listen_to_ports(selected_ports);
    let ctrl_c_task = tokio::spawn(async {
        signal::ctrl_c().await.expect("Failed to listen for CTRL+C");
        println!("Received CTRL+C, shutting down...");
    });

    tokio::select! {
        result = listener_task => {
            println!("Listener task completed.");
            result?
        },
        _ = ctrl_c_task => {
            println!("CTRL+C received.");
        }
    };

    println!("Application exiting...");
    Ok(())
}

async fn listen_to_ports(ports: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Binding to UDP socket...");
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect("127.0.0.1:34254").await?;
    println!("UDP socket bound and connected.");

    let (tx, mut rx) = mpsc::channel(100);

    let mut port_streams: Vec<SerialStream> = Vec::new();
    for port_name in &ports {
        let port = tokio_serial::new(port_name.clone(), 115200).open_native_async()?;
        port_streams.push(SerialStream::from(port));
    }

    let serial_handle = tokio::spawn(async move {
        let mut shared_buffer = vec![0u8; 0xffff];
        let timeout = tokio::time::Duration::from_millis(100); // Overall timeout for each read operation

        loop {
            for (idx, port_stream) in port_streams.iter_mut().enumerate() {
                for byte_to_send in [0x5, 0x6, 0x7, 0x8] {
                    match port_stream.write_all(&[byte_to_send]) {
                        Ok(()) => (),
                        Err(_) => println!("Error writing data to port index {}", idx)
                    }

                    let start_time = Instant::now();
                    let mut total_read = 0;
    
                    loop  {
                        if total_read >= 2 {
                            // Check if ends with "\n" like in the ICD
                            if shared_buffer[total_read - 1] == b'\n' {
                                break;
                            }
                        }
                        let time_elapsed = Instant::now().duration_since(start_time);
                        if time_elapsed >= timeout {
                            eprintln!("Timeout reached for port {}", ports[idx]);
                            break;
                        }
    
                        let time_remaining = timeout - time_elapsed;
                        match tokio::time::timeout(time_remaining, port_stream.read(&mut shared_buffer[total_read..])).await {
                            Ok(Ok(n)) if n == 0 => break,
                            Ok(Ok(n)) => total_read += n,
                            Ok(Err(e)) => {
                                eprintln!("Error reading from port {}: {}", ports[idx], e);
                                break;
                            },
                            Err(_) => {
                                let nicely_printed = std::str::from_utf8(&&shared_buffer[..total_read]).unwrap().escape_default().to_string();
                                eprintln!("Sent {} to port {}, waiting for more data. data: \"{}\"", byte_to_send, ports[idx], nicely_printed);
                                break;
                            }
                        }
                    }

                    if total_read > 0 {
                        let time_elapsed = Instant::now().duration_since(start_time);
                        let data = &shared_buffer[..total_read];
                        let encoded_data = STANDARD.encode(data);
                        let json = serde_json::json!({
                            "port": ports[idx],
                            "data": encoded_data,
                            "duration_microseconds": time_elapsed.as_micros()
                        });

                        if let Ok(data_string) = serde_json::to_string(&json) {
                            if tx.send(data_string).await.is_err() {
                                eprintln!("Error sending data via channel");
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    let udp_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if socket.send(data.as_bytes()).await.is_err() {
                eprintln!("Error sending data via UDP");
            }
        }
    });

    tokio::try_join!(serial_handle, udp_handle)?;

    Ok(())
}