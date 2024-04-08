use std::path::Path;
use std::env;
use std::fs::File;
use std::io::{self, Write, Read, prelude::*};
use serde_json::{json, Value, Number, to_vec};
use std::hash::{Hash, Hasher};
use nalgebra::{Vector2, Vector3, Vector4, Unit};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::{Reverse, Ordering};

#[derive(Debug)]
struct Remove(u32, u32, f32);

impl PartialEq for Remove {
    fn eq(&self, other: &Self) -> bool {
        self.2 == other.2
    }
}

impl Eq for Remove {}

impl PartialOrd for Remove {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.2.partial_cmp(&other.2)
    }
}

impl Ord for Remove {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, Clone)]
struct Prim {
    bufferView: u32,
    byteOffset: u32,
    componentType: u32,
    normalized: bool,
    count: u32,
    prim_type: String,
}

impl Prim {
    fn new(bufferView:u32, byteOffset:u32, componentType:u32,
            normalized:bool, count:u32, prim_type:String) -> Self {
        Prim {
            bufferView,
            byteOffset,
            componentType,
            normalized,
            count,
            prim_type,
        }
    }
}

#[derive(Debug, Clone)]
struct View {
    buffer: u32,
    byteLength: u32,
    byteOffset: u32,
}

impl View {
    fn new(buffer:u32, byteLength:u32, byteOffset:u32) -> Self {
        View {
            buffer,
            byteLength,
            byteOffset,
        }
    }
}

#[derive(Debug, Clone)]
struct Vertex {
    edge_set: Vec<Vector2<u32>>,
    face_set: Vec<Vector3<u32>>,
    q_matrix: Vec<f32>,
}

impl Vertex {
    fn new(edge_set:Vec<Vector2<u32>>, face_set:Vec<Vector3<u32>>,
             q_matrix:Vec<f32>) -> Self {
        Vertex {
            edge_set,
            face_set,
            q_matrix,
        }
    }
}

fn main() {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the correct number of arguments are provided
    if args.len() != 4 {
        eprintln!("Usage: {} <file_path> <method> <limit>", args[0]);
    }
    let path = Path::new(&args[1]);
    let method = &args[2];
    let limit: f64 = match args[3].parse() {
        Ok(n) => n,
        Err(..) => todo!(),
    };

    // Takes input value
    //let path = Path::new("./assets/test.glb");
    //let method = "percent";
    //let limit = 0.1;

    // Exam file format
    match path.extension().and_then(|f| f.to_str()).unwrap() {
        "gltf"| "glb" => decimation_gltf(&path, &method, limit),
        &_ => todo!(),
    }
}

