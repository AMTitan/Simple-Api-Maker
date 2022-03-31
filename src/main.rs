#[macro_use]
extern crate lazy_static;

use fancy_regex::Regex;
use rand::prelude::IteratorRandom;
use rand::prelude::SliceRandom;
use rustbreak::backend::FileBackend;
use rustbreak::deser::Ron;
use rustbreak::Database;
use rustbreak::FileDatabase;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;
use toml::Value;

lazy_static! {
    static ref DB: Database<HashMap<String, String>, FileBackend, rustbreak::deser::Ron> =
        FileDatabase::<HashMap<String, String>, Ron>::load_from_path_or_default("main.ron")
            .unwrap();
    static ref CONFIG: Value = fs::read_to_string("Config.toml")
        .unwrap_or("".to_string())
        .parse::<Value>()
        .unwrap();
}

fn main() {
    if !Path::new("Config.toml").exists() {
        println!("\x1b[31mERROR: you need a Config.toml file and you don't have one, I made one for you so can you please edit it and run this again.\x1b[0m");
        let mut file = File::create("Config.toml").unwrap();
        file.write_all(b"domain = \'http://example.com\' # this can be an ip or a domain")
            .unwrap();
        std::process::exit(1);
    }
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
            println!(
                "{} made a request for {}",
                stream
                    .local_addr()
                    .unwrap()
                    .to_string()
                    .split(':')
                    .nth(0)
                    .unwrap(),
                wants
            );
            if wants.starts_with("api/") {
                match get_item(wants[4..].to_string()) {
                    Ok(x) => {
                        let _ = DB.read(|db| {
                            stream
                                .write(&get_response(
                                    x.clone(),
                                    wants[4..].to_string(),
                                    db.get(&x).unwrap().to_string(),
                                ))
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
                if wants[6..].starts_with("create/") || wants[6..].starts_with("edit/") {
                    if input.split("\r\n\r\n").nth(1).unwrap().len() == 0 {
                        stream.write(b"HTTP/1.1 400 Bad Request\r\n\r\n").unwrap();
                    } else {
                        match DB.write(|db| {
                            db.insert(
                                wants[13..].to_string(),
                                {
                                    input.split("\r\n\r\n").nth(1).unwrap().to_string()
                                },
                            );
                        }) {
                            Ok(_) => stream.write(b"HTTP/1.1 200 Ok\r\n\r\n").unwrap(),
                            Err(x) => stream.write(format!("HTTP/1.1 500 Internal Server Error\r\nContent-length: {}\r\n\r\n{}", x.to_string().len(), x).as_bytes()).unwrap(),
                        };
                        DB.save();
                    }
                } else if wants[6..].starts_with("delete/") {
                    match DB.write(|db| {
                        db.remove(&wants[13..].to_string())
                    }) {
                        Ok(_) => stream.write(b"HTTP/1.1 200 Ok\r\n\r\n").unwrap(),
                        Err(x) => stream.write(format!("HTTP/1.1 500 Internal Server Error\r\nContent-length: {}\r\n\r\n{}", x.to_string().len(), x).as_bytes()).unwrap(),
                    };
                    DB.save();
                }
            } else if wants.starts_with("backend/") {
                if wants[8..] == *"list" {
                    let _ = DB.read(|db| {
                        stream
                            .write(
                                format!(
                                    "HTTP/1.1 200 Ok\r\nContent-length: {}\r\n\r\n{}",
                                    format!("{:?}", db).len(),
                                    format!("{:?}", db)
                                )
                                .as_bytes(),
                            )
                            .unwrap();
                    });
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
                stream.write(&buffer).unwrap();
            } else if File::open(format!("./assets/{}", wants[7..].to_string())).is_ok() {
                let mut f = File::open(format!("./assets/{}", wants[7..].to_string())).unwrap();
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
                stream.write(&buffer).unwrap();
            } else {
                stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
            }
            stream.flush().unwrap();
        });
    }
}

fn get_item(wants: String) -> Result<String, String> {
    if DB.read(|db| db.get(&wants).is_some()).unwrap() {
        Ok(wants)
    } else {
        DB.read(|db| {
            let mut could_be = Vec::new();
            for (x, y) in db {
                could_be.push(x);
            }
            for i in 0..wants.split("/").count() {
                let mut offset = 0;
                for x in 0..could_be.len() {
                    if !(could_be[x - offset].split("/").nth(i).is_some()
                        && could_be[x - offset].split("/").nth(i).unwrap()
                            == wants.split("/").nth(i).unwrap()
                        || (could_be[x - offset].split("/").nth(i).is_some()
                            && could_be[x - offset]
                                .split("/")
                                .nth(i)
                                .unwrap()
                                .contains("$")
                            && wants.split("/").nth(i).unwrap().starts_with(
                                could_be[x - offset]
                                    .split("/")
                                    .nth(i)
                                    .unwrap()
                                    .split("$")
                                    .nth(0)
                                    .unwrap(),
                            )))
                    {
                        could_be.remove(x - offset);
                        offset += 1;
                    }
                }
            }
            if could_be.len() != 0 {
                Ok(could_be[0].to_string())
            } else {
                Err("Not Found".to_string())
            }
        })
        .unwrap()
    }
}

fn get_response(endpoint: String, send_endpoint: String, mut returns: String) -> Vec<u8> {
    if !returns.contains('$') && !endpoint.contains('$') {
        format!(
            "HTTP/1.1 200 Ok\r\nConent-length: {}\r\n\r\n{}",
            returns.len(),
            returns
        )
        .as_bytes()
        .to_vec()
    } else {
        let mut variables: HashMap<String, String> = HashMap::new();
        if endpoint.contains("$") {
            let re = Regex::new(r"\$[^\/]+").unwrap();
            let mut offset = 0;
            for i in re.captures_iter(&endpoint) {
                let name = i.as_ref().unwrap().get(0).unwrap().as_str();
                let equals = send_endpoint[i.as_ref().unwrap().get(0).unwrap().start() + offset..]
                    .split("/")
                    .nth(0)
                    .unwrap();
                offset += equals.len() - name.len();
                variables.insert(name.to_string(), equals.to_string());
            }
        }
        if returns.contains("$") {
            for (x, y) in variables {
                returns = returns.replace(&x, &y);
            }
            while returns.contains("$") {
                let re = Regex::new(r"\$[^\/()]+\([^\/()]*\)").unwrap();
                let mut last_end = 0;
                let mut new_returns = "".to_string();
                for i in re.captures_iter(&returns.clone()) {
                    let cont = i.as_ref().unwrap().get(0).unwrap().as_str();
                    let replace_with = match &cont.split("(").nth(0).unwrap()[1..] {
                        "RFolder" => {
                            let paths = fs::read_dir(format!(
                                "./assets/{}",
                                cont.split("(").nth(1).unwrap()
                                    [..cont.split("(").nth(1).unwrap().len() - 1]
                                    .to_string()
                            ))
                            .unwrap();
                            let mut file = paths
                                .choose(&mut rand::thread_rng())
                                .unwrap()
                                .unwrap()
                                .path()
                                .display()
                                .to_string();
                            file = file
                                .split(&cont.split("(").nth(1).unwrap()[..1])
                                .last()
                                .unwrap()
                                .to_string();
                            format!(
                                "{}/assets/{}{}",
                                CONFIG["domain"].as_str().unwrap(),
                                cont.split("(").nth(1).unwrap()
                                    [..cont.split("(").nth(1).unwrap().len() - 1]
                                    .to_string(),
                                file[9..].to_string()
                            )
                        }
                        "RLine" => {
                            let cont = fs::read_to_string(format!(
                                "./assets/{}",
                                cont.split("(").nth(1).unwrap()
                                    [..cont.split("(").nth(1).unwrap().len() - 1]
                                    .to_string()
                            ))
                            .unwrap();
                            let lines: Vec<&str> = cont.lines().collect();
                            lines.choose(&mut rand::thread_rng()).unwrap().to_string()
                        }
                        _ => "".to_string(),
                    };
                    new_returns.push_str(
                        &returns[last_end..i.as_ref().unwrap().get(0).unwrap().start()],
                    );
                    new_returns.push_str(&replace_with);
                    last_end = i.as_ref().unwrap().get(0).unwrap().end();
                }
                new_returns.push_str(&returns[last_end..returns.len()]);
                returns = new_returns.to_string();
            }
        }
        format!(
            "HTTP/1.1 200 Ok\r\nConent-length: {}\r\n\r\n{}",
            returns.len(),
            returns
        )
        .as_bytes()
        .to_vec()
    }
}

fn get_cont(conts: String) {}
