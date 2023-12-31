use crate::RoundTo;
use serde_json::json;

pub struct 

pub struct ConectricParser;

impl ConectricParser {
    pub fn new() -> Self {
        Self
    }

    /**
     * This function is responsible for creating a payload and returns the payload
     * Parameter needs (Payload String)
     * @params (&str)
     */
    pub fn parse_data(payload: &str) {
        // Remove CRC
        let payload: &str = &payload[0..payload.len() - 4];

        let header_data: (i32, i32, i32, i32) = Self::get_data_details(payload);
        if header_data.2 != 0 && header_data.3 != 32 {
            println!("Dropping conectric message: {}, message has extended header or payload type not supported", payload);
            return;
        }

        let parsable_data = Self::get_parsable_data(header_data.1, payload);

        println!("Sensor Address: {}", parsable_data.1);
        println!("Battery Level: {:.2}%", parsable_data.3);
        println!("Sequence Since Power: {}", parsable_data.2);

        // TODO!:  Add in memory cache to prevent duplication of data

        let payload = Self::create_payload(parsable_data);
    }

    /**
     * This function would return a payload to be calculated
     * Parameter needs (Payload String)
     * @params (&str)
     * Returns (Hex Header, Header Length, Header Type, Payload Type)
     * @return (i32, i32, i32, i32)
     */
    fn get_data_details(payload: &str) -> (i32, i32, i32, i32) {
        let hex_header: i32 = i32::from_str_radix(&payload[0..2], 16).unwrap();
        let header_length: i32 = hex_header & 0x1F;
        let header_type: i32 = hex_header & 0x80;
        let payload_type: i32 = hex_header & 0x60;

        (hex_header, header_length, header_type, payload_type)
    }

    /**
     * This function would return a much easier to read payload
     * Parameter needs (Header Length, Payload String)
     * @params (i32, &str)
     * Returns (Message Type (i32 ver), Source MAC, Sequence Number, Battery Level, Data to be deconstructed)
     * @return (i32, &str, i32, f32, &str)
     */
    fn get_parsable_data(header_len: i32, payload: &str) -> (i32, &str, i32, f32, &str) {
        (
            payload[(2 + header_len as usize * 2)..(4 + header_len as usize * 2)]
                .parse::<i32>()
                .unwrap(),
            &payload[8..12],
            i32::from_str_radix(&payload[2..4], 16).unwrap(),
            i32::from_str_radix(
                &payload[(4 + header_len as usize * 2)..(6 + header_len as usize * 2)],
                16,
            )
            .unwrap() as f32
                / 10.0,
            &payload[6 + header_len as usize * 2..],
        )
    }

    fn calculate_humidity(humidity_raw: i32) -> f32 {
        RoundTo::round_to(-6.0 + 125.0 * (humidity_raw as f32 / 65536.0), 2)
    }

    fn calculate_temperature(temp_raw: i32) -> f32 {
        RoundTo::round_to(-46.85 + (temp_raw as f32 / 65536.0) * 175.72, 2)
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
            _ => "unknown",
        }
    }

    /**
     * This function is responsible for creating a dynamic payload
     */
    fn create_payload(payload: (i32, &str, i32, f32, &str)) {
        let msg_type = Self::generate_string_message_type(payload.0);
        if msg_type == "unknown" {
            return;
        }
        println!("Message Type: {}", msg_type);

        if msg_type == "tempHumidityAdc" {
            let temperature_raw = &payload.4[10..14];
            let humidity_raw = &payload.4[14..18];
            let adc_max_raw = &payload.4[22..26];
            let adc_in_raw = &payload.4[26..];

            println!(
                "Event Count: {:?}",
                i32::from_str_radix(&payload.4[2..10], 16).unwrap()
            );

            println!("Temperature Raw: {:?}", temperature_raw);
            println!("Humidity Raw: {:?}", humidity_raw);
            println!("Adc Max Raw: {:?}", adc_max_raw);
            println!("Adc In Raw: {:?}", adc_in_raw);

            if let Ok(temperature_raw) = i32::from_str_radix(temperature_raw, 16) {
                let temperature: f32 = Self::calculate_temperature(temperature_raw);
                println!("Calculated Temperature: {:.2}°C", temperature);
            } else {
                println!("Error parsing temperature raw.");
            }

            if let Ok(humidity_raw) = i32::from_str_radix(humidity_raw, 16) {
                let humidity: f32 = Self::calculate_humidity(humidity_raw);
                println!("Calculated Humidity: {:.2}%", humidity);
            } else {
                println!("Error parsing humidity raw.");
            }
        }
    }
}