fn decimation_gltf(path:&Path, method:&str, limit:f64) {
    // Unpack the data into json and binary chunks
    // Process the data to return informations in HashMaps (not supporting tangent, TEXCOORD_n, COLOR_n, Joints_n, Weight_n)
    let (mut index_list,  mut normal_list, mut position_list, mut tangent_list, mut texcoord_0_list, mut json, mut primitives) = unpack_gltf(path);
    let (mut vertex_list, tri_num) = initialize(&index_list,  &normal_list, &position_list, &tangent_list, &texcoord_0_list);
    let mut valid_edge = get_valid_edge(&index_list, &position_list, &vertex_list);
    let mut remove_list: BinaryHeap<Reverse<Remove>> = BinaryHeap::new();
    // Iterate over key-value pairs in valid_edge and insert them into remove_list
    for ((u, v), cost) in valid_edge.iter() {
        let remove = Remove(*u, *v, *cost);
        remove_list.push(Reverse(remove));
    }
    // Set up stop criteria
    let mut goal = valid_edge.len();
    match method.as_ref() {
        "percent" => goal = std::cmp::min(goal, (tri_num as f64 * (1.0 - limit)) as usize),
        "max" => goal = std::cmp::min(goal, (tri_num - limit as u32) as usize),
        &_ => todo!(),
    }
    // Start iteration of vertex removement
    let mut index_ref = Vec::with_capacity(position_list.len());
    for i in 0..position_list.len() {
        index_ref.push(true);
    }
    while goal > 0 {
        println!("need reduce: {}", goal);
        println!("{}", remove_list.len());
        let mut remove = remove_list.pop().unwrap().0;
        println!("{:?}", remove);
        while !index_ref[remove.0 as usize] && !index_ref[remove.1 as usize] {
            remove = remove_list.pop().unwrap().0;
            println!("{:?}", remove);
        }
        index_ref[remove.1 as usize] = false;
        // vertex
        let v1 = vertex_list.get(&remove.0).unwrap();
        let v2 = vertex_list.get(&remove.1).unwrap();
        let mut q_matrix = Vec::new();
        for i in 0..10 {
            q_matrix.push(v1.q_matrix[i] + v2.q_matrix[i]);
        }
        let mut edge_set:Vec<Vector2<u32>> = Vec::new();
        for edge in &v1.edge_set {
            if edge[0] == remove.0 {
                if edge[1] == remove.1 {
                    continue;
                } else {
                    edge_set.push(*edge);
                }
            } else if edge[1] == remove.0 {
                if edge[0] == remove.1 {
                    continue;
                } else {
                    edge_set.push(*edge);
                }
            } else if edge[0] == remove.1 {
                edge_set.push(Vector2::new(remove.0, edge[1]));
            } else if edge[1] == remove.1 {
                edge_set.push(Vector2::new(edge[1], remove.0));
            } else {
                edge_set.push(*edge);
            }
        }
        for edge in &v2.edge_set {
            if edge[0] == remove.0 {
                if edge[1] == remove.1 {
                    continue;
                } else {
                    edge_set.push(*edge);
                }
            } else if edge[1] == remove.0 {
                if edge[0] == remove.1 {
                    continue;
                } else {
                    edge_set.push(*edge);
                }
            } else if edge[0] == remove.1 {
                edge_set.push(Vector2::new(remove.0, edge[1]));
            } else if edge[1] == remove.1 {
                edge_set.push(Vector2::new(edge[1], remove.0));
            } else {
                edge_set.push(*edge);
            }
        }
        let mut face_set:Vec<Vector3<u32>> = Vec::new();
        for face in &v1.face_set {
            if face[0] == remove.0 {
                if face[1] == remove.1 || face[2] == remove.1 {
                    continue;
                } else {
                    face_set.push(*face);
                }
            } else if face[1] == remove.0 {
                if face[0] == remove.1 || face[2] == remove.1 {
                    continue;
                } else {
                    face_set.push(*face);
                }
            } else if face[2] == remove.0 {
                if face[0] == remove.1 || face[1] == remove.1 {
                    continue;
                } else {
                    face_set.push(*face);
                }
            } else if face[0] == remove.1 {
                face_set.push(Vector3::new(remove.0, face[1], face[2]));
            } else if face[1] == remove.1 {
                face_set.push(Vector3::new(face[0], remove.0, face[2]));
            } else if face[2] == remove.1 {
                face_set.push(Vector3::new(face[0], face[1], remove.0));
            } else {
                face_set.push(*face);
            }
        }
        for face in &v2.face_set {
            if face[0] == remove.0 {
                if face[1] == remove.1 || face[2] == remove.1 {
                    continue;
                } else {
                    face_set.push(*face);
                }
            } else if face[1] == remove.0 {
                if face[0] == remove.1 || face[2] == remove.1 {
                    continue;
                } else {
                    face_set.push(*face);
                }
            } else if face[2] == remove.0 {
                if face[0] == remove.1 || face[1] == remove.1 {
                    continue;
                } else {
                    face_set.push(*face);
                }
            } else if face[0] == remove.1 {
                face_set.push(Vector3::new(remove.0, face[1], face[2]));
            } else if face[1] == remove.1 {
                face_set.push(Vector3::new(face[0], remove.0, face[2]));
            } else if face[2] == remove.1 {
                face_set.push(Vector3::new(face[0], face[1], remove.0));
            } else {
                face_set.push(*face);
            }
        }
        vertex_list.remove(&remove.0);
        vertex_list.remove(&remove.1);
        vertex_list.insert(remove.0, Vertex::new(edge_set, face_set, q_matrix));
        // position
        let p1 = position_list.get(&remove.0).unwrap();
        let p2 = position_list.get(&remove.1).unwrap();
        let new_p = Vector3::new((p1[0]+p2[0])/2.0, (p1[1]+p2[1])/2.0, (p1[2]+p2[2])/2.0);
        position_list.remove(&remove.0);
        position_list.remove(&remove.1);
        position_list.insert(remove.0, new_p);
        // normal
        let n1 = normal_list.get(&remove.0).unwrap();
        let n2 = normal_list.get(&remove.1).unwrap();
        // Correct later
        let new_n = Vector3::new((n1[0]+n2[0])/2.0, (n1[1]+n2[1])/2.0, (n1[2]+n2[2])/2.0);
        normal_list.remove(&remove.0);
        normal_list.remove(&remove.1);
        normal_list.insert(remove.0, new_n);
        // tangent
        //let t1 = tangent_list.get(&remove.0).unwrap();
        //let t2 = tangent_list.get(&remove.1).unwrap();
        // texcoord_0
        let tex01 = texcoord_0_list.get(&remove.0).unwrap();
        let tex02 = texcoord_0_list.get(&remove.1).unwrap();
        let new_tex = Vector2::new((tex01[0]+tex02[0])/2.0, (tex01[1]+tex02[1])/2.0);
        texcoord_0_list.remove(&remove.0);
        texcoord_0_list.remove(&remove.1);
        texcoord_0_list.insert(remove.0, new_tex);
        // index
        let mut in_list = Vec::new();
        for i in 0..index_list.len() {
            if index_list[i] == remove.1 {
                index_list[i] = remove.0;
            }
        }
        for i in 0..index_list.len()/3 {
            if (index_list[i*3] == remove.0 && index_list[i*3+1] == remove.0) || 
                (index_list[i*3+1] == remove.0 && index_list[i*3+2] == remove.0) ||
                (index_list[i*3] == remove.0 && index_list[i*3+2] == remove.0) {
                goal = goal-1;
            } else {
                in_list.push(index_list[i*3]);
                in_list.push(index_list[i*3+1]);
                in_list.push(index_list[i*3+2]);
            }
        }
        index_list.clear();
        index_list = in_list;
        println!("new index list: {:?}", index_list);

        // Update new cost
        let mut temp_list: BinaryHeap<Reverse<Remove>> = BinaryHeap::new();
        while let Some(mut value) = remove_list.pop() {
            if value.0.0 == remove.1 {
                if value.0.1 == remove.0 {
                    continue;
                } else {
                    value.0.0 = remove.0;
                    //println!("{}, {}, {:?}", value.0.0, value.0.1, position_list);
                    if index_ref[value.0.0 as usize] && index_ref[value.0.1 as usize] {
                        let p1 = position_list.get(&value.0.0).unwrap();
                        let p2 = position_list.get(&value.0.1).unwrap();
                        let new_pos = Vector3::new(p1[0]+p2[0]/2.0, p1[1]+p2[1]/2.0, p1[2]+p2[2]/2.0);
                        let q1 = &vertex_list.get(&value.0.0).unwrap().q_matrix;
                        let q2 = &vertex_list.get(&value.0.1).unwrap().q_matrix;
                        temp_list.push(Reverse(Remove(value.0.0, value.0.1, update_cost(q1, q2, &new_pos))));
                    }
                }
            } else if value.0.1 == remove.1 {
                if value.0.0 == remove.0 {
                    continue;
                } else {
                    value.0.1 = remove.0;
                    if index_ref[value.0.0 as usize] && index_ref[value.0.1 as usize]  {
                        let p1 = position_list.get(&value.0.0).unwrap();
                        let p2 = position_list.get(&value.0.1).unwrap();
                        let new_pos = Vector3::new(p1[0]+p2[0]/2.0, p1[1]+p2[1]/2.0, p1[2]+p2[2]/2.0);
                        let q1 = &vertex_list.get(&value.0.0).unwrap().q_matrix;
                        let q2 = &vertex_list.get(&value.0.1).unwrap().q_matrix;
                        temp_list.push(Reverse(Remove(value.0.0, value.0.1, update_cost(q1, q2, &new_pos))));
                    }
                }
            } else if value.0.0 == remove.0 || value.0.1 == remove.0 {
                if index_ref[value.0.0 as usize] && index_ref[value.0.1 as usize]  {
                    let p1 = position_list.get(&value.0.0).unwrap();
                    let p2 = position_list.get(&value.0.1).unwrap();
                    let new_pos = Vector3::new(p1[0]+p2[0]/2.0, p1[1]+p2[1]/2.0, p1[2]+p2[2]/2.0);
                    let q1 = &vertex_list.get(&value.0.0).unwrap().q_matrix;
                    let q2 = &vertex_list.get(&value.0.1).unwrap().q_matrix;
                    temp_list.push(Reverse(Remove(value.0.0, value.0.1, update_cost(q1, q2, &new_pos))));
                }
            } else {
                if index_ref[value.0.0 as usize] && index_ref[value.0.1 as usize]  {
                    temp_list.push(value);
                }
            }
        }
        remove_list = temp_list;
    }
    // Finished decimation
    println!("indices elements numnber is: {}. \n{:?}", index_list.len(), index_list);
    println!("normal elements numnber is: {}. \n{:?}", normal_list.len(), normal_list);
    println!("position elements numnber is: {}. \n{:?}", position_list.len(), position_list);
    println!("texcoord_0 elements numnber is: {}. \n{:?}", texcoord_0_list.len(), texcoord_0_list);

    // Write the new glb file
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    let file = repack_gltf(json, index_ref, index_list, &normal_list, &position_list, &texcoord_0_list, &primitives);
}

