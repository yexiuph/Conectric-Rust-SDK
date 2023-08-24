use std::{fs, sync::Arc};

use crate::RoundTo;
use chrono::{format::StrftimeItems, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, Number};

#[derive(Debug)]
struct UnknownMessageTypeError;

struct HeaderData {
    header_length: i32,
    header_type: i32,
    payload_type: i32,
}

struct ParsableData {
    message_type_raw: i32,
    source_address: String,
    sequence_number: Option<i32>,
    battery_level: f32,
    payload_data: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ParsedData {
    pub gateway_id: String,
    pub sensor_id: String,
    pub sequence_number: i32,
    pub sensor_type: String,
    pub timestamp: i64,
    pub payload: Value,
}

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
    pub fn parse_data(payload: &str, serial_mac: Arc<Option<String>>) {
        // Remove CRC
        let payload: &str = &payload[0..payload.len() - 4];

        let header_data: HeaderData = Self::get_data_details(payload);
        if header_data.header_type != 0 && header_data.payload_type != 32 {
            println!("Dropping conectric message: {}, message has extended header or payload type not supported", payload);
            return;
        }

        let parsable_data = Self::get_parsable_data(header_data.header_length, payload);
        let sensor_id = parsable_data.source_address.clone();
        let cloned_sequence_number = parsable_data.sequence_number.clone();

        // TODO!:  Add in memory cache to prevent duplication of data

        // TODO: payload variable would hold a JSON for data outbound
        let mut payload = Self::create_payload(parsable_data);
        let payload_ref = &mut payload;

        // Construct the ParsedData struct
        let parsed_data: ParsedData = ParsedData {
            gateway_id: serial_mac.as_ref().clone().unwrap(),
            sensor_id,
            sequence_number: cloned_sequence_number.unwrap(),
            sensor_type: payload_ref.0.to_owned(),
            payload: payload_ref.1.to_owned(),
            timestamp: Utc::now().timestamp(),
        };

        println!("JSON Output: {:?}", parsed_data);

        // Format the timestamp as a string
        let timestamp_formatted = Utc::now()
            .format_with_items(StrftimeItems::new("%Y%m%d%H%M%S"))
            .to_string();

        // Generate the JSON filename using sensor_type and timestamp
        let json_filename = format!("{}_{}.json", payload_ref.0, timestamp_formatted);

        // Serialize the parsed_data to JSON
        let json_data = serde_json::to_string_pretty(&parsed_data).unwrap();

        // Write the JSON data to the file
        if let Err(err) = fs::write(json_filename, json_data) {
            eprintln!("Error writing JSON data to file: {}", err);
        }
    }

    /**
     * This function would return a payload to be calculated
     * Parameter needs (Payload String)
     * @params (&str)
     * For the return please check the HeaderData struct
     * @return HeaderData
     */
    fn get_data_details(payload: &str) -> HeaderData {
        let hex_header: i32 = i32::from_str_radix(&payload[0..2], 16).unwrap();
        HeaderData {
            header_length: hex_header & 0x1F,
            header_type: hex_header & 0x80,
            payload_type: hex_header & 0x60,
        }
    }

    /**
     * This function would return a much easier to read payload
     * Parameter needs (Header Length, Payload String)
     * @params (i32, &str)
     * Returns (Message Type (i32 ver), Source MAC, Sequence Number, Battery Level, Data to be deconstructed)
     * @return (i32, &str, i32, f32, &str)
     */
    fn get_parsable_data(header_len: i32, payload: &str) -> ParsableData {
        ParsableData {
            message_type_raw: payload[(2 + header_len as usize * 2)..(4 + header_len as usize * 2)]
                .parse::<i32>()
                .unwrap(),
            source_address: payload[8..12].to_string(),
            sequence_number: Some(i32::from_str_radix(&payload[2..4], 16).unwrap()),
            battery_level: i32::from_str_radix(
                &payload[(4 + header_len as usize * 2)..(6 + header_len as usize * 2)],
                16,
            )
            .unwrap() as f32
                / 10.0,
            payload_data: payload[6 + header_len as usize * 2..].to_string(),
        }
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
            33 => "keepAlive",
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
    fn create_payload(payload: ParsableData) -> (String, Value) {
        let msg_type = Self::generate_string_message_type(payload.message_type_raw);

        let mut json_payload = match msg_type {
            "unknown" => json!({"unknown": "unknown"}),
            "tempHumidity" => Self::create_th_payload(),
            "tempHumidityAdc" => Self::create_thadc_payload(&payload.payload_data),
            "boot" => Self::create_boot_payload(&payload.payload_data),
            "keepAlive" => json!({}),
            _ => json!({"null": "null"})
        };
        
        // Add battery here to the Value
        let formatted_battery = format!("{:.2}", payload.battery_level);
        let battery_number = formatted_battery.parse::<f64>().unwrap();
        let battery_value = Number::from_f64(battery_number).unwrap();
        json_payload["battery"] = json!(battery_value);
        
        (
            msg_type.to_string().to_string(),
            json_payload,
        )

        // if msg_type == "tempHumidityAdc" {
        //     let temperature_raw = &payload.4[10..14];
        //     let humidity_raw = &payload.4[14..18];
        //     let adc_max_raw = &payload.4[22..26];
        //     let adc_in_raw = &payload.4[26..];

        //     println!(
        //         "Event Count: {:?}",
        //         
        //     );

        //     println!("Temperature Raw: {:?}", temperature_raw);
        //     println!("Humidity Raw: {:?}", humidity_raw);
        //     println!("Adc Max Raw: {:?}", adc_max_raw);
        //     println!("Adc In Raw: {:?}", adc_in_raw);

        //     if let Ok(temperature_raw) = i32::from_str_radix(temperature_raw, 16) {
        //         let temperature: f32 = Self::calculate_temperature(temperature_raw);
        //         println!("Calculated Temperature: {:.2}°C", temperature);
        //     } else {
        //         println!("Error parsing temperature raw.");
        //     }

        //     if let Ok(humidity_raw) = i32::from_str_radix(humidity_raw, 16) {
        //         let humidity: f32 = Self::calculate_humidity(humidity_raw);
        //         println!("Calculated Humidity: {:.2}%", humidity);
        //     } else {
        //         println!("Error parsing humidity raw.");
        //     }
    }

    fn create_th_payload() -> Value {
        return json!({});
    }

    fn create_thadc_payload(payload: &str) -> Value {
        json!({
            "eventCount": i32::from_str_radix(&payload[2..10], 16).unwrap_or(0),
            "temperature": Self::calculate_temperature(i32::from_str_radix(&payload[10..14], 16).unwrap_or(0)),
            "temperature_unit": "C",
            "humidity": Self::calculate_humidity(i32::from_str_radix(&payload[14..18], 16).unwrap_or(0)),
            "adcMax" : i32::from_str_radix(&payload[22..26], 10).unwrap_or(0),
            "adcIn" : i32::from_str_radix(&payload[26..], 10).unwrap_or(0)
        })
    }

    fn create_boot_payload(payload: &str) -> Value {
        let boot_status = match payload {
            "00" => "powerOn",
            "01" => "externalReset",
            "02" => "watchdogReset",
            _ => "unknown",
        };

        json!({ "resetCause": boot_status })
    }
}
