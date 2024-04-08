use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::io::{self, Error as IoError, ErrorKind, Read};
//use std::fmt::Display;
use serde_json::{Value, Number};
use std::cmp::Reverse;
use std::cmp::Ordering;
//use std::collections::{BinaryHeap, HashSet};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
enum dataType {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    F32(f32),
    Text(String),
}

impl PartialEq for dataType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (dataType::U8(a), dataType::U8(b)) => a == b,
            (dataType::I8(a), dataType::I8(b)) => a == b,
            (dataType::U16(a), dataType::U16(b)) => a == b,
            (dataType::I16(a), dataType::I16(b)) => a == b,
            (dataType::U32(a), dataType::U32(b)) => ((a - b) as f64).abs() < f64::EPSILON,
            (dataType::F32(a), dataType::F32(b)) => a == b,
            (dataType::Text(a), dataType::Text(b)) => a == b,
            // Handle other combinations as needed
            _ => false,
        }
    }
}

// Data structure to store edge of mesh
#[derive(Debug)]
struct Edge {
    v_1: u32,
    v_2: u32,
    dist: f32,
    cost: f64,
}

impl Edge {
    fn new(v_1:u32, v_2:u32, dist: f32) -> Self {
        Edge {
            v_1,
            v_2,
            dist,
            cost: f64::MAX,
        }
    }
    fn update_cost(&mut self, new_cost: f64) {
        self.cost = new_cost;
    }
}

// Define the equality trait
impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        self.v_1 == other.v_1 && self.v_2 == other.v_2
    }
}
impl Eq for Edge {}

// Implement the hash trait
impl Hash for Edge {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.v_1.hash(hasher);
        self.v_2.hash(hasher);
    }
}

// Data structure to store face of mesh
#[derive(Debug)]
struct Face {
    v_1: u32,
    v_2: u32,
    v_3: u32,
    // K matrix with respect to ax + by + cz + d = 0
    //     a^2   ab    ac    ad
    //     ab    b^2   bc    bd
    //     ac    bc    c^2   cd
    //     ad    bd    cd    d^2
    // k_matrix stores data in the order: a^2,ab,ac,ad,b^2,bc,bd,c^2,cd,d^2
    k_matrix: Vec<f64>,
}

impl Face {
    fn new(v_1:u32, v_2:u32, v_3:u32, k_matrix: Vec<f64>) -> Self {
        Face {
            v_1,
            v_2,
            v_3,
            k_matrix,
        }
    }
}

// Define the equality trait
impl PartialEq for Face {
    fn eq(&self, other: &Self) -> bool {
        self.v_1 == other.v_1 && self.v_2 == other.v_2 && self.v_3 == other.v_3
    }
}
impl Eq for Face {}

// Implement the hash trait
impl Hash for Face {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.v_1.hash(hasher);
        self.v_2.hash(hasher);
        self.v_3.hash(hasher);
    }
}

// Data structure to store vertex of struct
#[derive(Debug)]
struct Vertex {
    index: u32,
    x: f64,
    y: f64,
    z: f64,
    q_matrix: Vec<f64>,
    edge_set: Vec<u32>,
    face_set: Vec<u32>,
}

impl Vertex {
    // Create a new Vertex with empty lists
    fn new(index:u32, x:f64, y:f64, z:f64) -> Self {
        Vertex {
            index,
            x,
            y,
            z,
            q_matrix: Vec::new(),
            edge_set: Vec::new(),
            face_set: Vec::new(),
        }
    }
}

// Implement the equality trait
impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl Eq for Vertex {}

// Implement the hash trait
impl Hash for Vertex {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.index.hash(hasher);
    }
}

fn main() {
    let path = Path::new("./assets/test.glb");
    let method: String = String::from("percent");
    let limit: u32 = 50;

    println!("Method:{}, Limit:{}", method, limit);

    match path.extension().and_then(|f| f.to_str()).unwrap() {
        "gltf"| "glb" => decimation_gltf(&path, method, limit,),
        &_ => todo!(),
    }
}

