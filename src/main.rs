extern crate libc;
extern crate elf;
extern crate flate2;
extern crate walkdir;

use std::io::{BufReader,BufRead};
use std::fs::File;
use std::io::Read;
use std::io::Cursor;

const F_NAME : usize = 0;
const F_SIZE : usize = 1;
const F_USECOUNT : usize = 2;
const F_DEPENDENCIES : usize = 3;

#[allow(dead_code)]
struct Module {
    name: String,
    size: usize,
    ref_count: usize,
    dependencies: Vec<String>,
}

#[derive(Debug, Default)]
struct ModuleParameter {
    name: String,
    description: String,
    kind: String,
}

#[derive(Debug, Default)]
struct ModInfo {
    license: String,
    parameters: std::collections::HashMap<String, ModuleParameter>,
    alias: Vec<String>,
    dependencies: Vec<String>,
    description: String,
    authors: Vec<String>,
    vermagic: String,
    intree: bool,
    firewares: Vec<String>,
}

fn get_kernel_release() -> String {
    unsafe {
        let mut result : libc::utsname = std::mem::uninitialized();
        libc::uname(&mut result);
        let release : Vec<u8> = std::mem::transmute::<[i8; 65], [u8; 65]>(result.release)
            .iter()
            .filter(|&&chr| { chr != 0u8 })
            .map(|&refbox| { refbox.to_owned() })
            .collect();

        String::from_utf8(release).unwrap()
    }
}

fn get_module_paths() -> Vec<String> {
    let release = get_kernel_release();
    let rootdir = format!("/lib/modules/{}", release);

    let mut result = vec![];
    for entry in walkdir::WalkDir::new(rootdir) {
        let entry = entry.unwrap();
        let filename = entry.path().to_str().unwrap();
        if filename.ends_with(".ko.gz") {
            result.push(filename.to_owned());
        }
    }
    result
}

fn get_modinfo_from_file(path: String) -> Result<ModInfo, &'static str> {
    let mut gzipped = File::open(&path).unwrap();
    let mut compressed = Vec::new();
    gzipped.read_to_end(&mut compressed).unwrap();
    let mut buffer = Cursor::new(&compressed);

    let mut decoder = flate2::read::GzDecoder::new(&mut buffer).unwrap();
    let mut buf = Cursor::new(Vec::new());
    decoder.read_to_end(buf.get_mut()).unwrap();

    let file = match elf::File::open_stream(&mut buf) {
        Ok(f) => f,
        Err(e) => panic!("Error: {:?}", e),
    };

    match file.get_section(".modinfo") {
        Some(s) => {
            let mut result : ModInfo = ModInfo::default();
            for one in s.data.split(|chr| *chr == 0) {
                let entry = String::from_utf8(one.to_vec()).unwrap();
                if entry.starts_with("parm=") {
                    let tmp = &entry["parm=".len()..];
                    let (name, description) = tmp.split_at(tmp.find(":").unwrap());
                    result.parameters.insert(name.to_owned(), ModuleParameter {
                        name: name.to_owned(),
                        description: description[1..].to_owned(),
                        kind: "".to_owned(),
                    });
                } else if entry.starts_with("parmtype=") {
                    let tmp = &entry["parmtype=".len()..];
                    let mut split = tmp.split(":");
                    if let Some(x) = result.parameters.get_mut(&split.next().unwrap().to_owned()) {
                        (*x).kind = split.next().unwrap().to_owned();
                    }
                } else if entry.starts_with("license=") {
                    result.license = (&entry["license=".len()..]).to_owned();
                } else if entry.starts_with("alias=") {
                    result.alias.push((&entry["alias=".len()..]).to_owned());
                } else if entry.starts_with("depends=") {
                    for dependency in (&entry["depends=".len()..]).split(",") {
                        result.dependencies.push(dependency.to_owned());
                    }
                } else if entry.starts_with("description=") {
                    result.description = (&entry["description=".len()..]).to_owned();
                } else if entry.starts_with("author=") {
                    result.authors.push((&entry["author=".len()..]).to_owned());
                } else if entry.starts_with("vermagic=") {
                    result.vermagic = (&entry["vermagic=".len()..]).to_owned();
                } else if entry.starts_with("intree=") {
                    result.intree = (&entry["intree=".len()..]).to_owned() == "Y";
                } else if entry.starts_with("firmware=") {
                    result.firewares.push((&entry["fireware=".len()..]).to_owned());
                } else if entry.starts_with("version=") {
                    // TODO
                } else if entry.starts_with("srcversion=") {
                    // TODO
                } else if entry.starts_with("staging=") {
                    // TODO
                } else if entry.starts_with("release_date=") {
                    // TODO
                } else if entry.starts_with("softdep=") {
                    // TODO
                } else if entry == "" {
                } else {
                    println!("Unmatched {}, {}", &path, &entry);
                }
            };
            Ok(result)
        },
        None => Err("Cannot find .modinfo section"),
    }
}

fn get_loaded_modules() -> Vec<Module> {
    match File::open("/proc/modules") {
        Ok(file) => {
            let mut result = vec![];
            for line in BufReader::new(file).lines() {
                let text = line.unwrap();
                let parts : Vec<&str> = text.split(" ").collect();
                let module = Module {
                    name: parts[F_NAME].to_string(),
                    size: parts[F_SIZE].parse::<usize>().unwrap(),
                    ref_count: parts[F_USECOUNT].parse::<usize>().unwrap(),
                    dependencies: {
                        match parts[F_DEPENDENCIES] {
                            "-" => vec![],
                            _ => parts[F_DEPENDENCIES].split(",")
                            .filter(|&one| { one != "" })
                            .map(|one| { one.to_string() })
                            .collect(),
                        }
                    },
                };
                result.push(module);
            }
            result
        }
        Err(e) => {
            // fallback in case of failure.
            // you could log the error, panic, or do anything else.
            panic!("{}", e);
        }
    }
}

fn main() {
    println!("Module {:?} are loaded", get_loaded_modules().into_iter().map(|module| module.name).collect::<Vec<String>>());
    for module_path in get_module_paths() {
        get_modinfo_from_file(module_path).unwrap();
    };
}
