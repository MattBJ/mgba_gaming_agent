// FFI for critical C API's -- look over ../../server.c

#[allow(non_snake_case,improper_ctypes_definitions,non_upper_case_globals,non_camel_case_types,dead_code,unreachable_code)]
pub mod mgba_bindings;
use mgba_bindings::{
    mgba, mCore, mLogger,
    BYTES_PER_PIXEL, mLogLevel,
    __va_list_tag, mCoreConfig,
    VFile,
};

use std::io::prelude::*;
use std::net::TcpListener;

// a couple of redefinitions? had to comment out
// bindgen ./wrapper.h -- -I../../../../../include > ../src/mgba_ffi/mgba_bindings.rs
use std::ffi;


use super::{
    GameConfigData,
    observation::{
        ObservationData,
        FrameBuffer
    }
};
use sdl2::pixels::PixelFormatEnum;

// pub fn spawn_server(port_listen: u16, rom_path: String) {
pub fn spawn_server(port_listen: u16, game_config_data: &GameConfigData,) {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port_listen)).unwrap();
    // spawn a tcp connection

    // this loop stays alive forever? Whatever for now, only expect once
    println!("Waitin for connection at {}", port_listen);
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Connection established! Stream = {:?}", stream);
        // get the core/observation data thingy
        let observation_data: ObservationData;
        let core_data: CoreData;
        (observation_data,core_data) = init_core(game_config_data);

        // send over socket
        stream.write(&observation_data.frame_buffer.width.to_be_bytes()).expect("Error writing to tcp socket stream 1");
        stream.write(&observation_data.frame_buffer.height.to_be_bytes()).expect("Error writing to tcp socket stream 2");
        stream.write(&observation_data.frame_buffer.bpp.to_be_bytes()).expect("Error writing to tcp socket stream 3");
        
        // ##########################################################################################
        // mgba STEADY STATE phase
        // ##########################################################################################
        let mut client_keycode_message_buffer = [0_u8; 2];

        // let server_stream_buffer = unsafe {
        //     std::slice::from_raw_parts(
        //         &mut observation_data.frame_buffer.frame_data[..] as *mut u8,
        //         observation_data.frame_buffer.get_size().try_into().unwrap(),
        //     )
        // };
        
        loop {
            // thread::sleep(Duration::from_millis(2000));
            stream
                .read_exact(&mut client_keycode_message_buffer)
                .expect("Error reading client keycode from stream");
            let client_keycode = u16::from_be_bytes(client_keycode_message_buffer);

            unsafe { execute_core_cycle(&core_data,client_keycode) };

            // stream.write(server_stream_buffer).expect("Error writing to tcp socket stream 0");
            stream.write(&observation_data.frame_buffer.frame_data[..]).expect("Error writing to tcp socket stream 0");
        }
    }
}

