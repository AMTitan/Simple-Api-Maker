#[macro_use]
extern crate lazy_static;

use rustbreak::backend::FileBackend;
use rustbreak::deser::Ron;
use rustbreak::Database;
use rustbreak::FileDatabase;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

lazy_static! {
    static ref DB: Database<HashMap<String, String>, FileBackend, rustbreak::deser::Ron> =
        FileDatabase::<HashMap<String, String>, Ron>::load_from_path_or_default("main.ron")
            .unwrap();
}

fn main() {
    println!("This is a simple api maker, please go to http://127.0.0.1 to create an endpoint");
    let listener = TcpListener::bind("0.0.0.0:80").unwrap();
    for stream in listener.incoming() {
        thread::spawn(move || {
            let mut stream = stream.unwrap();
            let mut request: Vec<u8> = Vec::new();
            let mut buf = [0; 4096];
            let mut cont = true;
            while cont {
                let len = stream.read(&mut buf).unwrap();
                request.extend_from_slice(&buf[..len]);
                cont = len == 4096;
            }
            let input = String::from_utf8_lossy(&request).to_string();
            let mut wants = input.split(' ').nth(1).unwrap().to_string();
            if wants == "/" {
                wants = "/index.html".to_string();
            }
            wants = wants[1..].to_string();
            println!("{} made a request for {}", stream.local_addr().unwrap().to_string().split(':').nth(0).unwrap(), wants);
            if wants.starts_with("api/") {
                match get_item(wants[4..].to_string()) {
                    Ok(x) => {
                        let _ = DB.read(|db| {
                            stream
                                .write(&get_response(x.clone(), db.get(&x).unwrap().to_string()))
                                .unwrap();
                        });
                    }
                    Err(_) => {
                        stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
                    }
                }
            } else if wants.starts_with("admin/")
                && stream
                    .local_addr()
                    .unwrap()
                    .to_string()
                    .split(':')
                    .nth(0)
                    .unwrap()
                    == "127.0.0.1"
            {
                if wants[6..].starts_with("create/") {
                    if wants[13..].to_string().split(':').count() != 2 {
                        stream.write(b"HTTP/1.1 400 Bad Request\r\n\r\n").unwrap();
                    }
                    else {
                        match DB.write(|db| {
                            db.insert(
                                wants[13..].to_string().split(':').nth(0).unwrap().into(),
                                wants[13..].to_string().split(':').nth(1).unwrap().into(),
                            );
                        }) {
                            Ok(_) => stream.write(b"HTTP/1.1 200 Ok\r\n\r\n").unwrap(),
                            Err(x) => stream.write(format!("HTTP/1.1 500 Internal Server Error\r\nContent-length: {}\r\n\r\n{}", x.to_string().len(), x).as_bytes()).unwrap(),
                        };
                        DB.save();
                    }
                }
                else if wants[6..].starts_with("delete/") {
                    match DB.write(|db| {
                        db.remove(&wants[13..].to_string())
                    }) {
                        Ok(_) => stream.write(b"HTTP/1.1 200 Ok\r\n\r\n").unwrap(),
                        Err(x) => stream.write(format!("HTTP/1.1 500 Internal Server Error\r\nContent-length: {}\r\n\r\n{}", x.to_string().len(), x).as_bytes()).unwrap(),
                    };
                    DB.save();
                }
            } else if File::open(format!("./www/{}", wants)).is_ok() {
                let mut f = File::open(format!("./www/{}", wants)).unwrap();
                let mut buffer = Vec::new();
                for i in format!(
                    "HTTP/1.1 200 Ok\r\nContent-length: {}\r\n\r\n",
                    f.metadata().unwrap().len()
                )
                .as_bytes()
                {
                    buffer.push(*i);
                }
                f.read_to_end(&mut buffer).expect("buffer overflow");
            } else {
                stream.write(b"HTTP/1.1 400 Bad Request\r\n\r\n").unwrap();
            }
            stream.flush().unwrap();
        });
    }
}

fn get_item(wants: String) -> Result<String, String> {
    if DB.read(|db| {
        db.get(&wants).is_some()
    }).unwrap() {
        Ok(wants)
    }
    else {
        Err("Not implemented".to_string())
    }
}

fn get_response(endpoint: String, Returns: String) -> Vec<u8> {
    if !Returns.contains('$') && !endpoint.contains('$') {
        format!("HTTP/1.1 200 Ok\r\nConent-length: {}\r\n\r\n{}", Returns.len(), Returns).as_bytes().to_vec()
    }
    else {
        Vec::new()
    }
}