fn repack_gltf(mut json:Value, index_ref:Vec<bool>, index_list:Vec<u32>,
                normal_list:&HashMap<u32, Vector3<f32>>, position_list:&HashMap<u32, Vector3<f32>>,
                texcoord_0_list:&HashMap<u32, Vector2<f32>>, 
                primitives:&HashMap<String,Prim>) -> Result<File, std::io::Error> {
    let filename = "output.glb";
    let version = 2;

    // edit json part
    let index = if let Some(mut index) = json["meshes"][0]["primitives"][0]["indices"].as_i64(){
        println!("Indices at : {}", index);
        index
    } else {
        println!("No Indices");
        -1
    };
    let normal = if let Some(normal) = json["meshes"][0]["primitives"][0]["attributes"]["NORMAL"].as_i64(){
        println!("NORMAL at : {}", normal);
        normal
    } else {
        println!("No NORMAL");
        -1
    };
    let position = if let Some(position) = json["meshes"][0]["primitives"][0]["attributes"]["POSITION"].as_i64(){
        println!("POSITION at : {}", position);
        position
    } else {
        println!("No POSITION");
        -1
    };
    let tangent = if let Some(tangent) = json["meshes"][0]["primitives"][0]["attributes"]["TANGENT"].as_i64(){
        println!("TANGENT at : {}", tangent);
        tangent
    } else {
        println!("No TANGENT");
        -1
    };
    let texcoord_0 = if let Some(texcoord_0) = json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_0"].as_i64(){
        println!("TEXCOORD_0 at : {}", texcoord_0);
        texcoord_0
    } else {
        println!("No TEXCOORD_0");
        -1
    };

    json["accessors"][index as usize]["count"] = json!(index_list.len());
    json["accessors"][normal as usize]["count"] = json!(normal_list.len());
    json["accessors"][position as usize]["count"] = json!(position_list.len());
    json["accessors"][texcoord_0 as usize]["count"] = json!(texcoord_0_list.len());

    let indices = primitives.get("indices").unwrap();
    let NORMAL = primitives.get("NORMAL").unwrap();
    let POSITION = primitives.get("POSITION").unwrap();
    let TEXCOORD_0 = primitives.get("TEXCOORD_0").unwrap();

    json["bufferViews"][indices.bufferView as usize]["byteLength"] = json!(index_list.len()*12);
    json["bufferViews"][NORMAL.bufferView as usize]["byteLength"] = json!(normal_list.len()*12);
    json["bufferViews"][POSITION.bufferView as usize]["byteLength"] = json!(position_list.len()*12);
    json["bufferViews"][TEXCOORD_0.bufferView as usize]["byteLength"] = json!(texcoord_0_list.len()*8);

    let mut offset = 0;
    for i in 0..4 {
        if index == i {
            json["bufferViews"][indices.bufferView as usize]["byteOffset"] = json!(offset);
            offset = offset + index_list.len()*12;
        } else if normal == i {
            json["bufferViews"][NORMAL.bufferView as usize]["byteOffset"] = json!(offset);
            offset = offset + normal_list.len()*12;
        } else if position == i {
            json["bufferViews"][POSITION.bufferView as usize]["byteOffset"] = json!(offset);
            offset = offset + position_list.len()*12;
        } else if texcoord_0 == i {
            json["bufferViews"][TEXCOORD_0.bufferView as usize]["byteOffset"] = json!(offset);
            offset = offset + texcoord_0_list.len()*8;
        }
    }
    json["buffers"][0]["byteLength"] = json!(offset);

    let json_data = &to_vec(&json).unwrap();
    let json_chunk_length = json_data.len() as u32;

    // TODO
    // write the binary part
    let mut binary_data = Vec::new();
    let mut new_index_list = Vec::new();
    let mut new_index = HashMap::new();
    for i in 0..4 {
        if indices.bufferView == i {
            let mut k = 0;
            for i in 0..index_ref.len() {
                if index_ref[i] == true {
                    new_index.insert(i, k);
                    k = k+1;
                }
            }
            for i in 0..index_list.len() {
                new_index_list.push(*new_index.get(&(index_list[i] as usize)).unwrap());
                match primitives.get("indices").unwrap().componentType {
                    5121 => {
                        let bytes = (new_index_list[i] as u8).to_le_bytes();
                        binary_data.extend_from_slice(&bytes);
                    }
                    5123 => {
                        let bytes = (new_index_list[i] as u16).to_le_bytes();
                        binary_data.extend_from_slice(&bytes);
                    }
                    5125 => {
                        let bytes = (new_index_list[i] as u32).to_le_bytes();
                        binary_data.extend_from_slice(&bytes);
                    }
                    5120 => {
                        let bytes = (new_index_list[i] as i8).to_le_bytes();
                        binary_data.extend_from_slice(&bytes);
                    }
                    5122 => {
                        let bytes = (new_index_list[i] as i16).to_le_bytes();
                        binary_data.extend_from_slice(&bytes);
                    }
                    _ => todo!()
                }
            }
            //for &value in &f32_values {
            //    let bytes = value.to_le_bytes();
            //    binary_data.extend_from_slice(&bytes);
            //}
        } else if NORMAL.bufferView == i {
            for i in 0..new_index_list.len() {
                let nor = normal_list.get(&(i as u32));
                let bytesx = nor.unwrap().x.to_le_bytes();
                binary_data.extend_from_slice(&bytesx);
                let bytesy = nor.unwrap().y.to_le_bytes();
                binary_data.extend_from_slice(&bytesy);
                let bytesz = nor.unwrap().z.to_le_bytes();
                binary_data.extend_from_slice(&bytesz);
            }
        } else if POSITION.bufferView == i {
            for i in 0..new_index_list.len() {
                let pos = position_list.get(&(i as u32));
                let bytesx = pos.unwrap().x.to_le_bytes();
                binary_data.extend_from_slice(&bytesx);
                let bytesy = pos.unwrap().y.to_le_bytes();
                binary_data.extend_from_slice(&bytesy);
                let bytesz = pos.unwrap().z.to_le_bytes();
                binary_data.extend_from_slice(&bytesz);
            }
        } else if TEXCOORD_0.bufferView == i {
            for i in 0..new_index_list.len() {
                let tex0 = texcoord_0_list.get(&(i as u32));
                let bytesx = tex0.unwrap().x.to_le_bytes();
                binary_data.extend_from_slice(&bytesx);
                let bytesy = tex0.unwrap().y.to_le_bytes();
                binary_data.extend_from_slice(&bytesy);
            }
        }
    }

    let binary_chunk_length = binary_data.len() as u32;
    let total_length = 12 + 8 + json_chunk_length + 8 + binary_chunk_length;

    let file = write_file(filename, version, total_length, json_chunk_length, 
                            json_data, binary_chunk_length, &binary_data);
    file
}

