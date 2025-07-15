use brdb::{Brdb, Guid, IntoReader, OwnerTableSoA, pending::BrPendingFs, schemas::OWNER_TABLE_SOA};
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf, process};
use uuid::Uuid;

/// Opens a world and replaces its owners with PUBLIC
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().into_iter().peekable();
    let cmd = args.next().unwrap();
    if !args.peek().is_some() {
        println!("usage: {cmd} show world.brdb");
        println!("usage: {cmd} apply world.brdb owners.csv");
        println!(
            "owners.csv must be `display_name,user_name,user_id,old_user_id` where old_user_id is the one to replace."
        );
        process::exit(0);
    }

    let command = args.next().unwrap();
    if command != "show" && command != "apply" {
        eprintln!("unknown command. expected `show` or `apply`");
        process::exit(1);
    }

    let Some(file) = args.next() else {
        eprintln!("missing world file arg");
        process::exit(1);
    };

    let dst = PathBuf::from(&file);
    if !dst.exists() {
        eprintln!("file {file} does not exist");
        process::exit(1);
    }

    let db = Brdb::open(dst)?.into_reader();

    let owners = db.owners_soa()?;

    if command == "show" {
        if args.peek().is_some() {
            eprintln!("too many arguments!");
            process::exit(1);
        }

        let owners_csv = owners
            .prop("DisplayNames")?
            .as_array()?
            .iter()
            .zip(owners.prop("UserNames")?.as_array()?.iter())
            .zip(owners.prop("UserIds")?.as_array()?.iter())
            .map(|((display_name, user_name), user_id)| {
                format!(
                    "{},{},{}",
                    display_name.as_str().unwrap(),
                    user_name.as_str().unwrap(),
                    Guid::try_from(user_id).unwrap().uuid(),
                )
            })
            .collect::<Vec<_>>();
        println!("display_name,user_name,user_id\n{}", owners_csv.join("\n"));
    } else if command == "apply" {
        let Some(apply_file) = args.next() else {
            eprintln!("missing owners csv file arg");
            process::exit(1);
        };
        if args.peek().is_some() {
            eprintln!("too many arguments!");
            process::exit(1);
        }

        let apply_path = PathBuf::from(&apply_file);
        if !apply_path.exists() {
            eprintln!("file {apply_file} does not exist");
            process::exit(1);
        }

        let mut display_name_index = None;
        let mut user_name_index = None;
        let mut user_id_index = None;
        let mut old_user_id_index = None;
        let mut apply_data = String::new();
        File::open(apply_path)?.read_to_string(&mut apply_data)?;
        let Some((header, rows)) = apply_data.split_once("\n") else {
            eprintln!("file {apply_file} does not have any rows");
            process::exit(1);
        };
        for (i, key) in header.split(",").enumerate() {
            match key.trim().to_ascii_lowercase().as_ref() {
                "display_name" => {
                    display_name_index = Some(i);
                }
                "user_name" => {
                    user_name_index = Some(i);
                }
                "user_id" => {
                    user_id_index = Some(i);
                }
                "old_user_id" => old_user_id_index = Some(i),
                other => {
                    eprintln!("unknown column {other} in {apply_file}");
                    process::exit(1);
                }
            }
        }

        let missing = [
            ("display_name", display_name_index.is_none()),
            ("user_name", user_name_index.is_none()),
            ("user_id", user_id_index.is_none()),
            ("old_user_id", old_user_id_index.is_none()),
        ]
        .into_iter()
        .filter_map(|(k, cond)| cond.then_some(k.to_owned()))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            eprintln!("missing columns: {}", missing.join(","));
            process::exit(1);
        }

        let display_name_index = display_name_index.unwrap();
        let user_name_index = user_name_index.unwrap();
        let user_id_index = user_id_index.unwrap();
        let old_user_id_index = old_user_id_index.unwrap();

        // Parse the owners from BrdbValues
        let mut new_soa = OwnerTableSoA::try_from(&owners.to_value())?;
        let owners_lut = rows
            .trim()
            .split("\n")
            .map(|r| r.trim().split(",").collect::<Vec<&str>>())
            .map(|cols| {
                let user_name = cols[user_name_index];
                let display_name = cols[display_name_index];
                let user_id = Uuid::parse_str(&cols[user_id_index])
                    .expect(&format!("invalid uuid: {}", cols[user_id_index]));
                let old_user_id = Uuid::parse_str(&cols[old_user_id_index])
                    .expect(&format!("invalid old uuid: {}", cols[old_user_id_index]));
                (old_user_id, (user_name, display_name, user_id))
            })
            .collect::<HashMap<_, _>>();
        println!("{owners_lut:?}");

        let mut changes = 0;

        for i in 0..new_soa.user_ids.len() {
            let old_id = new_soa.user_ids[i].uuid();
            let Some((user_name, display_name, user_id)) = owners_lut.get(&old_id) else {
                println!("missing old id for {old_id} - ignoring");
                continue;
            };
            println!("replacing {old_id} with {user_id} - {user_name} ({display_name})");
            new_soa.user_names[i] = (*user_name).to_owned();
            new_soa.display_names[i] = (*display_name).to_owned();
            new_soa.user_ids[i] = Guid::from_uuid((*user_id).clone());
            changes += 1;
        }

        if changes == 0 {
            println!("world left unchanged");
            std::process::exit(0);
        }

        // convert the owners struct of arrays into bytes using the owners schema
        let content = db.owners_schema()?.write_brdb(OWNER_TABLE_SOA, &new_soa)?;

        let patch = BrPendingFs::Root(vec![(
            "World".to_owned(),
            BrPendingFs::Folder(Some(vec![(
                "0".to_string(),
                BrPendingFs::Folder(Some(vec![(
                    "Owners.mps".to_string(),
                    BrPendingFs::File(Some(content)),
                )])),
            )])),
        )]);
        db.write_pending("Replace owners", db.to_pending_patch()?.with_patch(patch)?)?;
        println!("revision created")
    }

    Ok(())
}
