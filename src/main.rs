use clap::{clap_app, crate_authors, crate_description, crate_name, crate_version};
use dirs;
use raw_sync::locks::*;
use shared_memory::*;
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::process::Command;

fn is_number_string(s: String) -> Result<(), String> {
    if s.bytes().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(String::from(
            "Please specify number by ascii digit characters",
        ))
    }
}

fn make_cmd_db(file_name: &str) -> io::Result<HashMap<(u32, u32), String>> {
    let mut db = HashMap::new();

    let f = std::fs::File::open(file_name)?;
    let br = io::BufReader::new(f);
    for line in br.lines() {
        let line = line.unwrap();
        if !line.starts_with('#') {
            let cols: Vec<_> = line.splitn(3, '\t').collect();

            if cols.len() == 3 {
                let id = cols[0].parse::<u32>().unwrap();
                let count = cols[1].parse::<u32>().unwrap();
                let cmdstr = cols[2].to_owned();

                db.insert((id, count), cmdstr);
            }
        }
    }
    Ok(db)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap_app!((crate_name!()) =>
          (version: crate_version!())
          (author: crate_authors!())
          (about: crate_description!())
          (@arg CONFIG: -c --config [FILE] "Sets a custom config file")
          (@arg MSECS: -s --msecs +takes_value {is_number_string} "click separation time by milli seconds [default: 250ms]")
          (@arg ID: <ID> {is_number_string} "click id number")
    )
    .get_matches();

    let user_name = std::env::var("USER").expect("Please check $USER value");

    let map_file_name = format!("{}_{}", crate_name!(), user_name);

    let msecs = matches
        .value_of("MSECS")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(250);

    let dur = std::time::Duration::from_millis(msecs);

    let default_config_location = format!(
        "{}/clicklauncher/cmdtable.tsv",
        dirs::config_dir().unwrap().to_str().unwrap()
    );

    let config_file = matches
        .value_of("CONFIG")
        .and_then(|s| Some(s))
        .unwrap_or(&default_config_location);

    let cdb = match make_cmd_db(&config_file) {
        Ok(db) => db,
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                eprintln!("Please check config file: `{}'", config_file);
                std::process::exit(2)
            } else {
                return Err(Box::new(e));
            }
        }
    };

    let mut shmem = match ShmemConf::new().size(4096).flink(&map_file_name).create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink(&map_file_name).open()?,
        Err(e) => return Err(Box::new(e)),
    };

    let cid = matches.value_of("ID").unwrap().parse().unwrap();

    let count;

    let base_ptr = shmem.as_ptr();

    let (mutex, _) =
        unsafe { Mutex::from_existing(base_ptr, base_ptr.add(Mutex::size_of(Some(base_ptr))))? };
    {
        let guard = mutex.lock()?;
        let val: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(*guard as *mut u32, 2) };
        if shmem.is_owner() || val[0] != cid {
            val[0] = cid;
            val[1] = 1;
            count = 1;
        } else {
            val[1] += 1;
            count = val[1];
        }
    }

    std::thread::sleep(dur);

    {
        let guard = mutex.lock()?;
        let val: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(*guard as *mut u32, 2) };
        if cid == val[0] && count == val[1] {
            shmem.set_owner(true);
            if let Some(value) = cdb.get(&(cid, count)) {
                println!("run command: {}", value);

                if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .arg("/C")
                        .arg(value)
                        .spawn()
                        .expect("failed to execute process");
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(value)
                        .spawn()
                        .expect("failed to execute process");
                }
            }
        } else {
            shmem.set_owner(false);
        }
    }

    Ok(())
}
