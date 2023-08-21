use serialport::{ SerialPort, SerialPortInfo, SerialPortType, ErrorKind };
use std::{ 
    thread::sleep,
    time::Duration
};
use crate::{
    parser::parse_data
};

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
fn initialize_conectric_router(port:&mut Box<dyn SerialPort>) {
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

fn process_data(data: &str) {
    if data.starts_with('>') {
        parse_data(&data[1..]);
    } else if data.starts_with("MR:") {
        println!("MAC Address: {}", &data[3..]);
    } else if data.starts_with("DP:Ok") {
        println!("Switched to dump payload mode.");
    } else if data.starts_with("SS:Ok") {
        println!("Switched to sink mode.");
    } else if data.to_lowercase().starts_with("ver:contiki") {
        println!("Contiki Version: {}", &data[12..]);
    } else if data.to_lowercase().starts_with("ver:conectric") {
        println!("Conectric Version: {}", &data[14..]);
    }
}

pub fn start_gateway() {
    let ports = serialport::available_ports().expect("No USB router found!");
    let serial_port_name = find_conectric_router(&ports).unwrap();

    match open_serial_port(serial_port_name) {
        Ok(mut port) => {
            // Port was opened successfully
            initialize_conectric_router(&mut port);
            let mut serial_buf: Vec<u8> = vec![0; 256];
            let mut line_buffer = String::new();
            loop {
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            let data = String::from_utf8_lossy(&serial_buf[..bytes_read]);
                            for c in data.chars() {
                                if c == '\n' {
                                    // Process the complete line
                                    process_data(&line_buffer);
                                    line_buffer.clear();
                                } else {
                                    line_buffer.push(c);
                                }
                            }
                        }
                    }
                    Err(_e) => {
                       
                    }
                }
            }
        }
        Err(err) => {
            match err.kind() {
                ErrorKind::Io(_) => {
                    panic!("Error opening serial port: {:?}", err);
                }
                ErrorKind::NoDevice => {
                    println!("No serial port device found, this is expected for the test.");
                }
                _ => {
                    panic!("Unexpected error opening serial port: {:?}", err);
                }
            }
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
        println!("Available ports: {:?}", ports);
        
        let result = find_conectric_router(&ports);
        println!("Router detection result: {:?}", result);
        assert!(result.is_some(), "Expected to find a Conectric router.");
    }

    #[test]
    fn serial_port_open() {
        let ports = serialport::available_ports().expect("No ports found!");
        let serial_port_name = find_conectric_router(&ports).unwrap();
        
        match open_serial_port(serial_port_name) {
            Ok(port) => {
                // Port was opened successfully
                // You can perform additional assertions here if needed
                println!("Serial port opened successfully: {:?}", port);
            }
            Err(err) => {
                match err.kind() {
                    ErrorKind::Io(_) => {
                        panic!("Error opening serial port: {:?}", err);
                    }
                    ErrorKind::NoDevice => {
                        println!("No serial port device found, this is expected for the test.");
                    }
                    _ => {
                        panic!("Unexpected error opening serial port: {:?}", err);
                    }
                }
            }
        }
    }
}