fn write_file(filename: &str, version: u32, total_length: u32, json_chunk_length: u32, 
                json_data: &[u8], binary_chunk_length: u32, binary_data: &[u8]) -> io::Result<File> {
    let mut file = File::create(filename)?;
    
    // Write header
    file.write_all(&0x46546C67u32.to_le_bytes())?; // Magic number "glTF"
    file.write_all(&version.to_le_bytes())?;       // Version
    file.write_all(&total_length.to_le_bytes())?;  // Total length

    // Write JSON chunk length and data
    file.write_all(&json_chunk_length.to_le_bytes())?; // JSON chunk length
    file.write_all(b"JSON")?;                          // JSON chunk type
    file.write_all(json_data)?;                         // JSON data

    // Write binary chunk length and data
    file.write_all(&binary_chunk_length.to_le_bytes())?; // Binary chunk length
    file.write_all(b"BIN\x00")?;                         // Binary chunk type
    file.write_all(binary_data)?;                        // Binary data

    Ok(file)
}

fn tri_normal(v0: Vector3<f32>, v1: Vector3<f32>, v2: Vector3<f32>) -> Vector3<f32> {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let normal = edge1.cross(&edge2);
    normal.normalize()
}

// Select valid edges according to connectivity and distance
fn get_valid_edge(index_list:&Vec<u32>, position_list:&HashMap<u32,Vector3<f32>>, 
                vertex_list:&HashMap<u32, Vertex>) -> HashMap<(u32, u32), f32>{
    let mut valid_edge:HashMap<(u32, u32), f32> = HashMap::new();
    // Check by distance
    let index_num = position_list.len();
    for i in 0..index_num {
        for j in 0..index_num {
            if j > i {
                let p1 = position_list.get(&(i as u32)).unwrap();
                let p2 = position_list.get(&(j as u32)).unwrap();
                if (p1-p2).norm() < 0.001 && (p1-p2).norm() > 0.0 {
                    let new_pos = Vector3::new(p1[0]+p2[0]/2.0, p1[1]+p2[1]/2.0, p1[2]+p2[2]/2.0);
                    let q1 = &vertex_list.get(&(i as u32)).unwrap().q_matrix;
                    let q2 = &vertex_list.get(&(j as u32)).unwrap().q_matrix;
                    valid_edge.insert((i as u32, j as u32), update_cost(q1, q2, &new_pos));
                }
            }
        }
    }
    // Check by connection
    for (k,v) in vertex_list {
        for x in &v.edge_set {
            if x[1] > x[0] {
                if !valid_edge.contains_key(&(x[0], x[1])) {
                    let p1 = position_list.get(&x[0]).unwrap();
                    let p2 = position_list.get(&x[1]).unwrap();
                    let new_pos = Vector3::new(p1[0]+p2[0]/2.0, p1[1]+p2[1]/2.0, p1[2]+p2[2]/2.0);
                    let q1 = &vertex_list.get(&x[0]).unwrap().q_matrix;
                    let q2 = &vertex_list.get(&x[1]).unwrap().q_matrix;
                    valid_edge.insert((x[0], x[1]), update_cost(q1, q2, &new_pos));

                }
            } else {
                if !valid_edge.contains_key(&(x[1], x[0])) {
                    let p1 = position_list.get(&x[1]).unwrap();
                    let p2 = position_list.get(&x[0]).unwrap();
                    let new_pos = Vector3::new(p1[0]+p2[0]/2.0, p1[1]+p2[1]/2.0, p1[2]+p2[2]/2.0);
                    let q1 = &vertex_list.get(&x[1]).unwrap().q_matrix;
                    let q2 = &vertex_list.get(&x[0]).unwrap().q_matrix;
                    valid_edge.insert((x[1], x[0]), update_cost(q1, q2, &new_pos));
                }
            }
        }
    }
    valid_edge
}

