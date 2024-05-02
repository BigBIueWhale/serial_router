# 🚀 serial_router

This Rust application listens to multiple serial ports and forwards the received data over UDP to a specified IP address and port. It provides a convenient way to bridge communication between serial devices and network applications.

## 📋 Platform
Tested on Pop! OS 22.04

Designed to be cross platform, untested.

## 📦 Dependencies
* Run: `sudo apt install build-essential libudev-dev`

* Install Rust using Rustup (cargo 1.72.0 (103a7ff2e 2023-08-15))

## 🔒 Serial Permissions
To allow a user to perform serial communication without needing sudo privileges on Ubuntu 22.04, you can add the user to the `dialout` group. This group is typically granted permissions to access serial interfaces like `/dev/ttyS0`, `/dev/ttyUSB0`, etc.

Here are the commands you'll need to run:

1. Open a terminal.

2. Add your user (in this case, "user") to the `dialout` group by using the following command:
   ```bash
   sudo usermod -a -G dialout user
   ```

3. To ensure the changes take effect, you might need to log out and log back in, or you can use the following command to apply the changes immediately:
   ```bash
   newgrp dialout
   ```

After performing these steps, the user "user" should be able to access serial communication devices without needing sudo privileges.

## 📥 Inputs
The application prompts the user to enter the numbers of the serial ports they want to listen on, separated by commas. For example:

```
Available serial ports:
1: /dev/ttyUSB0
2: /dev/ttyACM0
3: /dev/ttyS0

Type the numbers of the ports you want to listen on, separated by commas:
1,3
```

## 📤 Outputs
The application forwards the received serial data over UDP to the specified IP address and port. The data is sent as JSON packets with the following format:

```json
{
  "port": "/dev/ttyUSB0",
  "data": "SGVsbG8sIFdvcmxkIQ=="
}
```

- `port`: The name of the serial port from which the data was received.
- `data`: The received serial data, encoded in Base64 format.

## 📊 Data Flow

```
+----------------+      +-------------+      +-----------------+
| Serial Port(s) | ---> | Application | ---> | UDP Destination |
+----------------+      +-------------+      +-----------------+
```

1. The application listens to the selected serial ports.
2. When data is received on a serial port, it is encoded in Base64 format.
3. The encoded data, along with the port name, is packaged into a JSON object.
4. The JSON object is sent as a UDP packet to the specified IP address and port.

## 🛑 Graceful Shutdown
The application supports graceful shutdown by listening for the CTRL+C signal. When CTRL+C is received, the application stops listening to the serial ports and exits cleanly.

## 🏁 Getting Started
1. Clone the repository.
2. Install the dependencies as mentioned in the "Dependencies" section.
3. Build the application using `cargo build --release`.
4. Run the application using `./target/release/serial_router`.
5. Follow the prompts to select the desired serial ports.
6. The application will start forwarding the serial data to the specified UDP destination.

Feel free to customize and extend the application to suit your specific requirements! 😊
