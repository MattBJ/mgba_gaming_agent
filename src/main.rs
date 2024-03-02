use argparse::{ArgumentParser, List, Store};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;

pub mod c_land {
    use std::path::Path;
    use std::process::Command;
    pub fn spawn_server(dst_path: &Path, rom_path: &Path, server_name: &str) -> std::process::Child {
        // spawn server first
        // -C sgb.borders=0
        Command::new(format!("{}/{}", dst_path.display(), server_name))
            // .arg("-1") // server doesn't implement that arg apparently
            .arg("-C")
            .arg("sgb.borders=0") // but for some reason resolution is not updated properly?
            .arg(rom_path)
            .spawn()
            .expect("Why dont he want me man")
    }

    pub fn compile_binaries(
        include_path: &Path,
        src_path: &Path,
        dst_path: &Path,
        client_name: &str,
        server_name: &str,
        sdl2_lib: &str,
        mgba_lib: &str,
    ) {
        // ################################################################################
        // Compile Client and Server
        // ################################################################################

        println!(
            "Building the client: {}/{}",
            src_path.display(),
            client_name
        );
        // build client
        Command::new("gcc")
            .arg("-g3") // debug symbols
            .arg(format!("-I{}", include_path.display()))
            .arg(format!("{}/{}.c", src_path.display(), client_name))
            .arg(format!("-l{}", sdl2_lib))
            .arg("-o")
            .arg(format!("{}/{}", dst_path.display(), client_name))
            .output()
            .expect(&format!("Failed to compile the {}", client_name));

        println!(
            "Building the server: {}/{}",
            src_path.display(),
            server_name
        );
        println!(
            "gcc -I{} {}/{}.c -l{} -o {}/{}",
            include_path.display(),
            src_path.display(),
            server_name,
            mgba_lib,
            dst_path.display(),
            server_name
        );
        // build server
        Command::new("gcc")
            .arg("-g3") // debug symbols
            .arg(format!("-I{}", include_path.display()))
            .arg(format!("{}/{}.c", src_path.display(), server_name))
            .arg(format!("-l{}", mgba_lib)) // I didn't get error message when I linker gave the issue?
            .arg("-o")
            .arg(format!("{}/{}", dst_path.display(), server_name))
            .output()
            .expect(&format!("Failed to compile the {}", server_name));
    }

    pub fn run_client(
        dst_path: &Path,
        client_name: &str,
    ) -> std::process::Child {
        Command::new(format!("{}/{}", dst_path.display(), client_name))
            .spawn()
            .expect("Gahhdamn")
    }

    pub fn clean_and_exit(
        dst_path: &Path,
        client_name: &str,
        server_name: &str,
    ) {
        println!(
            "Cleaning the client: {}/{}",
            dst_path.display(),
            client_name
        );
        // build client
        Command::new("rm")
            .arg(format!("{}/{}", dst_path.display(), client_name))
            .output()
            .expect(&format!("Failed to compile the {}", client_name));

        println!(
            "Cleaning the server: {}/{}",
            dst_path.display(),
            client_name
        );
        Command::new("rm")
            .arg(format!("{}/{}", dst_path.display(), server_name))
            .output()
            .expect(&format!("Failed to compile the {}", server_name));
    }
}

// mod observation;
mod agent_stuff;
use agent_stuff::{
    Agent, AgentConfiguration,
    AgentDriver, GameConfigData,
};
// mod mgba_ffi; // need this line to invoke compiler on that module, good for testing

