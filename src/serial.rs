use crate::parser::ConectricParser;
use serialport::{ErrorKind, SerialPort, SerialPortInfo, SerialPortType};
use std::{sync::Arc, thread::sleep, time::Duration};

// Struct Implementation for Calling
pub struct ConectricSerial {
    // Exposing the Serial Port
    pub serial_port: Option<Box<dyn SerialPort>>,
    pub serial_mac: Arc<Option<String>>,
}

impl ConectricSerial {
    pub fn new(serial_mac: Arc<Option<String>>) -> Self {
        Self {
            serial_port: None,
            serial_mac,
        }
    }

    /**
     * This fuction find a suitable conectric router plugged in into the device
     * and return a Option<String>
     */
    fn find_conectric_router(ports: &[SerialPortInfo]) -> Option<String> {
        for p in ports {
            if let SerialPortType::UsbPort(usb_info) = &p.port_type {
                match (usb_info.vid, usb_info.pid, usb_info.manufacturer.as_deref()) {
                    (1027, 24597, Some("FTDI")) => return Some(p.port_name.clone()),
                    _ => (),
                }
            }
        }
        None
    }

    /**
     * This fuction open the serial port from a port name
     * and return a Box<dyn SerialPort> if it succesfully connected.
     */
    fn open_serial_port(port_name: String) -> Result<Box<dyn SerialPort>, serialport::Error> {
        return serialport::new(port_name.clone(), 230_400)
            .timeout(Duration::from_millis(100))
            .open();
    }

    /**
     * This fuction initialize the router to start listening into the sensors broadcasting messages
     * DP - Dump payload
     * VER - Gets version of contiki and conectric
     * MR - Gets the MAC Address of the router
     * SS - Switch the router to sink mode
     */
    fn initialize_conectric_router(port: &mut Box<dyn SerialPort>) {
        println!("Connected to the serial port.");
        sleep(Duration::from_millis(10));
        port.write(b"DP\n").expect("Write failed!");
        sleep(Duration::from_millis(10));
        port.write(b"VER\n").expect("Write failed!");
        sleep(Duration::from_millis(10));
        port.write(b"MR\n").expect("Write failed!");
        sleep(Duration::from_millis(10));
        port.write(b"SS\n").expect("Write failed!");
    }

    fn process_data(&mut self, data: &str, serial_mac: Arc<Option<String>>) {
        if data.starts_with('>') {
            ConectricParser::parse_data(&data[1..], serial_mac);
        } else {
            match data {
                s if s.starts_with("test") => println!("Test Received"),
                s if s.starts_with("MR:") => {
                    println!("MAC Address: {}", &s[3..]);
                    self.serial_mac = Some(s[3..].to_string()).into();
                }
                "DP:Ok" => println!("Switched to dump payload mode."),
                "SS:Ok" => println!("Switched to sink mode."),
                s if s.to_lowercase().starts_with("ver:contiki") => {
                    println!("Contiki Version: {}", &s[12..])
                }
                s if s.to_lowercase().starts_with("ver:conectric") => {
                    println!("Conectric Version: {}", &s[14..])
                }
                _ => println!("Unknown response: {}", data),
            }
        }
    }

    pub fn start_gateway(&mut self) {
        const MAX_RETRIES: usize = 5;
        let mut retries = 0;
    
        loop {
            match serialport::available_ports() {
                Ok(ports) => {
                    if let Some(serial_port_name) = Self::find_conectric_router(&ports) {
                        match Self::open_serial_port(serial_port_name) {
                            Ok(mut port) => {
                                // Port was opened successfully
                                Self::initialize_conectric_router(&mut port);
                                // Populate the serial_port field
                                self.serial_port = Some(port);
    
                                let mut serial_buf: Vec<u8> = vec![0; 256];
                                let mut line_buffer = String::new();
                                loop {
                                    match self
                                        .serial_port
                                        .as_mut()
                                        .unwrap()
                                        .read(serial_buf.as_mut_slice())
                                    {
                                        Ok(bytes_read) => {
                                            if bytes_read > 0 {
                                                let data = String::from_utf8_lossy(&serial_buf[..bytes_read]);
                                                for c in data.chars() {
                                                    if c == '\n' {
                                                        // Process the complete line
                                                        self.process_data(&line_buffer, self.serial_mac.clone()); // Pass gateway_id
                                                        line_buffer.clear();
                                                    } else {
                                                        line_buffer.push(c);
                                                    }
                                                }
                                            }
                                        }
                                        Err(_e) => {
                                            // Ignore read errors
                                        }
                                    }
                                }
                            }
                            Err(err) => match err.kind() {
                                ErrorKind::Io(_) => {
                                    eprintln!("Error opening serial port: {:?}", err);
                                }
                                ErrorKind::NoDevice => {
                                    println!("No serial port device found.");
                                }
                                _ => {
                                    eprintln!("Unexpected error opening serial port: {:?}", err);
                                }
                            },
                        }
                    } else {
                        println!("No USB router found.");
                    }
                }
                Err(err) => {
                    eprintln!("Error retrieving available ports: {:?}", err);
                }
            }
    
            retries += 1;
            if retries >= MAX_RETRIES {
                println!("Max retry attempts reached, giving up.");
                break; // Exit the loop after max retries
            }
    
            // Wait for a moment before retrying
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}

#[cfg(test)]
mod serial_tests {
    use super::*;
    use serialport::ErrorKind;

    #[test]
    fn serial_port_detection() {
        let ports = serialport::available_ports().expect("No ports found!");
        println!("Available ports: {:#?}", ports);

        let result = ConectricSerial::find_conectric_router(&ports);
        println!("Router detection result: {:?}", result);
        assert!(result.is_some(), "Expected to find a Conectric router.");
    }

    #[test]
    fn serial_port_open() {
        let ports = serialport::available_ports().expect("No ports found!");
        let serial_port_name = ConectricSerial::find_conectric_router(&ports).unwrap();

        match ConectricSerial::open_serial_port(serial_port_name) {
            Ok(port) => {
                // Port was opened successfully
                // You can perform additional assertions here if needed
                println!("Serial port opened successfully: {:?}", port);
            }
            Err(err) => match err.kind() {
                ErrorKind::Io(_) => {
                    panic!("Error opening serial port: {:?}", err);
                }
                ErrorKind::NoDevice => {
                    println!("No serial port device found, this is expected for the test.");
                }
                _ => {
                    panic!("Unexpected error opening serial port: {:?}", err);
                }
            },
        }
    }
}
