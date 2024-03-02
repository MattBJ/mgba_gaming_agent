// Declare our modules
pub mod mgba_ffi; // this is private to user API
mod sdl_window;
mod observation;

// pub mod agent_stuff { // redundant here
// use super::sdl_window::{
use sdl_window::SdlWindow;

use sdl2::pixels::PixelFormatEnum;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpStream};
use std::option::Option;
use std::thread;
use std::time::{Duration, Instant};

// Going to be saving this data to files
use serde::{Deserialize, Serialize};
use std::path::Path;

// just use all of observation
// use observation::*;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)] // in order to do != operations!
pub enum AgentControl {
    Human,  // this is already how we're IO'ing it
    Replay, // this is going to be implemented now
    // replay output keycode iterable
    Intelligent, // this will be implemented later
}

use std::fmt;
impl fmt::Display for AgentControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>)-> fmt::Result {
        match self {
            Self::Human => write!(f, "AgentControl::Human"),
            Self::Replay => write!(f, "AgentControl::Replay"),
            Self::Intelligent => write!(f, "AgentControl::Intelligent"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentDriver {
    Sockets(u16), // port
    Native,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameConfigData {
    pub rom_path: String, // Path? ffi::CString?
    pub save_state_path: Option<String>, // Path? ffi::CString?
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClockRate {
    pub rate: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EmuClockMgr {
    Clock(ClockRate),
    CycleIgnore(u32),
}

impl ClockRate {
    pub fn get_duration(&self) -> Duration {
        Duration::from_micros((1_000_000/(self.rate)) as u64)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfiguration {
    pub agent_control: AgentControl,
    pub render_condition: bool,
    pub store_observations: bool,
    pub emu_clock_mgr: Option<EmuClockMgr>,
    pub agent_driver: AgentDriver,
    pub game_config_data: GameConfigData,
}

pub fn read_configuration_file<'a>(file_path: &'a Path) -> AgentConfiguration {
    use std::fs;
    let file_read = fs::read_to_string(file_path)
        .expect(format!("Error reading configuration file: {}",file_path.display()).as_str());
    serde_json::from_str::<AgentConfiguration>(&file_read)
        .expect(format!("Trouble parsing data from configuration file: {}",file_path.display()).as_str())
}

pub enum AgentIO {
    DirectIO(observation::ObservationData),
    SdlIO(SdlWindow),
}

impl fmt::Display for AgentIO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>)-> fmt::Result {
        match self {
            Self::DirectIO(_observation_data) => write!(f, "AgentIO::DirectIO"),
            Self::SdlIO(_sdl_window) => write!(f, "AgentIO::SdlIO"),
        }
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
// #[ouroboros::self_referencing]
pub struct Agent {
    agent_config: AgentConfiguration,
    agent_io: AgentIO,
    tcp_stream: Option<TcpStream>,
    mgba_core: Option<mgba_ffi::CoreData>,
    agent_stop_flag: Arc<AtomicBool>,
    stop_flag_polling_period: u32,
    cycle_duration: Option<Duration>,
}

/// This is the central struct of the library
/// init_core -> run the mgba emulator core inside of this struct
/// AgentConfiguration, run_client, and stop_flag are the only ways
/// the user interacts with this Agent right now.
impl Agent {
    pub fn new(agent_config: AgentConfiguration, agent_stop_flag: Option<Arc<AtomicBool>>) -> Agent {

        // if agent_config.emu_clock_mgr.is_some()

        let stop_flag_polling_period: u32 = match &agent_config.emu_clock_mgr {
            Some(EmuClockMgr::Clock(clock_rate)) => clock_rate.rate,
            _=>5000_u32,
        };
        
        let tcp_stream: Option<TcpStream>;

        let mgba_core: Option<mgba_ffi::CoreData>;
        let agent_io: AgentIO;

        (agent_io, mgba_core, tcp_stream) = match agent_config.agent_driver {
            AgentDriver::Native => Self::init_core(
                agent_config.render_condition,
                &agent_config.game_config_data
            ),
            AgentDriver::Sockets(mgba_port) => Self::init_connection(
                agent_config.render_condition,
                &agent_config.game_config_data,
                mgba_port
            ),
        };

        let cycle_duration = match &agent_config.emu_clock_mgr {
            Some(EmuClockMgr::Clock(clock_gen))=> Some(clock_gen.get_duration()),
            _=>None,
        };

        Agent {
            agent_config: agent_config,
            agent_io: agent_io, // observation data and optional sdl rendering environment
            tcp_stream: tcp_stream,
            // sdl_window: sdl_window, // moved into agent_io
            mgba_core: mgba_core,
            // observation: observation, // moved into agent_io
            agent_stop_flag: agent_stop_flag
                .unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
            stop_flag_polling_period: stop_flag_polling_period,
            cycle_duration: cycle_duration,
        }
    }

    // only called in the new function
    fn init_connection(
        render_condition: bool,
        // passing of game ROM data to server
        _game_config_data: &GameConfigData,
        mgba_port: u16
    ) -> (AgentIO, Option<mgba_ffi::CoreData>, Option<TcpStream>) {
        println!("Client connecting to port {}", mgba_port);
        let mgba_addr = SocketAddr::from(([127, 0, 0, 1], mgba_port)); // hard coded port -- DEFAULT port in mgba
        thread::sleep(Duration::from_millis(5));
        let mut tcp_stream = TcpStream::connect_timeout(&mgba_addr, Duration::from_millis(1000))
            .expect("Ruh roh shaggy");

        // assert we read 4 bytes each
        println!("Reading width");
        let mut buf = [0_u8; 4];
        tcp_stream.read_exact(&mut buf).expect("Initial reading from tcp stream server failed: 0");
        let width = u32::from_be_bytes(buf);
        println!("Reading height");
        tcp_stream.read_exact(&mut buf).expect("Initial reading from tcp stream server failed: 1");
        let height = u32::from_be_bytes(buf);
        println!("Reading bpp");
        tcp_stream.read_exact(&mut buf).expect("Initial reading from tcp stream server failed: 2");
        let bpp = u32::from_be_bytes(buf);

        // Didn't have to use 'ne' for network order, it comes properly through stream read from_ne_bytes
        println!("Agent.init_connection: width={}", width);
        println!("Agent.init_connection: height={}", height);
        println!("Agent.init_connection: bpp={}", bpp);

        // (*frame_data) = vec![0_u8; (width*height*bpp) as usize];

        let pixel_format: PixelFormatEnum = match bpp {
            2 => PixelFormatEnum::RGB565,
            4 => PixelFormatEnum::ABGR8888,
            _ => panic!("This is not documented"),
        };

        let observation_data = observation::ObservationData {
            // This will be overwritten
            frame_buffer: observation::FrameBuffer::new(width,height,bpp,pixel_format),
            keycode_data: 0_u16,
        };
        
        let agent_io: AgentIO = match render_condition {
            // create a barebones observation_data format
            false => AgentIO::DirectIO(observation_data),
            true => AgentIO::SdlIO(SdlWindow::new("Newly organized window",observation_data)),
        };

        (
            agent_io,
            None,
            Some(tcp_stream),
        )
    }

    fn init_core(
        render_condition: bool,
        game_config_data: &GameConfigData
    ) -> (AgentIO, Option<mgba_ffi::CoreData>, Option<TcpStream>) {
        let observation_data: observation::ObservationData;
        let core_data: mgba_ffi::CoreData;
        (observation_data,core_data) = mgba_ffi::init_core(game_config_data);
        let agent_io: AgentIO = match render_condition {
            // create a barebones observation_data format
            false => AgentIO::DirectIO(observation_data),
            // create an SdlWindow with the desired observation_data format
            true => AgentIO::SdlIO(SdlWindow::new("Newly organized window",observation_data)),
        };
        (
            agent_io,
            Some(core_data),
            None,
        )
    }

    #[inline(always)]
    // fn get_observation(&mut self) -> observation::ObservationData {
    fn get_observation(&mut self) {
        let observation_data = 
        match &mut self.agent_io {
            AgentIO::DirectIO(observation_data) => observation_data, // this compiles without & too
            AgentIO::SdlIO(sdl_window) => &mut (sdl_window.observation_data),
        };//.frame_buffer.post_process_data();
        observation_data.frame_buffer.post_process_data();
        // return the observation
        // observation_data.clone()
    }
    
    fn execute_cycle(&mut self) {
        let output_keycode = match &mut self.agent_io {
            AgentIO::DirectIO(ref mut observation_data) => observation_data.keycode_data,
            AgentIO::SdlIO(ref mut sdl_window) => sdl_window.observation_data.keycode_data,
        };
        // Execute an emulator cycle, write to new input
        match &mut self.agent_config.agent_driver {
            AgentDriver::Sockets(_) => {
                // Only way to make reference mutable is.. clone?
                // let mut stream = tcp_stream.try_clone().expect("");
                let mut tcp_stream = self.tcp_stream.as_ref().unwrap();
                tcp_stream.write(&output_keycode.to_be_bytes()).expect("Writing keycode to tcp stream failure");

                // read in frame_buffer from TCP stream into correct location in memory:
                match &mut self.agent_io {
                    AgentIO::DirectIO(ref mut observation_data)=>{
                        tcp_stream.read_exact(
                            &mut observation_data.frame_buffer.frame_data[..]
                        ).expect("Reading frame buffer from tcp stream failure");
                    },
                    AgentIO::SdlIO(ref mut sdl_window)=>{
                        tcp_stream.read_exact(
                            &mut sdl_window.observation_data.frame_buffer.frame_data[..]
                        ).expect("Reading frame buffer from tcp stream failure");
                    }
                }
            },
            AgentDriver::Native => unsafe {
                mgba_ffi::execute_core_cycle(
                    self.mgba_core.as_ref().expect(""),
                    output_keycode,
                );
            },
        }
    }

    pub fn run_client(&mut self) -> Option<observation::ObservationSet> {
        let mut ret_val: Option<observation::ObservationSet> 
            = match self.agent_config.store_observations {
                false => None,
                true => {
                    let observation_data = match &self.agent_io {
                        AgentIO::DirectIO(observation_data)=>&observation_data,
                        AgentIO::SdlIO(sdl_window)=>&sdl_window.observation_data,
                    };
                    Some(observation::ObservationSet::new(
                        None,
                        observation_data.frame_buffer.width,
                        observation_data.frame_buffer.height,
                        observation_data.frame_buffer.pixel_format,
                    ))
                },
        };
        
        let mut cycle_counter = 0_u32;
        let mut _frame_counter = 0_u32;
        let emu_loop_time = Instant::now();
        let mut cycle_timer = Instant::now();
        // Lower this is, higher our CPU usage
        // 1 micro = 99% usage, 100 micro = 7.5 % usage. Wow
        // 1 micro = we can request 1MFPS, which we know can't run
        // Max support right now should be like 6000 FPS, which 1/6000 = 166 micros max
        // just going with 100 for the in between, 7.5
        // 166 uses like 7% too. So not that big of a deal
        let cycle_sleep_dur = Duration::from_micros(100);
        'agent_loop_cycle: loop {
            let mut io_control_flow = true;
            // This is our hot loop
            match (0 == (cycle_counter % self.stop_flag_polling_period))
            // SeqCst, Acquire, and Relaxed
                .then_some(self.agent_stop_flag.load(Ordering::Relaxed))
            {
                // Either we didn't poll (returned None)
                // or we polled a false (continue running)
                None | Some(false) => {
                    // based on OPTIONAL emu clock manager, check some OPTIONAL things
                    match &self.agent_config.emu_clock_mgr {
                        // return immediately if None
                        None=>(),
                        Some(emu_clock_mgr)=> match emu_clock_mgr {
                            EmuClockMgr::CycleIgnore(cycles_ignore)=> {
                                if (0!=((cycle_counter+1) % (cycles_ignore+1))) {
                                    io_control_flow = false;
                                }
                            },
                            // Add a both case here. Anything with CycleIgnore has priority
                            EmuClockMgr::Clock(_)=> {
                                // already calculated self.cycle_duration based on Clock's cycle_rate
                                // So I don't need to run the calculation every loop
                                while cycle_timer.elapsed() < self.cycle_duration.unwrap() {
                                    thread::sleep(cycle_sleep_dur);
                                }
                                cycle_timer = Instant::now();
                            },
                        },
                    }

                    // Do some IO depending on io_control_flow
                    // Pattern matching with io_control_flow, render condition, and control mechanism
                    // io off means we don't render to screen NOR do any processing
                    if io_control_flow {
                        // Potentially render the frame to an Sdl instance based on a render condition
                        // If not rendering, this block is entirely useless
                        match &mut self.agent_io {
                            // make this first pattern quick
                            AgentIO::DirectIO(_) => (),
                            AgentIO::SdlIO(ref mut sdl_window) => sdl_window.update(),
                        }

                        // Generate output from Agent OR human (or replay?)
                        match self.agent_config.agent_control {
                            AgentControl::Human => { // always here right now
                                match &mut self.agent_io {
                                    AgentIO::DirectIO(ref mut observation_data)=>{
                                        observation_data.keycode_data = 0_u16; // for now, just return 0 every time
                                    },
                                    AgentIO::SdlIO(ref mut sdl_window)=>{
                                        match sdl_window.matt_events() {
                                            Some(keycode) => {sdl_window.observation_data.keycode_data = keycode;},
                                            None => break 'agent_loop_cycle,
                                        }
                                    },
                                }
                            }
                            AgentControl::Replay => {
                                static FRAME_COUNTER: u32 = 0_u32;
                                let tmp = format!("imgData/{:0>4}", FRAME_COUNTER);
                                let replay_path = Path::new(&tmp);
                                // replay_path = replay_path.join(format!("{:0>4}/",FRAME_COUNTER));
                                println!("{:?}", replay_path);
                                // println!("{}",format!("{:?}{:0>4}",replay_path,FRAME_COUNTER));
                                break 'agent_loop_cycle;
                                // step through to next output frame
                                // is probably better to read this from somewhere in self, which was already preprocessed
                                FRAME_COUNTER += 1;
                                // 0_u16
                            }
                            // Use our previous frame's observation (self.observation.frame_data) to base keycode
                            AgentControl::Intelligent => panic!("dead code!!!"),
                        };
                        
                        if self.agent_config.store_observations {
                            let _ = self.get_observation();
                            // ret_val.as_mut().unwrap().push(self.get_observation());
                        }
                        _frame_counter+=1;
                    } // Observationdata aka Frame Encounter
                    self.execute_cycle();
                    cycle_counter += 1;
                }
                Some(true) => break 'agent_loop_cycle,
            }
        }
        println!(
            "mGBA runtime: {:?}, mGBA average FPS: {:?}",
            emu_loop_time.elapsed(),
            ((cycle_counter as f64) / (emu_loop_time.elapsed().as_millis() as f64) * 1000.0)
        );
        // calculate average FPS based on cycle_counter

        // return ObservationData
        ret_val
    }
}
