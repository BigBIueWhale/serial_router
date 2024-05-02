use serialport::available_ports;
use std::io::{self};
use tokio::{net::UdpSocket, signal, sync::mpsc, task::JoinHandle};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio::io::AsyncReadExt;
use serde_json;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

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
    let selections: Vec<usize> = input.trim().split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .collect();

    let selected_ports: Vec<String> = selections.into_iter()
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
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for port_name in ports {
        let port = tokio_serial::new(port_name.clone(), 9600).open_native_async()?;
        let mut buf = vec![0; 1024];
        let mut port_stream = SerialStream::from(port);
        let tx_clone = tx.clone();

        let handle = tokio::spawn(async move {
            println!("Listening on serial port: {}", port_name);
            loop {
                match port_stream.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        let encoded_data = STANDARD.encode(&buf[..n]);
                        let json = serde_json::json!({
                            "port": port_name.clone(),
                            "data": encoded_data
                        });
                        if let Ok(data) = serde_json::to_string(&json) {
                            let _ = tx_clone.send(data).await;
                        }
                    },
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Error reading from {}: {}", port_name, e);
                        break;
                    },
                }
            }
            println!("Stopped listening on serial port: {}", port_name);
        });

        handles.push(handle);
    }

    tokio::spawn(async move {
        println!("Starting UDP sending task...");
        while let Some(data) = rx.recv().await {
            let _ = socket.send(data.as_bytes()).await;
        }
        println!("UDP sending task finished.");
    });

    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}
