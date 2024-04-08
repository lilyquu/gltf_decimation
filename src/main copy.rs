use std::path::Path;
use std::fs::File;
use serde_json::Value;
use std::error::Error;
use std::io::{self, Error as IoError, ErrorKind, Read};
//use std::fmt::Display;

fn main() {
    let path = Path::new("./assets/cube.glb");
    let method: String = String::from("percent");
    let limit: u32 = 50;

    println!("Method:{}, Limit:{}", method, limit);

    match path.extension().and_then(|f| f.to_str()).unwrap() {
        "gltf"| "glb" => decimation_gltf(&path, method, limit,),
        &_ => todo!(),
    }
}

fn decimation_gltf(path:&Path, method:String, limit:u32,) {

    let json = unpack_gltf(path);
    let mut tri_num = 0;
    //let mut tri_num = json.accessors[0].count;
    match method.as_ref() {
        "percent" => tri_num = 5,
        "max" => tri_num = limit,
        &_ => todo!(),
    }

}

fn unpack_gltf(path:&Path) -> Value{

    let mut file = File::open(path.to_str().unwrap()).expect("Failed to open file");
    let mut buffer = Vec::new();
    
    // Read the file contents into a buffer
    file.read_to_end(&mut buffer).expect("Failed to read file");

    // Parse the file contents as JSON
    if buffer.starts_with(b"glTF") {
        // Extract JSON chunk length (the length of the JSON chunk is stored as a little-endian u32 at byte offset 12)
        let json_length = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]) as usize;

        // Extract the JSON chunk from the buffer
        let json_chunk = &buffer[20..20 + json_length];
        let binary_chunk = &buffer[20 + json_length..];
        
        // Parse the JSON chunk as JSON
        let json:Value = serde_json::from_slice(json_chunk).expect("Failed to parse JSON");
        println!("{}", serde_json::to_string_pretty(&json).unwrap());

        // Decode binary data based on glTF information
        match decode_binary_chunk(binary_chunk, &json) {
            Ok(data) => {
                println!("Decoded data: {:?}", data);
                let data_u8 = data;
                let accessors = json.pointer("/accessors").and_then(|v| v.as_array()).unwrap();
                let buffer_views = json.pointer("/bufferViews").and_then(|v| v.as_array()).unwrap();
                let mut data_size = Vec::with_capacity(accessors.len()); 
                for accessor in accessors {
                    match accessor["type"].as_str().unwrap_or("") {
                        "SCALAR" => {
                            data_size.push(2);
                        }
                        "VEC2" => {
                            data_size.push(4);
                        }
                        "VEC3" => {
                            data_size.push(4);
                        }
                        "VEC4" => {
                            data_size.push(4);
                        }
                        _ => {
                            data_size.push(0);
                        }
                    };
                    //let mut data_u16: Vec<&[u8]> = Vec::with_capacity(data_u8.len()/data_type_size); 
                }
                let mut data_u16 = Vec::with_capacity(data_u8[3].len()/2); 
                let mut k = 0;
                while k < data_u8[3].len(){
                    data_u16.push(((data_u8[3][k+1] as u16) << 8) | data_u8[3][k] as u16);
                    k = k+2;
                }
                println!("{:?}", data_u16);
                println!("{:?}", data_u16.len());

                let x = 0;
                let mut data_f32 = Vec::with_capacity(data_u8[x].len()/4); 
                let mut j = 0;
                while j < data_u8[x].len(){
                    let byte1 = data_u8[x][j+3].to_le_bytes();
                    let byte2 = data_u8[x][j+2].to_le_bytes();
                    let byte3 = data_u8[x][j+1].to_le_bytes();
                    let byte4 = data_u8[x][j].to_le_bytes();
                    let mut bits = [0; 4];
                    bits[..1].copy_from_slice(&byte1);
                    bits[1..2].copy_from_slice(&byte2);
                    bits[2..3].copy_from_slice(&byte3);
                    bits[3..4].copy_from_slice(&byte4);

                    data_f32.push(f32::from_le_bytes(bits)); 
                    j = j+4;
                }
                println!("{:?}", data_f32);
                println!("{:?}", data_f32.len());
            }
            Err(err) => {
                eprintln!("Error decoding binary chunk: {}", err);
            }
        }
        return json;
    } else {
        // If it's not a glb file, assume it's a glTF file and parse it directly as JSON
        let json:Value = serde_json::from_slice(&buffer).expect("Failed to parse JSON");
        println!("{:?}", json);
        return json;
    };
}

fn decode_binary_chunk<'a>(binary_chunk: &'a [u8], json: &'a Value) -> Result<Vec<&'a [u8]>, Box<dyn Error + 'a>> {
    // Access accessors and buffer views from the glTF JSON
    let accessors = json.pointer("/accessors").and_then(|v| v.as_array()).ok_or(io::Error::new(io::ErrorKind::InvalidData, "Missing accessors"))?;
    let buffer_views = json.pointer("/bufferViews").and_then(|v| v.as_array()).ok_or(io::Error::new(io::ErrorKind::InvalidData, "Missing bufferViews"))?;
    let mut data: Vec<&[u8]> = Vec::with_capacity(accessors.len()); 
    // Iterate over accessors to decode binary data
    for accessor in accessors {
        let buffer_view_index = accessor["bufferView"].as_u64().ok_or(io::Error::new(io::ErrorKind::InvalidData, "Missing bufferView index"))? as usize;
        let buffer_view = buffer_views.get(buffer_view_index).ok_or(io::Error::new(io::ErrorKind::InvalidData, "Invalid bufferView index"))?;
        
        let offset = buffer_view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let length = buffer_view["byteLength"].as_u64().unwrap_or(0) as usize;
        let data_type_size = match accessor["type"].as_str().unwrap_or("") {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            _ => return Err(Box::new(io::Error::new(io::ErrorKind::InvalidData, "Invalid accessor type"))),
        };
        data.push(&binary_chunk[offset..offset + length]);
    }
    println!("Decoded data: {:?}", data);
    Ok(data)
}
