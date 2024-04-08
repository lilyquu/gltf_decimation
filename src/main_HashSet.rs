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
use nalgebra::{Vector3, Unit};

#[derive(Clone, Debug)]
enum DataType {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    F32(f32),
}

impl DataType {
    fn to_float(&self) -> f32 {
        match self {
            DataType::U8(value) => *value as f32,
            DataType::I8(value) => *value as f32,
            DataType::U16(value) => *value as f32,
            DataType::I16(value) => *value as f32,
            DataType::U32(value) => *value as f32,
            DataType::F32(value) => *value,
        }
    }

    fn to_uint(&self) -> u32 {
        match self {
            DataType::U8(value) => *value as u32,
            DataType::I8(value) => *value as u32,
            DataType::U16(value) => *value as u32,
            DataType::I16(value) => *value as u32,
            DataType::U32(value) => *value,
            DataType::F32(value) => *value as u32,
        }
    }

    fn to_int(&self) -> i64 {
        match self {
            DataType::U8(value) => *value as i64,
            DataType::I8(value) => *value as i64,
            DataType::U16(value) => *value as i64,
            DataType::I16(value) => *value as i64,
            DataType::U32(value) => *value as i64,
            DataType::F32(value) => *value as i64,
        }
    }
}

impl PartialEq for DataType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DataType::U8(a), DataType::U8(b)) => a == b,
            (DataType::I8(a), DataType::I8(b)) => a == b,
            (DataType::U16(a), DataType::U16(b)) => a == b,
            (DataType::I16(a), DataType::I16(b)) => a == b,
            (DataType::U32(a), DataType::U32(b)) => ((a - b) as f32).abs() < f32::EPSILON,
            (DataType::F32(a), DataType::F32(b)) => a == b,
            //(DataType::Text(a), DataType::Text(b)) => a == b,
            // Handle other combinations as needed
            _ => false,
        }
    }
}

// Data structure to store edge of mesh
#[derive(Debug, Clone)]
struct Edge {
    v_1: Vertex,
    v_2: Vertex,
    dist: f32,
    cost: f32,
}