fn decimation_gltf(path:&Path, method:String, limit:u32,) {
    // Unpack the data
    let (mut json, mut data) = unpack_gltf(path);
    println!("Json: {}", serde_json::to_string_pretty(&json).unwrap());
    println!("Data: {:?}", data);

    // Initialize list of vertices, edges, and faces using index list
    let mut vertex_list: HashSet<Vertex> = HashSet::new();
    let mut face_list:HashSet<Face> = HashSet::new();
    let mut edge_list:HashSet<Edge> = HashSet::new();
    initialize(&json, &data, &mut vertex_list, & mut edge_list, &mut face_list);

    // Create a list of vertices pair in ascending error order
    //let mut pair_list: BinaryHeap<Pair> = BinaryHeap::new();
    //pair_list = vec_pair(json, data, pair_list);
    //println!("Pair_list: {:?}", pair_list);
    let mut tri_num = 0;
    //let mut tri_num = json.accessors[0].count;
    match method.as_ref() {
        "percent" => tri_num = 5,
        "max" => tri_num = limit,
        &_ => todo!(),
    }

}

fn initialize(json:&Value, data:&Vec<Vec<dataType>>, vertex_list:&mut HashSet<Vertex>, edge_list:&mut HashSet<Edge>, face_list:&mut HashSet<Face>) {
    // If No Index Section, to be implemented.
    let index_layer = json["meshes"][0]["primitives"][0]["indices"].as_i64().unwrap_or(-1);
    let position_layer = json["meshes"][0]["primitives"][0]["attributes"]["POSITION"].as_i64().unwrap_or(-1);
    if index_layer == -1 {
        panic!("No Index Section, to be implemented.");
    }
    let index = data[index_layer as usize].clone();
    let position = data[position_layer as usize].clone();

    // iterate through triangles using index list
    for i in 0..index.len()-2 {
        // Initialize 3 vertices
        let v_1 = Vertex::new(index[i] as u32, );
        // Initialize 3 edges
        // Initialize 1 face
    }
}

//fn vec_pair(json:Value, data:Vec<Vec<dataType>>, pair_list:BinaryHeap<Pair>) -> BinaryHeap<Pair>{
//    pair_list
//}

//fn remove_same_pairs(mut heap: BinaryHeap<Pair>) -> BinaryHeap<Pair> {
//    let mut unique_values = HashSet::new();
//    let mut result = BinaryHeap::new();
//    
//    while let Some(pair) = heap.pop() {
//        if unique_values.insert(pair.clone()) {
//            result.push(pair);
//        }
//    }
//    
//    result
//}

