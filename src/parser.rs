use std::sync::Arc;

use crate::RoundTo;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};

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
    #[serde(rename = "gatewayId")]
    pub gateway_id: String,
    #[serde(rename = "sensorId")]
    pub sensor_id: String,
    #[serde(rename = "sequenceNum")]
    pub sequence_number: i32,
    #[serde(rename = "type")]
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

        println!("JSON Output: {:#?}", parsed_data);
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

    fn calculate_humidity(humidity_raw: i32) -> Number {
        let calculated_humidity = -6.0 + 125.0 * (humidity_raw as f32 / 65536.0);
        let formatted_humidity = format!("{:.2}", RoundTo::round_to(calculated_humidity, 2));
        let humidity_value = formatted_humidity.parse::<f64>().unwrap();
        Number::from_f64(humidity_value).unwrap()
    }

    fn calculate_temperature(temp_raw: i32) -> Number {
        let calculated_temp = -46.85 + (temp_raw as f32 / 65536.0) * 175.72;
        let formatted_temp = format!("{:.2}", RoundTo::round_to(calculated_temp, 2));
        let temperature_value = formatted_temp.parse::<f64>().unwrap();
        Number::from_f64(temperature_value).unwrap()
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
        if payload.battery_level != 0.0 && payload.battery_level.is_nan() == true {
            return ("".to_string(), json!(""));
        }
        let msg_type = Self::generate_string_message_type(payload.message_type_raw);

        // Add battery here to the Value
        let formatted_battery = format!("{:.2}", payload.battery_level);
        let battery_number = formatted_battery.parse::<f64>().unwrap();
        let battery_value = Number::from_f64(battery_number).unwrap();

        let json_payload = match msg_type {
            "unknown" => json!({ "unknown": "unknown" }),
            "tempHumidity" => {
                let mut data = Self::create_th_payload(&payload.payload_data);
                data["battery"] = json!(battery_value);
                data
            }
            "tempHumidityAdc" => {
                let mut data = Self::create_thadc_payload(&payload.payload_data);
                data["battery"] = json!(battery_value);
                data
            }
            "tempHumidityLight" => {
                let lux = RoundTo::round_to(
                    0.003
                        * (i32::from_str_radix(&&payload.payload_data[26..], 16).unwrap() as f32)
                            .powf(1.89 - (3.7 - payload.battery_level) / 25.0),
                    0,
                );
                let mut data = Self::create_thlight_payload(lux, &payload.payload_data);
                data["battery"] = json!(battery_value);
                data
            }
            // Can be implemented upon request.
            // "moisture" => {

            // }
            "echoStatus" | "rs485Status" => {
                let mut data = json!({});
                if payload.payload_data.len() == 10 {
                    data["eventCount"] =
                        json!(i32::from_str_radix(&payload.payload_data[2..], 16).unwrap());
                    data
                } else {
                    data
                }
            }
            "motion" => {
                if payload.payload_data.starts_with("20") {
                    json!({
                        "status": false,
                        "battery": battery_value
                    })
                } else {
                    let mut data = json!({
                        "status": true,
                        "battery": battery_value
                    });

                    if payload.payload_data.len() == 10 {
                        data["eventCount"] =
                            json!(i32::from_str_radix(&payload.payload_data[2..], 16).unwrap());
                        data
                    } else {
                        data
                    }
                }
            }
            "pulse" => {
                if payload.payload_data.starts_with("20") {
                    json!({
                        "status": false,
                        "battery": battery_value
                    })
                } else {
                    let mut data = json!({
                        "status": true,
                        "battery": battery_value
                    });
                    if payload.payload_data.len() == 10 {
                        data["eventCount"] =
                            json!(i32::from_str_radix(&payload.payload_data[2..], 16).unwrap());
                        data
                    } else {
                        data
                    }
                }
            }
            "switch" => {
                let mut data = Self::create_switch_payload(&payload.payload_data);
                data["battery"] = json!(battery_value);
                data
            }
            "keepAlive" => json!({ "battery": battery_value }),
            "boot" => {
                let mut data = Self::create_boot_payload(&payload.payload_data);
                data["battery"] = json!(battery_value);
                data
            }
            "rs485Config" => Self::create_rsconfig_payload(&payload.payload_data),
            "rs485Request" => json!({"data": &payload.payload_data}),
            "rs485Response" => Self::rs485_response_payload(&payload.payload_data),
            "rs485ChunkEnvelopResponse" => json!({
                "battery" : battery_value,
                "numChunks" : i32::from_str_radix(&payload.payload_data[0..2], 16).unwrap(),
                "chunkSize" : i32::from_str_radix(&payload.payload_data[2..], 16).unwrap(),
            }),
            "rs485ChunkResponse" | "text" => json!({
                "battery" : battery_value,
                "data": &payload.payload_data,
            }),

            _ => json!({ "null": "null" }),
        };

        (msg_type.to_string().to_string(), json_payload)
    }

    fn create_thlight_payload(lux: f32, payload: &str) -> Value {
        let bucketed_lux = ((lux / 100.0).round()).min(15.0);
        json!({
            "eventCount": i32::from_str_radix(&payload[2..10], 16).unwrap_or(0),
            "temperature": Self::calculate_temperature(i32::from_str_radix(&payload[10..14], 16).unwrap_or(0)),
            "temperatureUnit": "C",
            "humidity": Self::calculate_humidity(i32::from_str_radix(&payload[14..18], 16).unwrap_or(0)),
            "adcMax" : i32::from_str_radix(&payload[22..26], 16).unwrap_or(0),
            "adcIn" : i32::from_str_radix(&payload[26..], 16).unwrap_or(0),
            "bucketedLux": bucketed_lux,
        })
    }

    fn rs485_response_payload(payload: &str) -> Value {
        // TODO: Add Return Temp Calculation
        let mut data = json!({
            "data": payload,
        });
        let identifier = &payload[0..2];
        if identifier == "02" {
            data["co2"] = json!(i32::from_str_radix(&payload[6..10], 16).unwrap());
            data
        } else if i32::from_str_radix(identifier, 16).unwrap() >= 0x03 {
            let actuator_number = i32::from_str_radix(identifier, 16).unwrap();
            let payload_name = format!("actuator{}", actuator_number);
            data[payload_name] = json!(i32::from_str_radix(&payload[6..10], 16).unwrap());
            data
        } else {
            data
        }
    }

    fn create_rsconfig_payload(payload: &str) -> Value {
        if payload.len() != 7 {
            json!({})
        } else {
            let baud_rate: i32 = match &payload[0..2] {
                "00" => 2400,
                "01" => 4800,
                "02" => 9600,
                "03" => 19200,
                _ => 0,
            };

            let parity: &str = match &payload[2..4] {
                "00" => "none",
                "01" => "odd",
                "02" => "even",
                _ => "?",
            };

            let stop_bits: i32 = match &payload[4..6] {
                "00" => 1,
                "01" => 2,
                _ => -1,
            };

            let bit_mask: i32 = match &payload[4..6] {
                "00" => 1,
                "01" => 2,
                _ => -1,
            };

            json!({
                "baudRate": baud_rate,
                "parity": parity,
                "stopBits": stop_bits,
                "bitMask" : bit_mask
            })
        }
    }

    fn create_th_payload(payload: &str) -> Value {
        if payload.len() != 8 {
            println!("Ignoring temperature humidity payload. Expecting length as 8");
            return json!({});
        } else {
            json!({
                "eventCount": i32::from_str_radix(&payload[2..10], 16).unwrap_or(0),
                "temperature": Self::calculate_temperature(i32::from_str_radix(&payload[10..14], 16).unwrap_or(0)),
                "temperatureUnit": "C",
                "humidity": Self::calculate_humidity(i32::from_str_radix(&payload[14..18], 16).unwrap_or(0)),
            })
        }
    }

    fn create_switch_payload(payload: &str) -> Value {
        let status = if payload.starts_with("81") {
            true
        } else {
            false
        };

        if payload.len() == 10 {
            let event_count = json!(i32::from_str_radix(&payload[2..], 16).unwrap());
            json!({"eventCount": event_count ,"switch": status } )
        } else {
            json!({"switch": status})
        }
    }

    fn create_thadc_payload(payload: &str) -> Value {
        json!({
            "eventCount": i32::from_str_radix(&payload[2..10], 16).unwrap_or(0),
            "temperature": Self::calculate_temperature(i32::from_str_radix(&payload[10..14], 16).unwrap_or(0)),
            "temperatureUnit": "C",
            "humidity": Self::calculate_humidity(i32::from_str_radix(&payload[14..18], 16).unwrap_or(0)),
            "adcMax" : i32::from_str_radix(&payload[22..26], 16).unwrap_or(0),
            "adcIn" : i32::from_str_radix(&payload[26..], 16).unwrap_or(0)
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