impl Edge {
    fn new(v_1:Vertex, v_2:Vertex, dist: f32) -> Self {
        Edge {
            v_1,
            v_2,
            dist,
            cost: f32::MAX,
        }
    }
    fn update_cost(&mut self) {
        // Cost: q11x^2 + 2q12xy + 2q13xz + 2q14x + q22y^2
        //       + 2q23yz + 2q24y + q33z^2 + 2q34z + q44
        // Q: 0 1 2 3
        //    1 4 5 6
        //    2 5 7 8
        //    3 6 8 9
        let q1 = &self.v_1.q_matrix;
        let q2 = &self.v_2.q_matrix;
        let cost1 = q1[0]*self.v_1.x*self.v_1.x +
                    2.0*q1[1]*self.v_1.x*self.v_1.y + 
                    2.0*q1[2]*self.v_1.x*self.v_1.z + 
                    2.0*q1[3]*self.v_1.x + 
                    q1[4]*self.v_1.y*self.v_1.y + 
                    2.0*q1[5]*self.v_1.y*self.v_1.z + 
                    2.0*q1[6]*self.v_1.y + 
                    q1[7]*self.v_1.z*self.v_1.z + 
                    2.0*q1[8]*self.v_1.z + q1[9];
        let cost2 = q2[0]*self.v_2.x*self.v_2.x +
                    2.0*q2[1]*self.v_2.x*self.v_2.y + 
                    2.0*q2[2]*self.v_2.x*self.v_2.z + 
                    2.0*q2[3]*self.v_2.x + 
                    q2[4]*self.v_2.y*self.v_2.y + 
                    2.0*q2[5]*self.v_2.y*self.v_2.z + 
                    2.0*q2[6]*self.v_2.y + 
                    q2[7]*self.v_2.z*self.v_2.z + 
                    2.0*q2[8]*self.v_2.z + q2[9];
        self.cost = cost1 + cost2;
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
#[derive(Debug, Clone)]
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
    k_matrix: Vec<f32>,
}

impl Face {
    fn new(v_1:u32, v_2:u32, v_3:u32, k_matrix:Vec<f32>) -> Self {
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
#[derive(Debug, Clone)]
struct Vertex {
    index: u32,
    x: f32,
    y: f32,
    z: f32,
    q_matrix: Vec<f32>,
    edge_set: HashSet<Edge>,
    face_set: HashSet<Face>,
}

impl Vertex {
    // Create a new Vertex with empty lists
    fn new(index:u32, x:f32, y:f32, z:f32) -> Self {
        Vertex {
            index,
            x,
            y,
            z,
            q_matrix: Vec::with_capacity(10),
            edge_set: HashSet::new(),
            face_set: HashSet::new(),
        }
    }

    // Calculate the distance between two vertices
    fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    // Adding edge to edge_set
    fn add_edge(&mut self, new_edge:Edge) {
        self.edge_set.insert(new_edge);
    }

    // Adding face to face_set
    fn add_face(&mut self, new_face:Face) {
        self.face_set.insert(new_face);
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
    initialize(&json, &data, vertex_list, edge_list, face_list);

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

fn initialize(json:&Value, data:&Vec<Vec<DataType>>, mut vertex_list:HashSet<Vertex>, mut edge_list:HashSet<Edge>, mut face_list:HashSet<Face>) {
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
        let mut v_1 = Vertex::new(index[i].to_uint(), position[i*3].to_float(), position[i*3+1].to_float(), position[i*3+2].to_float());
        let mut v_2 = Vertex::new(index[i+1].to_uint(), position[(i+1)*3].to_float(), position[(i+1)*3+1].to_float(), position[(i+1)*3+2].to_float());
        let mut v_3 = Vertex::new(index[i+2].to_uint(), position[(i+2)*3].to_float(), position[(i+2)*3+1].to_float(), position[(i+2)*3+2].to_float());

        // Initialize 3 edges
        let e_1 = Edge::new(v_1.clone(), v_2.clone(), v_1.distance_to(&v_2));
        let e_2 = Edge::new(v_2.clone(), v_3.clone(), v_2.distance_to(&v_3));
        let e_3 = Edge::new(v_3.clone(), v_1.clone(), v_3.distance_to(&v_1));

        // Initialize 1 face
        let f = set_FEQ(&v_1, &v_2, &v_3);

        // Setting up facelist and edgelist for vertices
        v_1.add_face(f.clone());
        v_1.add_edge(e_1.clone());
        v_1.add_edge(e_3.clone());

        v_2.add_face(f.clone());
        v_2.add_edge(e_1.clone());
        v_2.add_edge(e_2.clone());

        v_3.add_face(f.clone());
        v_3.add_edge(e_2.clone());
        v_3.add_edge(e_3.clone());

        vertex_list.insert(v_1);
        vertex_list.insert(v_2);
        vertex_list.insert(v_3);
        edge_list.insert(e_1);
        edge_list.insert(e_2);
        edge_list.insert(e_3);
        face_list.insert(f);
    }

    // Compute and update the Q matrix for each vertices
    let vertex_num = vertex_list.len();
    for ver in vertex_list.iter() {
        vertex_list.insert(update_q_matrix(vertex_list.get(ver)));
    }


    // Compute and update the cost for each edge

}

fn update_q_matrix(mut vertex:Vertex) -> Vertex{
    // Q matrix position reference
    // 0  1  2  3
    // 4  5  6  7
    // 8  9  10 11
    // 12 13 14 15
    // K matrix position reference
    // 0 1 2 3 
    // 1 4 5 6
    // 2 5 7 8
    // 3 6 8 9
    let mut q:Vec<f32> = vec![0.0; 10];
    for face in vertex.face_set.iter() {
        for i in 0..10 {
            q[i] = q[i] + face.k_matrix[i];
        }
    }
    for j in 0..10 {
        vertex.q_matrix.push(q[j]);
    }
    vertex
}

// Implement trait to form a k matrix(Fundamental Error Quadric) for the face
fn set_FEQ(v1:&Vertex, v2:&Vertex, v3:&Vertex) -> Face {
    // Calculate vectors on the plane
    let v1v2 = Vector3::new(v2.x - v1.x, v2.y - v1.y, v2.z - v1.z);
    let v1v3 = Vector3::new(v3.x - v1.x, v3.y - v1.y, v3.z - v1.z);
    // Calculate the normal vector to the plane
    let normal = v1v2.cross(&v1v3);
    // Normalize the normal vector
    let normal = Unit::new_normalize(normal);
    // Calculate the distance from the origin to the plane
    let d = -normal.dot(&Vector3::new(v1.x, v1.y, v1.z));
    // Extract components of the normalized normal vector
    let (a, b, c) = (normal[0], normal[1], normal[2]);

    let mut k = Vec::with_capacity(10);

    //     a^2   ab    ac    ad
    //     ab    b^2   bc    bd
    //     ac    bc    c^2   cd
    //     ad    bd    cd    d^2
    k.push(a*a);
    k.push(a*b);
    k.push(a*c);
    k.push(a*d);
    k.push(b*b);
    k.push(b*c);
    k.push(b*d);
    k.push(c*c);
    k.push(c*d);
    k.push(d*d);

    Face::new(v1.index, v2.index, v3.index, k)
}


//fn vec_pair(json:Value, data:Vec<Vec<DataType>>, pair_list:BinaryHeap<Pair>) -> BinaryHeap<Pair>{
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

fn unpack_gltf(path:&Path) -> (Value, Vec<Vec<DataType>>){

    let mut file = File::open(path.to_str().unwrap()).expect("Failed to open file");
    let mut buffer = Vec::new();
    //Store accessor's properties
    let mut element_num = Vec::new(); // type
    let mut element_type = Vec::new(); // componentType
    let mut element_length = Vec::new();
    let mut element_count = Vec::new(); // count
    let mut buff_view = Vec::new(); // bufferView

    let mut data_list: Vec<Vec<DataType>> = Vec::new();


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

        let mut index_list:Vec<DataType> = Vec::new();
        let mut normal_list:Vec<DataType> = Vec::new();
        let mut position_list:Vec<DataType> = Vec::new();
        let mut texture_list:Vec<DataType> = Vec::new();

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

fn byte_i8(buff:&[u8], x:usize) -> DataType{
    let byte = &buff[x..x+1];
    DataType::I8(i8::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_u8(buff:&[u8], x:usize) -> DataType{
    let byte = &buff[x..x+1];
    DataType::U8(u8::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_i16(buff:&[u8], x:usize) -> DataType{
    let byte = &buff[x..x+2];
    DataType::I16(i16::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_u16(buff:&[u8], x:usize) -> DataType{
    let byte = &buff[x..x+2];
    DataType::U16(u16::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_u32(buff:&[u8], x:usize) -> DataType{
    let byte = &buff[x..x+4];
    DataType::U32(u32::from_le_bytes(byte.try_into().unwrap()))
}
fn byte_f32(buff:&[u8], x:usize) -> DataType{
    let byte = &buff[x..x+4];
    DataType::F32(f32::from_le_bytes(byte.try_into().unwrap()))
}