pub fn init_core(
    game_config_data: &GameConfigData,
) -> (ObservationData, CoreData) {
    
    let libmgba_so_path = ffi::OsStr::new("libmgba.so.0.11");
    // load in dynamic library!
    let loaded_mgba_lib = unsafe {
        mgba::new(libmgba_so_path).expect("Error loading in dynamic libmgba.so lib")
    };

    let rom_path = ffi::CString::new(game_config_data.rom_path.as_str()).expect("Could not convert ROM path");
    let mut sav_path = game_config_data.rom_path.clone();
    let truncate_len = if sav_path.find(".gba").is_some() {3} else {2};
    sav_path.truncate(sav_path.len()-truncate_len);
    sav_path+="sav";
    // println!("sav_path: {}",sav_path);
    let sav_path = ffi::CString::new(sav_path.as_str()).expect("naur");
        // .truncate(game_config_data.rom_path.len()-3)+"sav";
    // save path

    let idle_optimization = ffi::CString::new("idleOptimization").expect("CString::new failed");
    let detect = ffi::CString::new("detect").expect("CString::new failed");
    let sgb_borders = ffi::CString::new("sgb.borders").expect("");
    // let log_level = ffi::CString::new("logLevel").expect("");

    let mut allocated_width: ffi::c_uint = 0; // = 0_u32;
    let mut allocated_height: ffi::c_uint = 0; // = 0_u32;
    
    let mut observation_data: ObservationData;
    // begin unsafe C API section
    let core_data: CoreData = unsafe {
        // struct mLogger logger = { .log = _log };
        // let logger: Box<mgba_bindings::mLogger> =
        //     Box::new_zeroed();
        let mut logger: Box<mgba_bindings::mLogger> =
            Box::new(mgba_bindings::mLogger {
                log:Some(log_handler),
                filter:std::ptr::null::<*mut mgba_bindings::mLogFilter>() as *mut mgba_bindings::mLogFilter,
            });
        
        loaded_mgba_lib.mLogSetThreadLogger(&mut *logger as *mut mgba_bindings::mLogger);
        
        let core: *mut mgba_bindings::mCore = loaded_mgba_lib.mCoreFind(rom_path.as_ptr());
        if core.is_null() {
            panic!("We have no core, abort mission!!");
        }
        
        (*core).init.unwrap()(core);

        loaded_mgba_lib.mCoreLoadFile(core, rom_path.as_ptr());
        
        loaded_mgba_lib.mCoreConfigInit(&mut (*core).config, std::ptr::null());

        loaded_mgba_lib.mCoreConfigLoad(&mut (*core).config);

        // apply arguments, mArgumentsApply,mCoreConfigSetDefaultValue,
        
        loaded_mgba_lib.mCoreConfigSetDefaultValue(
            &mut (*core).config,
            idle_optimization.as_ptr(),
            detect.as_ptr(),
        );
        // mCoreConfigSetDefaultIntValue(&m_config, "sgb.borders", 1);
        
        loaded_mgba_lib.mCoreConfigSetDefaultIntValue(
            &mut (*core).config,
            sgb_borders.as_ptr(),
            0,
        );
        
        loaded_mgba_lib.mCoreLoadConfig(core);

        (*core).currentVideoSize.unwrap()(core, &mut allocated_width, &mut allocated_height); // idk how to get a mutable raw pointer reference?
        
        // need to supply a frame buffer

        let pixel_format: PixelFormatEnum = match BYTES_PER_PIXEL {
            2 => PixelFormatEnum::RGB565,
            4 => PixelFormatEnum::ABGR8888,
            _ => panic!("This is not documented"),
        };

        observation_data = ObservationData {
            // This will be overwritten
            frame_buffer: FrameBuffer::new(
                allocated_width,
                allocated_height,
                BYTES_PER_PIXEL,pixel_format
            ),
            keycode_data: 0_u16,
        };
        
        (*core).setVideoBuffer.unwrap()(
            core,
            observation_data.frame_buffer.frame_data.as_mut_ptr() as *mut u32,
            allocated_width.try_into().unwrap(),
        );

        // So, not even keeping track of this
        // Seems as if the core deinit'ing will close the file
        let save_file = Box::new(loaded_mgba_lib.VFileOpen(
            sav_path.as_ptr(),
            mgba_bindings::O_RDWR as i32,
        ));
        (*core).loadSave.unwrap()(core,&mut *(*save_file) as *mut VFile);

        // Reset the core. This is needed before it can run.
        (*core).reset.unwrap()(core);

        // loading a state file if we want to, otherwise load the save file
        match &game_config_data.save_state_path {
            Some(path) => {
                // let x: () = path;
                let save_state = ffi::CString::new(path.as_str()).expect("Could not convert save_state path");
                let vf = loaded_mgba_lib.VFileOpen(
                    save_state.as_ptr(),
                    mgba_bindings::O_RDONLY as i32,
                );
                // would need to make bindings for serialize.h at least in addition
                // but for now just defining here
                const _SAVESTATE_SCREENSHOT: u8 = 1;
                const _SAVESTATE_SAVEDATA: u8 = 2;
                const _SAVESTATE_CHEATS: u8 = 4;
                const _SAVESTATE_RTC: u8 = 8;
                const _SAVESTATE_METADATA: u8 = 16;
                const _SAVESTATE_ALL: u8 = 31;
                loaded_mgba_lib.mCoreLoadStateNamed(core,vf,_SAVESTATE_RTC.into());
                (*vf).close.unwrap()(vf);
            },
            None => (),
        }
        
        // return the core
        CoreData::new(core,loaded_mgba_lib,logger,)
    };

    (observation_data,core_data)
}

#[inline(always)]
pub unsafe fn execute_core_cycle(core_data: &CoreData, keycode_data: u16) {
    // Assumes the core is already setup at this point, that's why it's unsafe
    (*(core_data.core)).setKeys.unwrap()(core_data.core, keycode_data.into());
    (*(core_data.core)).runFrame.unwrap()(core_data.core);
}

// all of these lifetimes need to correspond with agent
pub struct CoreData {
    // Can create a drop for mCore that would call its deinit things.
    // Would also need to be packaged with the bindings themselves,
    // as it requires a library call to deinig a core->config
    // Not going to worry about organizing this better right now, @TODO!
    pub core: *mut mCore,
    // These 2 only needed as pub for constructor
    mgba_lib: mgba,
    // Data about the logger, needed such that stdout doesn't get flooded with mgba logs
    _logger: Box<mLogger>,
}

impl CoreData {
    pub fn new(
        core: *mut mCore,
        mgba_lib: mgba,
        _logger: Box<mLogger>,
    ) -> Self {
        Self {core,mgba_lib,_logger,}
    }
}

impl Drop for CoreData {
    fn drop(&mut self) {
        // printing just for debugging purposes, like did we actually drop our memory?
        println!("Dropping some CoreData!");
        // run mGBA deinit process
        unsafe {
            // Deinitialization associated with the core.
            self.mgba_lib.mCoreConfigDeinit(&mut (*(self.core)).config as *mut mCoreConfig);
            (*(self.core)).deinit.unwrap()(self.core);
        }
    }
}


#[no_mangle]
pub extern "C" fn log_handler(
    _arg1: *mut mLogger,
    _category: ::std::os::raw::c_int,
    _level: mLogLevel,
    _format: *const ::std::os::raw::c_char,
    _args: *mut __va_list_tag,
) {
    // println!("WAAAAH");
    // Do nothing!
}