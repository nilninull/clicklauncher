use anyhow::{self, Context};
use clap::{clap_app, crate_authors, crate_description, crate_name, crate_version};
use dirs;
use raw_sync::locks::{LockInit, Mutex};
use shared_memory::{ShmemConf, ShmemError};
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

fn make_cmd_db(file_name: &str) -> anyhow::Result<HashMap<Vec<u32>, String>> {
    let mut db = HashMap::new();

    let f = std::fs::File::open(file_name)?;
    let br = io::BufReader::new(f);
    for line in br.lines() {
        let line = line?;
        if !line.starts_with('#') {
            if let Some((idstr, cmdstr)) = line.split_once('\t') {
                let mut idvec = vec![];
                idstr
                    .split(' ')
                    .filter(|s| !s.is_empty())
                    .try_for_each(|s| {
                        s.parse()
                            .map(|u| idvec.push(u))
                            .with_context(|| format!("`{}' is not suitable for id number", s))
                    })?;

                db.insert(idvec, cmdstr.to_owned());
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

    let user_name = std::env::var("USER").context("Please check $USER value")?;

    let map_file_name = format!("{}_{}", crate_name!(), user_name);

    let msecs = matches
        .value_of("MSECS")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(250);

    let dur = std::time::Duration::from_millis(msecs);

    let config_dir_path = dirs::config_dir().context("Please check your config directory")?;
    let default_config_location =
        format!("{}/clicklauncher/cmdtable.tsv", config_dir_path.display());

    let config_file = matches
        .value_of("CONFIG")
        .and_then(|s| Some(s))
        .unwrap_or(&default_config_location);

    let cdb = make_cmd_db(&config_file)
        .with_context(|| format!("Please check the config file `{}'", config_file))?;

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
        let val: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(*guard as *mut u32, 1013) };
        val[0] += 1;
        count = val[0];
        val[count as usize] = cid;
    }

    std::thread::sleep(dur);

    {
        let guard = mutex.lock()?;
        let val: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(*guard as *mut u32, 1013) };
        if count == val[0] {
            shmem.set_owner(true);
            let mut cid_seq = vec![];
            for i in 1..=count {
                cid_seq.push(val[i as usize])
            }
            if let Some(value) = cdb.get(&cid_seq) {
                println!("run command: {}", value);

                if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .arg("/C")
                        .arg(value)
                        .spawn()
                        .context("failed to execute process")?;
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(value)
                        .spawn()
                        .context("failed to execute process")?;
                }
            }
        } else {
            shmem.set_owner(false);
        }
    }

    Ok(())
}