fn unpack_gltf(path:&Path) -> (Value, Vec<Vec<dataType>>){

    let mut file = File::open(path.to_str().unwrap()).expect("Failed to open file");
    let mut buffer = Vec::new();
    //Store accessor's properties
    let mut element_num = Vec::new(); // type
    let mut element_type = Vec::new(); // componentType
    let mut element_length = Vec::new();
    let mut element_count = Vec::new(); // count
    let mut buff_view = Vec::new(); // bufferView

    let mut data_list: Vec<Vec<dataType>> = Vec::new();


    // Read the file contents into a buffer
    file.read_to_end(&mut buffer).expect("Failed to read file");

    // Parse the file contents as JSON
    //if buffer.starts_with(b"glTF") {
        // Extract JSON chunk length (the length of the JSON chunk is stored as a little-endian u32 at byte offset 12)
        let json_length = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]) as usize;

        // Extract the JSON chunk and binary chunk from the buffer
        let json_chunk = &buffer[20..20 + json_length];
        let binary_chunk = &buffer[20 + json_length + 8..];
        
        // Parse the json_chunk as JSON
        let json:Value = serde_json::from_slice(json_chunk).expect("Failed to parse JSON");
        //println!("{}", serde_json::to_string_pretty(&json).unwrap());

        // Store accessor's properties into pre-set vectors
        if let Some(accessors) = json["accessors"].as_array() {
            for accessor in accessors {
                buff_view.push(accessor["bufferView"].as_u64().unwrap_or(0) as usize);
                element_count.push(accessor["count"].as_u64().unwrap_or(0) as usize);
                match accessor["componentType"].as_u64().unwrap_or(0) {
                    5120 => { //BYTE
                        element_type.push("i8");
                        element_length.push(1);
                    }
                    5121 => { // UNSIGNED_BYTE
                        element_type.push("u8");
                        element_length.push(1);
                    }
                    5122 => { // SHORT
                        element_type.push("i16");
                        element_length.push(2);
                    }
                    5123 => { // UNSIGNED_SHORT
                        element_type.push("u16");
                        element_length.push(2);
                    }
                    5125 => { // UNSIGNED_INT
                        element_type.push("u32");
                        element_length.push(4);
                    }
                    5126 => { // FLOAT
                        element_type.push("f32");
                        element_length.push(4);
                    }
                    _ => {
                        element_type.push("");
                        element_length.push(0);
                    }
                };
                match accessor["type"].as_str().unwrap_or("") {
                    "SCALAR" => {
                        element_num.push(1);
                    }
                    "VEC2" => {
                        element_num.push(2);
                    }
                    "VEC3" => {
                        element_num.push(3);
                    }
                    "VEC4" => {
                        element_num.push(4);
                    }
                    "MAT2" => {
                        element_num.push(4);
                    }
                    "MAT3" => {
                        element_num.push(9);
                    }
                    "MAT4" => {
                        element_num.push(16);
                    }
                    _ => {
                        element_num.push(0);
                    }
                };
            }
        }

        let index_buffview = json["meshes"][0]["primitives"][0]["indices"].as_number().unwrap_or( &Number::from(-1));
        let normal_buffview = json["meshes"][0]["primitives"][0]["attributes"]["NORMAL"].as_number().unwrap_or( &Number::from(-1));
        let position_buffview = json["meshes"][0]["primitives"][0]["attributes"]["POSITION"].as_number().unwrap_or( &Number::from(-1));
        let texture_buffview = json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_0"].as_number().unwrap_or( &Number::from(-1));
        let mode = json["meshes"][0]["primitives"][0]["mode"].as_u64().unwrap_or(4) as usize;

        let mut index_list:Vec<dataType> = Vec::new();
        let mut normal_list:Vec<dataType> = Vec::new();
        let mut position_list:Vec<dataType> = Vec::new();
        let mut texture_list:Vec<dataType> = Vec::new();

        let mut view_num = 0;
        if let Some(buffviews) = json["bufferViews"].as_array() {
            for buffview in buffviews {
                println!("buffview: {:?}", buffview);
                let view_offset = buffview["byteOffset"].as_u64().unwrap_or(0) as usize;
                //let view_length = json["bufferViews"][view_num as usize]["byteLength"].as_u64().unwrap_or(0) as usize;
                match &Number::from(view_num) {
                    index_buffview => {
                        let k = view_num;
                        match element_type[k] {
                            "u8" => {
                                for i in 0..element_count[k]{
                                    index_list.push(byte_u8(binary_chunk, view_offset+i));
                                }
                            }
                            "i8" => {
                                for i in 0..element_count[k]{
                                    index_list.push(byte_i8(binary_chunk, view_offset+i));
                                }
                            }
                            "u16" => {
                                for i in 0..element_count[k]{
                                    index_list.push(byte_u16(binary_chunk, view_offset+i*2));
                                }
                            }
                            "i16" => {
                                for i in 0..element_count[k]{
                                    index_list.push(byte_i16(binary_chunk, view_offset+i*2));
                                }
                            }
                            "u32" => {
                                for i in 0..element_count[k]{
                                    index_list.push(byte_u32(binary_chunk, view_offset+i*4));
                                }
                            }
                            "f32" => {
                                for i in 0..element_count[k]{
                                    index_list.push(byte_f32(binary_chunk, view_offset+i*4));
                                }
                            }
                            _ => {}
                        }
                        data_list.push(index_list.clone());
                    },
                    normal_buffview => {
                        let k = view_num;
                        match element_type[k] {
                            "u8" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        normal_list.push(byte_u8(binary_chunk, view_offset+(i*element_num[k])+j));
                                    }
                                }
                            }
                            "i8" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        normal_list.push(byte_i8(binary_chunk, view_offset+(i*element_num[k])+j));
                                    }
                                }
                            }
                            "u16" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        normal_list.push(byte_u16(binary_chunk, view_offset+(i*2*element_num[k])+j*2));
                                    }
                                }
                            }
                            "i16" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        normal_list.push(byte_i16(binary_chunk, view_offset+(i*2*element_num[k])+j*2));
                                    }
                                }
                            }
                            "u32" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        normal_list.push(byte_u32(binary_chunk, view_offset+(i*4*element_num[k])+j*4));
                                    }
                                }
                            }
                            "f32" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        normal_list.push(byte_f32(binary_chunk, view_offset+(i*4*element_num[k])+j*4));
                                    }
                                }
                            }
                            _ => {}
                        }
                        data_list.push(normal_list.clone());
                    },
                    position_buffview => {
                        let k = view_num;
                        match element_type[k] {
                            "u8" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        position_list.push(byte_u8(binary_chunk, view_offset+(i*element_num[k])+j));
                                    }
                                }
                            }
                            "i8" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        position_list.push(byte_i8(binary_chunk, view_offset+(i*element_num[k])+j));
                                    }
                                }
                            }
                            "u16" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        position_list.push(byte_u16(binary_chunk, view_offset+(i*2*element_num[k])+j*2));
                                    }
                                }
                            }
                            "i16" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        position_list.push(byte_i16(binary_chunk, view_offset+(i*2*element_num[k])+j*2));
                                    }
                                }
                            }
                            "u32" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        position_list.push(byte_u32(binary_chunk, view_offset+(i*4*element_num[k])+j*4));
                                    }
                                }
                            }
                            "f32" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        position_list.push(byte_f32(binary_chunk, view_offset+(i*4*element_num[k])+j*4));
                                    }
                                }
                            }
                            _ => {}
                        }
                        data_list.push(position_list.clone());
                    },
                    texture_buffview => {
                        let k = view_num;
                        match element_type[k] {
                            "u8" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        texture_list.push(byte_u8(binary_chunk, view_offset+(i*element_num[k])+j));
                                    }
                                }
                            }
                            "i8" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        texture_list.push(byte_i8(binary_chunk, view_offset+(i*element_num[k])+j));
                                    }
                                }
                            }
                            "u16" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        texture_list.push(byte_u16(binary_chunk, view_offset+(i*2*element_num[k])+j*2));
                                    }
                                }
                            }
                            "i16" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        texture_list.push(byte_i16(binary_chunk, view_offset+(i*2*element_num[k])+j*2));
                                    }
                                }
                            }
                            "u32" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        texture_list.push(byte_u32(binary_chunk, view_offset+(i*4*element_num[k])+j*4));
                                    }
                                }
                            }
                            "f32" => {
                                for i in 0..element_count[k]{
                                    for j in 0..element_num[k]{
                                        texture_list.push(byte_f32(binary_chunk, view_offset+(i*4*element_num[k])+j*4));
                                    }
                                }
                            }
                            _ => {}
                        }
                        data_list.push(texture_list.clone());
                    },

                    _ => {},
                }
                view_num = view_num + 1;
            }
        }
    //}
    return (json, data_list);
}

fn byte_i8(buff:&[u8], x:usize) -> dataType{
    let byte = &buff[x..x+1];
    dataType::I8(i8::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_u8(buff:&[u8], x:usize) -> dataType{
    let byte = &buff[x..x+1];
    dataType::U8(u8::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_i16(buff:&[u8], x:usize) -> dataType{
    let byte = &buff[x..x+2];
    dataType::I16(i16::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_u16(buff:&[u8], x:usize) -> dataType{
    let byte = &buff[x..x+2];
    dataType::U16(u16::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_u32(buff:&[u8], x:usize) -> dataType{
    let byte = &buff[x..x+4];
    dataType::U32(u32::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_f32(buff:&[u8], x:usize) -> dataType{
    let byte = &buff[x..x+4];
    dataType::F32(f32::from_le_bytes(byte.try_into().unwrap()))
}
