pub fn parse_data(payload : &str) {
    println!("Raw Data with CRC: {}", payload);
    let payload = &payload[0..payload.len() - 4];
    println!("Raw Data without CRC: {}", payload);

    // Start getting the details from the payload.
    let header_data = get_data_details(payload);
    println!("Header Details: {:?}", header_data);

    if header_data.2 != 0 && header_data.3 != 32 {
        println!("Dropping conectric message: {}, message has extended header or payload type not supported", payload);
        return;
    }

    let readable_data = get_readable_data(header_data.1, payload);
    println!("Raw Readable Data: {:?}", readable_data);
    
    // TODO : Add Caching System to prevent duplicates
    // -----------------------------------------------

    create_payload(readable_data);
}

/**
* This function is responsible for creating a dynamic payload
*/
fn create_payload(payload: (i32, &str, i32, f32, &str)) {
    let msg_type = generate_string_message_type(payload.0);
    if msg_type == "unknown" { return; }
    println!("Message Type: {}", msg_type);

    if msg_type == "tempHumidityAdc" {
        let temperature_raw = &payload.4[10..14];
        let humidity_raw = &payload.4[14..18];
        let adc_max_raw = &payload.4[22..26];
        let adc_in_raw = &payload.4[26..];

        println!("Event Count: {:?}", i32::from_str_radix(&payload.4[2..10],16).unwrap());

        println!("Temperature Raw: {:?}", temperature_raw);
        println!("Humidity Raw: {:?}", humidity_raw);
        println!("Adc Max Raw: {:?}", adc_max_raw);
        println!("Adc In Raw: {:?}", adc_in_raw);

        if let Ok(temperature_raw) = i32::from_str_radix(temperature_raw, 16) {
            let temperature = calculate_temperature(temperature_raw);
            println!("Calculated Temperature: {:.2}°C", temperature);
        } else {
            println!("Error parsing temperature raw.");
        }

        if let Ok(humidity_raw) = i32::from_str_radix(humidity_raw, 16) {
            let humidity = calculate_humidity(humidity_raw);
            println!("Calculated Humidity: {:.2}%", humidity);
        } else {
            println!("Error parsing humidity raw.");
        }
    }
}



fn round(method: fn(f32) -> f32, number: f32, precision: i32) -> f32 {
    let factor = 10_f32.powi(precision);
    let abs_number = number.abs();
    (method(abs_number * factor) / factor) * if number < 0.0 { -1.0 } else { 1.0 }
}

fn round_to(number: f32, precision: i32 ) -> f32 {
    round(f32::round, number, precision)
}

fn calculate_humidity(humidity_raw: i32) -> f32 {
    round_to(-6.0 + 125.0 * (humidity_raw as f32 / 65536.0), 2)
}
 
fn calculate_temperature(temp_raw: i32) -> f32 {
   round_to(-46.85 + (temp_raw as f32 / 65536.0) * 175.72, 2)
}


fn generate_string_message_type(message_type: i32) -> &'static str {
    match message_type {
        30 => "tempHumidity",
        31 => "switch",
        32 => "motion",
        36 => "rs485Request",
        37 => "rs485Response",
        38 => "rs485ChunkRequest",
        39 => "rs485ChunkResponse",
        40 => "pulse",
        41 => "echoStatus",
        42 => "rs485ChunkEnvelopeResponse",
        43 => "rs485Status",
        44 => "moisture",
        45 => "tempHumidityLight",
        46 => "tempHumidityAdc",
        60 => "boot",
        61 => "text",
        70 => "rs485Config",
        _ => "unknown"
    }
}

/**
* This function would return a much easier to read payload
* Parameter needs (Header Length, Payload String)
* @params (i32, &str)
* Returns (Message Type (i32 ver), Source MAC, Sequence Number, Battery Level, Data to be deconstructed)
* @return (i32, &str, i32, f32, &str)
*/
fn get_readable_data(header_len: i32, payload: &str) -> (i32, &str, i32, f32, &str) {
    (
        payload[(2 + header_len as usize * 2)..(4 + header_len as usize * 2)]
        .parse::<i32>()
            .unwrap(),
        &payload[8..12],
        i32::from_str_radix(&payload[2..4],16)
            .unwrap(),
        i32::from_str_radix(&payload[(4 + header_len as usize * 2)..(6 + header_len as usize * 2)], 16)
            .unwrap() as f32 / 10.0,
        &payload[6 + header_len as usize * 2..]
    )
}

/**
* This function would return a payload to be calculated
* Parameter needs (Payload String)
* @params (&str)
* Returns (Hex Header, Header Length, Header Type, Payload Type)
* @return (i32, i32, i32, i32)
*/
fn get_data_details(payload: &str) -> (i32, i32, i32, i32) {
    let hex_header : i32 = i32::from_str_radix(&payload[0..2], 16).unwrap();
    let header_length : i32 = hex_header & 0x1F;
    let header_type : i32 = hex_header & 0x80;
    let payload_type : i32 = hex_header & 0x60;

    (hex_header, header_length, header_type, payload_type)
}