fn update_cost(q1:&Vec<f32>, q2:&Vec<f32>, new_pos:&Vector3<f32>) -> f32{
    // Cost: q11x^2 + 2q12xy + 2q13xz + 2q14x + q22y^2
    //       + 2q23yz + 2q24y + q33z^2 + 2q34z + q44
    // Q: 0 1 2 3
    //    1 4 5 6
    //    2 5 7 8
    //    3 6 8 9
    let mut q = vec![0.0; 10];
    for i in 0..10 {
        q[i] = q1[i] + q2[i];
    }
    let cost = q[0]*new_pos[0]*new_pos[0] +
                2.0*q[1]*new_pos[0]*new_pos[1] + 
                2.0*q[2]*new_pos[0]*new_pos[2] + 
                2.0*q[3]*new_pos[0] + 
                q[4]*new_pos[1]*new_pos[1] + 
                2.0*q[5]*new_pos[1]*new_pos[2] + 
                2.0*q[6]*new_pos[1] + 
                q[7]*new_pos[2]*new_pos[2] + 
                2.0*q[8]*new_pos[2] + q[9];
    cost
}

fn initialize(index_list:&Vec<u32>,  normal_list:&HashMap<u32,Vector3<f32>>,
             position_list:&HashMap<u32,Vector3<f32>>, 
             tangent_list:&HashMap<u32,Vector4<f32>>, 
             texcoord_0_list:&HashMap<u32,Vector2<f32>>) -> (HashMap<u32, Vertex>, u32) {

    let mut vertex_list: HashMap<u32, Vertex> = HashMap::new();
    // Calculate k_matrix for each triangle faces
    let tri_num = index_list.len() / 3;
    let mut k_list:Vec<Vec<f32>> = Vec::with_capacity(tri_num);
    for i in 0..tri_num {
        let v1 = position_list.get(&index_list[i]).unwrap();
        let v2 = position_list.get(&index_list[i+1]).unwrap();
        let v3 = position_list.get(&index_list[i+2]).unwrap();
        k_list.push(get_k_matrix(v1, v2, v3));
        println!("k_matrix: {:?}", k_list[i])
    }
    println!("k_matrix number: {}", k_list.len());

    // Initialize neighbor edges and faces for each vertex
    // Calculate the q_matrix of the vertex according to face_set
    let v_num = position_list.len();
    for i in 0..v_num {
        let mut edge_set: Vec<Vector2<u32>> = Vec::new();
        let mut face_set: Vec<Vector3<u32>> = Vec::new();
        let mut q_matrix:Vec<f32> = vec![0.0; 10];
        for j in 0..tri_num {
            if index_list[j*3] == i as u32 {
                edge_set.push(Vector2::new(index_list[j*3], index_list[j*3+1]));
                edge_set.push(Vector2::new(index_list[j*3+2], index_list[j*3]));
                face_set.push(Vector3::new(index_list[j*3],index_list[j*3+1],index_list[j*3+2]));
                for x in 0..10 {
                    q_matrix[x] = q_matrix[x] + k_list[j][x];
                }
            } else if index_list[j*3+1] == i as u32 {
                edge_set.push(Vector2::new(index_list[j*3], index_list[j*3+1]));
                edge_set.push(Vector2::new(index_list[j*3+1], index_list[j*2]));
                face_set.push(Vector3::new(index_list[j*3],index_list[j*3+1],index_list[j*3+2]));
                for x in 0..10 {
                    q_matrix[x] = q_matrix[x] + k_list[j][x];
                }
            } else if index_list[j*3+2] == i as u32 {
                edge_set.push(Vector2::new(index_list[j*3+1], index_list[j*3+2]));
                edge_set.push(Vector2::new(index_list[j*3+2], index_list[j*3]));
                face_set.push(Vector3::new(index_list[j*3],index_list[j*3+1],index_list[j*3+2]));
                for x in 0..10 {
                    q_matrix[x] = q_matrix[x] + k_list[j][x];
                }
            }
        }
        println!("q_matrix: {:?}", q_matrix);
        vertex_list.insert(i as u32, Vertex::new(edge_set, face_set, q_matrix));
    }
    (vertex_list, tri_num as u32)
}