use std::process;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> process::ExitCode {
    // here is our agent_configuration_file
    let agent_config_file = Path::new("./agent_config.json");
    // assumes a configuration file
    let agent_config = agent_stuff::read_configuration_file(agent_config_file);
    
    let atomic_bool_rc = Arc::new(AtomicBool::new(false));
    let r = atomic_bool_rc.clone();

    ctrlc::set_handler(move || {
        println!("Captured SIGINT"); // they are still getting it, but why aren't they being written to?
        r.store(true, Ordering::SeqCst) // ordering::seqcst is important for this atomic business
    }).expect("Error setting Ctrl-C handler");

    // clean, client, headless_agent
    #[derive(Debug)]
    enum ProgramAction {
        Clean,   // clean c libraries. Might should be removed lol
        TestRun, // all of our development to this point
        // instead of just forking, going to use Management to do it better
        Management, // this will be either boss or worker
    }
    use std::str::FromStr;
    impl FromStr for ProgramAction {
        type Err = ();
        fn from_str(src: &str) -> Result<ProgramAction, ()> {
            return match src {
                // do some to_lower conversion
                "Clean" => Ok(ProgramAction::Clean),
                "TestRun" => Ok(ProgramAction::TestRun),
                "Management" => Ok(ProgramAction::Management),
                _ => Err(()),
            };
        }
    }
    // need to initialize to something, but doesn't matter the value
    let mut action = ProgramAction::Clean;
    // remaining potential sub_arguments to parse
    let mut sub_args = vec![];
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Rust mGBA utility suite");
        // management=boss
        // management=worker

        // then add options depending on match statement

        ap.refer(&mut action).add_argument(
            "action",
            Store,
            "Specify an action to take: Clean, TestRun, or Management",
        );
        ap.refer(&mut sub_args).add_argument(
            "sub arguments",
            List,
            // r#"Arguments for command"#
            "Sub arguments to potentially pass for further parsing",
        );
        ap.stop_on_first_argument(true);
        ap.parse_args_or_exit();
    }

    sub_args.insert(0, format!("action {:?}", action));
    let include_path = Path::new("../../../../../include");
    let src_path = Path::new("..");
    let dst_path = Path::new("c_binaries");
    const SDL2_LIB: &str = "SDL2";
    const MGBA_LIB: &str = "mgba";
    const SERVER_NAME: &str = "server";
    const CLIENT_NAME: &str = "client";
    match action {
        ProgramAction::Clean => {
            println!("Clean argument detected!");
            c_land::clean_and_exit(dst_path, CLIENT_NAME, SERVER_NAME);
        }
        // uses client-server communication
        ProgramAction::TestRun => {
            println!("TestRun argument detected!");
            #[derive(Debug)]
            // let's not get ahead of ourselves with options etc.
            enum MgbaTestScenario {
                OriginalC,     // c client, c server
                RustHybrid,    // rust client, c server
                TotalRust,     // rust client, rust server
                RustNoSockets, // no client-server, only rust Agent directly using mgba library/core
            }
            impl FromStr for MgbaTestScenario {
                type Err = ();
                fn from_str(src: &str) -> Result<MgbaTestScenario, ()> {
                    return match src {
                        // do some to_lower conversion
                        "OriginalC" => Ok(MgbaTestScenario::OriginalC),
                        "RustHybrid" => Ok(MgbaTestScenario::RustHybrid),
                        "TotalRust" => Ok(MgbaTestScenario::TotalRust),
                        "RustNoSockets" => Ok(MgbaTestScenario::RustNoSockets),
                        _ => Err(()),
                    };
                }
            }
            let mut test_scenario = MgbaTestScenario::OriginalC;
            {
                let mut ap = ArgumentParser::new();
                ap.set_description("Runs an mgba application variant, which makes use of varying tech stacks. C/Rust, and optional TCP socket comms");
                ap.refer(&mut test_scenario)
                    .add_argument(
                        "which test",
                        Store,
                        // r#"Output sink to play to"#
                        "which test scenario we want to run: OriginalC, RustHybrid, TotalRust, RustNoSockets",
                    );
                use std::io::{stderr, stdout};
                match ap.parse(sub_args, &mut stdout(), &mut stderr()) {
                    Ok(()) => {}
                    Err(x) => {
                        std::process::exit(x);
                    }
                }
            }
            // test_scenario should be set:
            match test_scenario {
                MgbaTestScenario::OriginalC => {
                    println!("Running the OriginalC test scenario!");
                    c_land::compile_binaries(
                        include_path,
                        src_path,
                        dst_path,
                        CLIENT_NAME,
                        SERVER_NAME,
                        SDL2_LIB,
                        MGBA_LIB,
                    );
                    let mut server_handle = c_land::spawn_server(dst_path, Path::new(&agent_config.game_config_data.rom_path), SERVER_NAME);
                    let mut client_handle = c_land::run_client(dst_path, CLIENT_NAME);
                    server_handle.wait().expect("server didn't exit properly?");
                    client_handle.wait().expect("client didn't exit properly?");
                }
                MgbaTestScenario::RustHybrid => {
                    println!("Running the RustHybrid test scenario!");
                    c_land::compile_binaries(
                        include_path,
                        src_path,
                        dst_path,
                        CLIENT_NAME,
                        SERVER_NAME,
                        SDL2_LIB,
                        MGBA_LIB,
                    );
                    let mut server_handle = c_land::spawn_server(dst_path, Path::new(&agent_config.game_config_data.rom_path), SERVER_NAME);

                    // let agent_config = agent_stuff::read_configuration_file();
                    let rusthybrid_agent_config = AgentConfiguration {
                        agent_driver: AgentDriver::Sockets(13721_u16),
                        ..agent_config.clone()
                    };
                    let mut mgba_agent = Agent::new(rusthybrid_agent_config,None);
                    let observation_data_set = mgba_agent.run_client();
                    server_handle
                        .kill()
                        .expect("Duhh we had trouble moyduhing the soyvuh boss");
                    println!("Observation data: {:?}", observation_data_set.clone().unwrap().len());
                    // observation_data_set.to_file(Path::new("foobar.json"));
                    observation_data_set.unwrap().save_album(Path::new("imageData/"));
                }
                MgbaTestScenario::TotalRust => {
                    println!("Running the TotalRust test scenario!");
                    println!("Wahoo!");

                    let mgba_port = 10103_u16;
                    let totalrust_agent_config = AgentConfiguration {
                        agent_driver: AgentDriver::Sockets(mgba_port),
                        ..agent_config.clone()
                    };
                    
                    // spawn the server with a thread
                    let server_handle = thread::spawn(move || {
                        agent_stuff::mgba_ffi::spawn_server(mgba_port,&agent_config.game_config_data);
                    });
                    
                    let mut mgba_agent = Agent::new(totalrust_agent_config,Some(atomic_bool_rc));
                    let observation_data_set = mgba_agent.run_client();
                    println!("Finished running our rust client-rust server impl!");
                    observation_data_set.unwrap().save_album(Path::new("imageData/"));
                    server_handle.join().expect("Couldn't join our rust server ;-;");
                }
                MgbaTestScenario::RustNoSockets => {
                    println!("Running the RustNoSockets test scenario!");
                    // println!("No socket connection all rust!!");

                    let mut mgba_agent = Agent::new(agent_config,Some(atomic_bool_rc));
                    let observation_data_set = mgba_agent.run_client();
                    println!("Finished running our rust core impl!");
                    observation_data_set.unwrap().save_album(Path::new("imageData/"));
                }
            }
        }
        ProgramAction::Management => {
            println!("Management argument detected!");
            // non option argument: boss or worker
            #[derive(Debug)]
            // let's not get ahead of ourselves with options etc.
            enum Management {
                Boss,
                Worker
            }
            impl FromStr for Management {
                type Err = ();
                fn from_str(src: &str) -> Result<Management, ()> {
                    return match src {
                        // do some to_lower conversion
                        "Boss" => Ok(Management::Boss),
                        "Worker" => Ok(Management::Worker),
                        _ => Err(()),
                    };
                }
            }
            //
            let mut process_type = Management::Boss;
            let mut server_uds_path = "".to_string();
            {
                let mut ap = ArgumentParser::new();
                ap.set_description("Runs a proper multi-process environment based on CLI args");
                ap.refer(&mut process_type).add_argument(
                    "process type",
                    Store,
                    // r#"Output sink to play to"#
                    "Which type of process are we in the Management paradigm: Boss, Worker",
                );
                ap.refer(&mut server_uds_path)
                    .add_option(
                        &["--socket"],
                        Store,
                        "Unix domain socket path -- should probably NOT be optional for the Worker.. idk how to tix"
                    );
                use std::io::{stderr, stdout};
                match ap.parse(sub_args, &mut stdout(), &mut stderr()) {
                    Ok(()) => {}
                    Err(x) => {
                        std::process::exit(x);
                    }
                }
            }
            match process_type {
                Management::Boss => {
                    println!("We are THE Boss Process");
                    let this_file = file!();
                    let _filename_only = Path::new(this_file)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    // use std::env::curent_exe; // dangerous to store
                    const TOTAL_WORKERS: u8 = 6_u8;

                    struct WorkerManager {
                        worker_process: process::Child,
                        unix_socket: std::os::unix::net::UnixStream, // unix domain socket stuff
                        agent_config: AgentConfiguration,
                    }
                    
                    // based on game path AND save state path, going to copy those into directory
                    
                    let mut worker_manager_list: Vec<WorkerManager> = vec![];
                    use std::os::unix::net::UnixListener;
                    
                    use std::path::PathBuf;
                    // Before we create filesystem, ensure that
                    let rom_path = Path::new(&agent_config.game_config_data.rom_path);
                    let sav_path = 
                        if agent_config.game_config_data.save_state_path.is_some() {
                            Some(PathBuf::from(agent_config.game_config_data.save_state_path.clone().unwrap()))
                        } else {None};

                    assert!(rom_path.exists());
                    if sav_path.is_some() {
                        assert!(sav_path.clone().unwrap().exists());
                    }
                    
                    let manager_temporary_fs = format!("/tmp/rust_mgba/{}", process::id());
                    fs::create_dir_all(manager_temporary_fs.clone()).expect("What the flub?");

                    // spawn loop
                    for spawn_worker_idx in 0..TOTAL_WORKERS {
                        println!(
                            "Boss is spawning a new Worker process #{}",
                            spawn_worker_idx
                        );
                        let worker_temporary_fs = format!("{}/{}",manager_temporary_fs,spawn_worker_idx);
                        fs::create_dir_all(worker_temporary_fs.clone()).expect("What the flubberino?");
                        // When spawning processes, pass in socket info
                        // probably not good to pass in something with PID ? Idk
                        let uds_listener = match UnixListener::bind(format!(
                            "{}/manager.sock",
                            worker_temporary_fs.clone()
                        )) {
                            Ok(sock) => sock,
                            Err(e) => {
                                println!("Couldn't bind: {e:?}");
                                return process::ExitCode::from(1);
                            }
                        };
                        // create copies of save file & save state
                        // Creating worker's agent_config
                        let worker_agent_config = {
                            let dest_rom_path = format!(
                                "{}/{}",
                                worker_temporary_fs.clone(),
                                rom_path.file_name().unwrap().to_str().unwrap(),
                            );
                            fs::copy(
                                rom_path,
                                dest_rom_path.clone(),
                            ).expect("Failed to copy game file data over");

                            // Check for a game save file (.sav) we can use (auto parsed based on ROM location)
                            {
                                let mut src_save_data_path = String::from_str(rom_path.to_str().unwrap()).expect("Pl0s");
                                // depending
                                // let mut sav_path = game_config_data.rom_path.clone();
                                let truncate_len = if src_save_data_path.find(".gba").is_some() {3} else {2};
                                src_save_data_path.truncate(src_save_data_path.len()-truncate_len);
                                src_save_data_path+="sav";
                                let src_save_data_path = Path::new(&src_save_data_path);
                                if src_save_data_path.exists() {
                                    // println!("src_save_data_path: {}",src_save_data_path.to_str().unwrap());
                                    let save_data_fname = String::from_str(src_save_data_path.file_name().unwrap().to_str().unwrap()).expect("naur");
                                        // .truncate(game_config_data.rom_path.len()-3)+"sav";
                                    // save path
                                    let dest_save_data_path = format!(
                                        "{}/{}",
                                        worker_temporary_fs.clone(),
                                        save_data_fname,
                                    );
                                    // println!("Copying save data paths: {} -> {}",src_save_data_path.to_str().unwrap(),dest_save_data_path,);
                                    fs::copy(
                                        src_save_data_path,
                                        dest_save_data_path.clone(),
                                    ).expect("Failed to copy game file data over");
                                }
                            }
                            
                            // if we want to make use of a save state file (optional GameConfigData field)
                            let dest_sav_path = if sav_path.is_some() {
                                let dest_sav_path = format!(
                                    "{}/{}",
                                    worker_temporary_fs.clone(),
                                    sav_path.clone().unwrap().file_name().unwrap().to_str().unwrap(),
                                );
                                fs::copy(
                                    sav_path.clone().unwrap(),
                                    dest_sav_path.clone(),
                                ).expect("Failed to copy game file data over");
                                Some(dest_sav_path)
                            } else {None};

                            AgentConfiguration {
                                game_config_data: GameConfigData{
                                    rom_path: dest_rom_path,
                                    save_state_path: dest_sav_path,
                                },
                                ..agent_config.clone()
                            }
                        }; // new agent_configuration


                        println!(
                            "Created uds: {}",
                            uds_listener
                                .local_addr()
                                .expect("Burp")
                                .as_pathname()
                                .unwrap()
                                .display()
                        );
                        // Bind our socket server for this process, we will communicate via unix sockets
                        // Command::new(current_exe()) // bad sec?
                        let mut worker_handle = 
                            Command::new("cargo")
                                // can we pass any args to cargo to not annoy us
                                .arg("run") // run our specific binary (safer than using current_exe?)
                                .arg("--release") // run release optimized for worker
                                .arg("--") // start parsing args for program
                                .arg("Management") // use the Management paradigm
                                .arg("Worker") // running a Worker process
                                .arg(
                                    format!(
                                        "--socket={}",
                                        uds_listener
                                            .local_addr()
                                            .expect("Burp")
                                            .as_pathname()
                                            .unwrap()
                                            .display()
                                    )
                                )
                                // .stderr(Stdio::null()) // let's not read the error?
                                .spawn()
                                .expect(
                                    format!("Failed to spawn worker #{}", spawn_worker_idx).as_str(),
                                );
                        // accept a connection
                        // write over the configuration
                        // tell it to run?
                        match uds_listener.accept() {
                            Ok((socket, _addr)) => {
                                // could do some initialization in here
                                worker_manager_list.push(WorkerManager {
                                    worker_process: worker_handle,
                                    unix_socket: socket,
                                    agent_config: worker_agent_config,
                                })
                            }
                            Err(e) => {
                                // kill all the worker handles?
                                worker_handle.kill().expect("Worker rose up and couldn't be killed O_o..");
                                panic!("accept function failed: {e:?}");
                            }
                        }
                    } // spawn loop

                    // Beginning of IPC State machine
                    // This one is simply initialize and GO (meaning send an Agent configuraiton)
                    
                    // println!("Boss: About to serialize agent_config for our Workers, with value:\n{:?}",agent_config);
                    
                    use std::io::prelude::*;
                    // initialize the Workers with proper configuration
                    for worker_manager in &mut worker_manager_list {
                        println!("Boss: Sending configuration {:?}",worker_manager.agent_config);
                        let serialized_agent_config = serde_json::to_string(&worker_manager.agent_config).expect("serde pls");
                        println!("Boss: Serialized configuration {:?}",serialized_agent_config);
                        // Write message length
                        let message_len_data = (serialized_agent_config.len() as u32).to_be_bytes();
                        println!("Boss: Finished serializing, sending data");
                        worker_manager.unix_socket.write(&message_len_data)
                            .expect("Boss: Failure to write to unix socket 0");
                        // Write message
                        worker_manager.unix_socket.write_all(&serialized_agent_config.clone().into_bytes())
                            .expect("Boss: Failure to write to unix socket 1");
                        println!("Boss: Data sent to Worker!");
                    } // initialize configuration loop

                    // Overwrite the signal handler here for this Boss!
                    
                    // wait for Workers loop
                    for worker_manager in &mut worker_manager_list {
                        // non blocking wait
                        // or kill the Worker
                        'worker_pending_loop: loop {
                            match worker_manager.worker_process.try_wait() {
                                // some exit status available
                                Ok(Some(_status)) => {break 'worker_pending_loop;},
                                // still running, go to sleep
                                Ok(None) => thread::sleep(Duration::from_millis(50)),
                                Err(e) => {panic!("Ran into some error: {}",e);}
                            }
                        }
                    } // wait for Workers loop

                    // Don't change this! Dropping the PID socket_path_tree, maybe unnecessary?
                    fs::remove_dir_all(manager_temporary_fs.clone())
                        .expect(format!("Failure to remove dir: {:?}",manager_temporary_fs).as_str());
                }
                Management::Worker => {
                    println!("We are a Worker Process");
                    // just spawns the stuffs we want
                    println!("Path to socket: {}", server_uds_path);
                    use std::io::prelude::*;
                    use std::os::unix::net::UnixStream;
                    let mut stream = UnixStream::connect(format!("{}", server_uds_path))
                        .expect("Couldn't connect to Boss' socket, DOY!!");

                    // Read some sockets data for Agent configuration

                    // Read in a serialized version of our Agent configuration
                    // read size first

                    // let mut buf = [0_u8; 1];
                    let mut buf = [0_u8; 4];
                    println!("Worker: About to read message length from Boss");
                    stream.read_exact(&mut buf).expect("Worker: Failure to read message length");
                    let message_len = u32::from_be_bytes(buf);
                    println!("Worker: About to read in Agent configuration");
                    // Agent configuration comes from parent process
                    let mut incoming_configuration = vec![0_u8; message_len as usize];
                    stream.read_exact(&mut incoming_configuration[..]).expect("Worker: Failure to read Agent configuration");
                    println!("Worker: Received configuration data (size {} bytes)",message_len);
                    println!("Worker: Raw data: {}",std::str::from_utf8(&incoming_configuration[..]).expect(""));
                    let worker_agent_config: AgentConfiguration =
                        serde_json::from_slice(&incoming_configuration)
                            .expect("Trouble parsing data");
                    println!("Worker: Successfully parsed worker_agent_config");
                    // println!(
                    //     "Worker: Finished reading in configuration:\n{:?}",
                    //     worker_agent_config
                    // );

                    // run_client is running indefinitely
                    // but somehow we need to SIGNAL it to stop -- batch size? Part of configuration?
                    // Should also be able to just signal it to stop and do last activities
                    let mut mgba_agent = Agent::new(worker_agent_config,Some(atomic_bool_rc));
                    let _observation_data_set = mgba_agent.run_client();
                    // any final steps for this run, do we need to save some data?
                    // Store info about a neural network? --> This gets referenced via worker_agent_config
                    println!("Finished running our rust core impl!");
                    // observation_data_set.save_album(Path::new("imageData/"));
                }
            }
        }
    };

    process::ExitCode::from(0)
}
