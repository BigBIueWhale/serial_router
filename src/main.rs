use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json;
use serialport::available_ports;
use std::io::{self};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    println!("Iterating through ports");
    for port_name in &ports {
        let port = tokio_serial::new(port_name.clone(), 115200).open_native_async()?;
        port_streams.push(SerialStream::from(port));
    }

    println!("Opened all ports");

    let handle = tokio::spawn(async move {
        let bytes_to_send = [0x5, 0x6, 0x7, 0x8];
        loop {
            for (idx, port_stream) in port_streams.iter_mut().enumerate() {
                for byte in &bytes_to_send {
                    let start_time = Instant::now();
                    if let Err(e) = port_stream.write_all(&[*byte]).await {
                        eprintln!("Error writing to port {}: {}", ports[idx], e);
                        continue;
                    }

                    let mut buf = Vec::new();
                    loop {
                        let mut temp_buf = [0; 1];
                        match tokio::time::timeout(tokio::time::Duration::from_millis(100), port_stream.read(&mut temp_buf)).await {
                            Ok(Ok(0)) => break,
                            Ok(Ok(_)) => buf.extend_from_slice(&temp_buf),
                            Ok(Err(e)) => {
                                eprintln!("Error reading from port {}: {}", ports[idx], e);
                                break;
                            },
                            Err(_) => {
                                eprintln!("Timed out waiting for response from port {}", ports[idx]);
                                break;
                            },
                        }

                        if buf.ends_with(b"\r\n") {
                            break;
                        }
                    }

                    let end_time = Instant::now();
                    let duration = end_time.duration_since(start_time);

                    let encoded_data = STANDARD.encode(&buf);
                    let json = serde_json::json!({
                        "port": ports[idx].clone(),
                        "data": encoded_data,
                        "duration": duration.as_millis()
                    });

                    if let Ok(data) = serde_json::to_string(&json) {
                        let _ = tx.send(data).await;
                    }

                    buf.clear();
                }
            }
        }
    });

    tokio::spawn(async move {
        println!("Starting UDP sending task...");
        while let Some(data) = rx.recv().await {
            let _ = socket.send(data.as_bytes()).await;
        }
        println!("UDP sending task finished.");
    });

    handle.await?;

    Ok(())
}