// Implement trait to form a k matrix(Fundamental Error Quadric)
fn get_k_matrix(v1:&Vector3<f32>, v2:&Vector3<f32>, v3:&Vector3<f32>) -> Vec<f32> {
    // Calculate 2 vectors on the plane
    let v1v2 = Vector3::new(v2[0] - v1[0], v2[1] - v1[1], v2[2] - v1[2]);
    let v1v3 = Vector3::new(v3[0] - v1[0], v3[1] - v1[1], v3[2] - v1[2]);
    // Calculate the normal vector to the plane
    let normal = v1v2.cross(&v1v3);
    // Normalize the normal vector
    //let normal = Unit::new_normalize(normal);
    // Calculate the distance from the origin to the plane
    let d = -normal.dot(v1);
    // Extract components of the normalized normal vector
    let (a, b, c) = (normal[0], normal[1], normal[2]);
    let mut k = Vec::with_capacity(10);
    //     a^2   ab    ac    ad
    //     ab    b^2   bc    bd
    //     ac    bc    c^2   cd
    //     ad    bd    cd    d^2
    println!("abcd: {}, {}, {}, {}", a, b, c, d);
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
    k
}

fn unpack_gltf(path:&Path) -> (Vec<u32>, HashMap<u32,Vector3<f32>>, HashMap<u32,Vector3<f32>>, 
                                HashMap<u32,Vector4<f32>>, HashMap<u32,Vector2<f32>>, 
                                Value, HashMap<String,Prim>) {
    let mut file = File::open(path.to_str().unwrap()).expect("Failed to open file");
    let mut buffer = Vec::new();

    // Read the file contents into a buffer
    file.read_to_end(&mut buffer).expect("Failed to read file");
    // Extract JSON chunk length (the length of the JSON chunk is stored as a little-endian u32 at byte offset 12)
    let json_length = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]) as usize;
    // Extract the JSON chunk and binary chunk from the buffer
    let json_chunk = &buffer[20..20 + json_length];
    let binary_chunk = &buffer[20 + json_length + 8..];
    // Parse the json_chunk as JSON
    let mut json:Value = serde_json::from_slice(json_chunk).expect("Failed to parse JSON");

    // Processing json information into HashMaps of struct
    let mut views:HashMap<i64,View> = HashMap::new();
    let mut primitives:HashMap<String,Prim> = HashMap::new();
    println!("{}", serde_json::to_string_pretty(&json).unwrap());

    // Write buffer view information
    if let Some(buffer_views) = json["bufferViews"].as_array() {
        let mut i = 0;
        for buffer_view in buffer_views {
            let buffer = buffer_view["buffer"].as_u64().unwrap_or(0) as u32;
            let byte_length = buffer_view["byteLength"].as_u64().unwrap_or(0) as u32;
            let byte_offset = buffer_view["byteOffset"].as_u64().unwrap_or(0) as u32;
            views.insert(i, View::new(
                buffer,
                byte_length,
                byte_offset,
            ));
            i += 1;
        }
    }

    let indices = if let Some(mut indices) = json["meshes"][0]["primitives"][0]["indices"].as_i64(){
        println!("Indices at : {}", indices);
        indices
    } else {
        println!("No Indices");
        -1
    };
    let normal = if let Some(normal) = json["meshes"][0]["primitives"][0]["attributes"]["NORMAL"].as_i64(){
        println!("NORMAL at : {}", normal);
        normal
    } else {
        println!("No NORMAL");
        -1
    };
    let position = if let Some(position) = json["meshes"][0]["primitives"][0]["attributes"]["POSITION"].as_i64(){
        println!("POSITION at : {}", position);
        position
    } else {
        println!("No POSITION");
        -1
    };
    let tangent = if let Some(tangent) = json["meshes"][0]["primitives"][0]["attributes"]["TANGENT"].as_i64(){
        println!("TANGENT at : {}", tangent);
        tangent
    } else {
        println!("No TANGENT");
        -1
    };
    let texcoord_0 = if let Some(texcoord_0) = json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_0"].as_i64(){
        println!("TEXCOORD_0 at : {}", texcoord_0);
        texcoord_0
    } else {
        println!("No TEXCOORD_0");
        -1
    };

    // Write primitives' information
    if let Some(accessors) = json["accessors"].as_array() {
        let mut i:i64 = 0;
        for accessor in accessors {
            let bufferView = accessor["bufferView"].as_u64().unwrap_or(0) as u32;
            let byteOffset = accessor["byteOffset"].as_u64().unwrap_or(0) as u32;
            let componentType = accessor["componentType"].as_u64().unwrap() as u32;
            let normalized= accessor["normalized"].as_bool().unwrap_or(false);
            let count = accessor["count"].as_u64().unwrap() as u32;
            let prim_type = String::from(accessor["type"].as_str().unwrap());
            let mut key = String::from("ERROR");
            if i == indices{
                key = String::from("indices");
            } else if i == normal{
                key = String::from("NORMAL");
            } else if i == position{
                key = String::from("POSITION");
            } else if i == texcoord_0{
                key = String::from("TEXCOORD_0");
            }
            println!("inserting key: {} at {}", key, i);
            primitives.insert(key, Prim::new(bufferView, byteOffset, componentType, normalized, count, prim_type));
            i = i+1;
        }
    }

    let mut index_list:Vec<u32> = Vec::new();
    // Can use struct later
    //let mut info_list: HashMap<u32, Point> = HashMap::new();
    println!{"Views: {:?}", views}

    // Write indices information
    if let Some(view) = views.get(&indices) {
        let off = view.byteOffset;
        let mut t = primitives.get("indices").unwrap().componentType;
        let mut num = primitives.get("indices").unwrap().count;
        match t {
            5120 => {
                for i in 0..num {
                    index_list.push(byte_i8(binary_chunk, (off + i) as usize));
                }
            }
            5121 => {
                for i in 0..num {
                    index_list.push(byte_u8(binary_chunk, (off + i) as usize));
                }
            }
            5122 => {
                for i in 0..num {
                    index_list.push(byte_i16(binary_chunk, (off + i*2) as usize));
                }
            }
            5123 => {
                for i in 0..num {
                    index_list.push(byte_u16(binary_chunk, (off + i*2) as usize));
                }
            }
            5125 => {
                for i in 0..num {
                    index_list.push(byte_u32(binary_chunk, (off + i*4) as usize));
                }
            }
            _ => todo!(),
        };
        println!("indices elements numnber is: {}. \n{:?}", index_list.len(), index_list);
    } else {
        println!("No view found for indices: {}", indices);
    }

    let mut normal_list: HashMap<u32,Vector3<f32>> = HashMap::new();
    let mut position_list: HashMap<u32,Vector3<f32>> = HashMap::new();
    let mut tangent_list: HashMap<u32,Vector4<f32>> = HashMap::new();
    let mut texcoord_0_list: HashMap<u32,Vector2<f32>> = HashMap::new();

    // Write normal information
    if let Some(view) = views.get(&normal) {
        let off = view.byteOffset;
        if primitives.get("NORMAL").unwrap().componentType != 5126 {
            println!("Incorrect type for normal: should be f32");
        } else {
            for i in &index_list {
                normal_list.insert(*i,Vector3::new(byte_f32(binary_chunk, (off + i*12) as usize),
                                    byte_f32(binary_chunk, (off + i*12+4) as usize),
                                    byte_f32(binary_chunk, (off + i*12+8) as usize)));
            }
            println!("normal elements numnber is: {}. \n{:?}", normal_list.len(), normal_list);
        }
    }

    // Write position information
    if let Some(view) = views.get(&position) {
        let off = view.byteOffset;
        if primitives.get("POSITION").unwrap().componentType != 5126 {
            println!("Incorrect type for position: should be f32");
        } else {
            for i in &index_list {
                position_list.insert(*i,Vector3::new(byte_f32(binary_chunk, (off + i*12) as usize),
                                    byte_f32(binary_chunk, (off + i*12+4) as usize),
                                    byte_f32(binary_chunk, (off + i*12+8) as usize)));
            }
            println!("position elements numnber is: {}. \n{:?}", position_list.len(), position_list);
        }
    }

    // Write tangent information
    if let Some(view) = views.get(&tangent) {
        let off = view.byteOffset;
        if primitives.get("TANGENT").unwrap().componentType != 5126 {
            println!("Incorrect type for tangent: should be f32");
        } else {
            for i in &index_list {
                tangent_list.insert(*i,Vector4::new(byte_f32(binary_chunk, (off + i*16) as usize),
                                    byte_f32(binary_chunk, (off + i*16+4) as usize),
                                    byte_f32(binary_chunk, (off + i*16+8) as usize),
                                    byte_f32(binary_chunk, (off + i*16+12) as usize)));
            }
            println!("tangent elements numnber is: {}. \n{:?}", tangent_list.len(), tangent_list);
        }
    }

    // Write texcoord_0 information
    if let Some(view) = views.get(&texcoord_0) {
        let off = view.byteOffset;
        if primitives.get("TEXCOORD_0").unwrap().componentType != 5126 {
            println!("Incorrect type for texcoord_0: should be f32");
        } else {
            for i in &index_list {
                texcoord_0_list.insert(*i,Vector2::new(byte_f32(binary_chunk, (off + i*8) as usize),
                                    byte_f32(binary_chunk, (off + i*8+4) as usize)));
            }
            println!("texcoord_0 elements numnber is: {}. \n{:?}", texcoord_0_list.len(), texcoord_0_list);
        }
    }

    (index_list, normal_list, position_list, tangent_list, texcoord_0_list, json, primitives)
}

fn byte_i8(buff:&[u8], x:usize) -> u32{
    let byte = &buff[x..x+1];
    i8::from_le_bytes(byte.try_into().unwrap()) as u32
}
fn byte_u8(buff:&[u8], x:usize) -> u32{
    let byte = &buff[x..x+1];
    u8::from_le_bytes(byte.try_into().unwrap()) as u32
}
fn byte_i16(buff:&[u8], x:usize) -> u32{
    let byte = &buff[x..x+2];
    i16::from_le_bytes(byte.try_into().unwrap()) as u32
}
fn byte_u16(buff:&[u8], x:usize) -> u32{
    let byte = &buff[x..x+2];
    u16::from_le_bytes(byte.try_into().unwrap()) as u32
}
fn byte_u32(buff:&[u8], x:usize) -> u32{
    let byte = &buff[x..x+4];
    u32::from_le_bytes(byte.try_into().unwrap())
}
fn byte_f32(buff:&[u8], x:usize) -> f32{
    let byte = &buff[x..x+4];
    f32::from_le_bytes(byte.try_into().unwrap())
}