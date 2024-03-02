use std::fs::{
    create_dir, // will have an error if dir already exists :')
    // read_dir,
    File,
};
use std::path::Path;
use serde::{Deserialize, Serialize};
use sdl2::pixels::{PixelFormatEnum,};
use std::io::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(remote = "PixelFormatEnum")]
pub enum PixelFormatEnumDef {
    Unknown = 0 as isize,
    Index1LSB = 286_261_504 as isize,
    Index1MSB = 287_310_080 as isize,
    Index4LSB = 303_039_488 as isize,
    Index4MSB = 304_088_064 as isize,
    Index8 = 318_769_153 as isize,
    RGB332 = 336_660_481 as isize,
    RGB444 = 353_504_258 as isize,
    RGB555 = 353_570_562 as isize,
    BGR555 = 357_764_866 as isize,
    ARGB4444 = 355_602_434 as isize,
    RGBA4444 = 356_651_010 as isize,
    ABGR4444 = 359_796_738 as isize,
    BGRA4444 = 360_845_314 as isize,
    ARGB1555 = 355_667_970 as isize,
    RGBA5551 = 356_782_082 as isize,
    ABGR1555 = 359_862_274 as isize,
    BGRA5551 = 360_976_386 as isize,
    RGB565 = 353_701_890 as isize,
    BGR565 = 357_896_194 as isize,
    RGB24 = 386_930_691 as isize,
    BGR24 = 390_076_419 as isize,
    RGB888 = 370_546_692 as isize,
    RGBX8888 = 371_595_268 as isize,
    BGR888 = 374_740_996 as isize,
    BGRX8888 = 375_789_572 as isize,
    ARGB8888 = 372_645_892 as isize,
    RGBA8888 = 373_694_468 as isize,
    ABGR8888 = 376_840_196 as isize,
    BGRA8888 = 377_888_772 as isize,
    ARGB2101010 = 372_711_428 as isize,
    YV12 = 842_094_169 as isize,
    IYUV = 1_448_433_993 as isize,
    YUY2 = 844_715_353 as isize,
    UYVY = 1_498_831_189 as isize,
    YVYU = 1_431_918_169 as isize,
    NV12 = 842_094_158 as isize,
    NV21 = 825_382_478 as isize,
}


#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    #[serde(with = "PixelFormatEnumDef")]
    pub pixel_format: PixelFormatEnum, // 2 bpp: RGB565,4 bpp: ABGR8888
    pub frame_data: Vec<u8>, // vector of frame buffer bytes
    pub processed_data: Vec<u8>,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32, bpp: u32, pixel_format: PixelFormatEnum) -> Self {
        Self {
            width: width,
            height: height,
            bpp: bpp,
            pixel_format: pixel_format,
            // frame_data: vec![vec![0_u8 ; ((width*height*bpp) as usize)]],
            frame_data: vec![0_u8; (width * height * bpp) as usize], // RGBA or RGB565?? Never see the latter
            // frame_data: Vec::with_capacity((width * height * bpp) as usize),
            processed_data: vec![0_u8; (width * height * 1) as usize], // grayscale
            // processed_data: vec![0_u8; (width * height * (bpp-1)) as usize], // RGB
        }
    }
    
    // pub fn from_data(width: u32, height: u32, bpp: u32, pixel_format: PixelFormatEnum, frame_data: Vec<u8>) -> Self {
    //     Self {width,height,bpp,pixel_format,frame_data}
    // }
    
    pub fn get_size(&self) -> usize {
        (self.width * self.height * self.bpp) as usize
    }

    pub fn get_buffer(&mut self) -> &mut [u8] {
        // return mutable reference
        &mut self.frame_data[..]
    }

    pub fn write_buffer(&mut self, data: &[u8]) {
        // copy data over
        self.frame_data.copy_from_slice(data);
    }

    // function for removing alpha pixel data
    // return processed frame data (inside a FrameBuffer?)
    // pub fn post_process_data(&self) -> Vec<u8> {
    #[inline(always)]
    pub fn post_process_data(&mut self) {
        let driver_buffer: &mut [u8] = &mut self.frame_data;
        let chunked_driver_buf: &mut [[u8; 4]] = unsafe { core::slice::from_raw_parts_mut(driver_buffer.as_mut_ptr().cast(), driver_buffer.len() / 4) };
        let processed_buffer: &mut [u8] = &mut self.processed_data;
        // let chunked_processed_buf: &mut [[u8; 3]] = unsafe { core::slice::from_raw_parts_mut(processed_buffer.as_mut_ptr().cast(), processed_buffer.len() / 3) };
        // let chunked_processed_buf: &mut [[u8; 1]] = unsafe { core::slice::from_raw_parts_mut(processed_buffer.as_mut_ptr().cast(), processed_buffer.len()) };
        // Set up references to vectorize the post processing
        chunked_driver_buf
            .iter().copied()
            // .iter() // removing .copied() doesn't change the benchmark
            .zip(processed_buffer)
            // return RGB
            // .for_each(|([a,b,c,_],d)| *d = [*a,*b,*c]);
            // scale and sum to make RGB-> Grayscale
            .for_each(|([a,b,c,_],d)| *d = (( (0.2989*((a) as f32)) + (0.5870*((b) as f32)) + (0.114*((c) as f32))) as u8));
            // .for_each(|([_r,_g,b,_a],e)| *e = b);
    }
}


// ObservationData = Current slice of time's data
// Relation to agent: Agent uses previous ObservationData's frame_data to calculate keycode_data
// As to run a frame we need keycode input. But our initial frame is set to all 0's anyway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationData {
    // I want to OWN this, and supply a reference to the SdlWindow
    pub frame_buffer: FrameBuffer, // contains a vector of pixel data + some other things
    pub keycode_data: u16, // keycodes are u16
}

impl ObservationData {
    #[allow(dead_code)]
    pub fn to_file(&self, file_path: &Path) {
        // flooie
        let mut file = File::create(file_path).expect("Cracka pls");
        // Convert struct into writeable type
        // file.write_all(serde_json::to_string(&self).expect("Y me"));
        write!(file, "{}", serde_json::to_string(&self).expect("Y me"))
            .expect(format!("Failure to write to file {:?}",file).as_str());
    }

    // pub fn from_file(file_path: &Path) {
    //     //
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationSet {
    observations: Vec<ObservationData>,
    width: u32,
    height: u32,
    // serialize deserialize needs to be added
    #[serde(with = "PixelFormatEnumDef")]
    pixel_type: PixelFormatEnum,
}

impl ObservationSet {
    pub fn new(
        data: Option<ObservationData>,
        width: u32,
        height: u32,
        pixel_type: PixelFormatEnum,
    ) -> Self {
        if data.is_some() {
            Self {
                observations: vec![data.unwrap()],
                width,
                height,
                pixel_type,
            }
        } else {
            Self {
                observations: vec![],
                width,
                height,
                pixel_type,
            }
        }
    }

    // pub fn to_file(&self, file_path: &Path) {
    //     // flooie
    //     // let mut file = File::create(file_path).expect("Cracka pls");
    //     // Convert struct into writeable type
    //     // file.write_all(serde_json::to_string(&self).expect("Y me"));
    //     // write!(file,"{}",serde_json::to_string(&self).expect("Y me"));
    // }

    pub fn save_album(&self, album_dir: &Path) {
        // create dir if doesn't exist
        create_dir(album_dir).expect("Going to kms");
        // loop through and write
        // only do the last 120 frames (2 seconds)
        // subtract 120 from length and saturate to 0
        const FPS: usize = 60;
        const TIME_DURATION_BUFFER: usize = 10;
        const MAX_FRAME_DATA_SAVE: usize = TIME_DURATION_BUFFER * FPS;
        let start_idx = self.observations.len().saturating_sub(MAX_FRAME_DATA_SAVE);
        println!("Observation length: {}, start index: {}",self.observations.len(),start_idx);
        for (idx, observation_data) in self.observations[start_idx..].iter().enumerate() {
            // convert observation_data.frame_buffer.frame_data[] into imagebuffer
            // let mut img = image::ImageBuffer::<P, Vec<u8>>::from_raw(self.width,self.height,observation_data.frame_buffer.frame_data);

            // observation_data.frame_buffer.frame_data;//
            let image_dir = album_dir.join(format!("{:0>4}/", idx));
            create_dir(image_dir.clone()).expect("Por que mijo");
            // println!("Trying to write image: {:?}",image_dir);
            let image_path = image_dir.join("img.png");
            let image_data = image_dir.join("img.data");
            // new file name with the album_dir
            // convert self.pixel_type to image::ColorType
            // let color_type: image::ColorType = match self.pixel_type {
            //     PixelFormatEnum::RGB565 => image::ColorType::Rgb16, // 5 + 6 + 5 = 16bit.. not 16 bits per I presume
            //     // PixelFormatEnum::ABGR8888=>image::ColorType::Rgba8,
            //     PixelFormatEnum::ABGR8888 => image::ColorType::Rgb8,
            //     _ => panic!("This is not documented"),
            // };
            // instead of saving buffer, make a frame collection?
            image::save_buffer(
                image_path,
                &observation_data.frame_buffer.processed_data[..],
                self.width,
                self.height,
                // color_type,
                image::ColorType::L8,
            ).expect("Failed to save image");
            // write serialized data
            let mut file = File::create(image_data).expect("couasdpocuasdiopucapdiosu");
            // file.write_all(serde_json::to_string(&self).expect("Y me"));
            #[derive(Serialize)]
            struct JsonStruct<'a> {
                observation_data: &'a ObservationData,
                width: u32,
                height: u32,
                // serialize deserialize needs to be added
                #[serde(with = "PixelFormatEnumDef")]
                pixel_type: PixelFormatEnum,
                pixel_checksum: u32,
            }
            // let pixel_checksum = get_pixel_buffer_checksum(&observation_data.frame_buffer.frame_data[..]);
            let pixel_checksum = get_pixel_buffer_checksum(&observation_data.frame_buffer.frame_data);
            // filter out alpha channel
            // let pixel_checksum = get_pixel_buffer_checksum(&observation_data.frame_buffer.frame_data.iter().position(|x| ((*x)+1)%4).collect());
            let out_data = JsonStruct {
                observation_data: observation_data,
                width: self.width,
                height: self.height,
                pixel_type: self.pixel_type,
                pixel_checksum: pixel_checksum,
            };
            write!(file, "{}", serde_json::to_string(&out_data).expect("Y me"))
                .expect(format!("Failed to write to file {:?}",file).as_str());

            // rgba = image::open(path).unwrap().into_rgba8();
            // gray = DynamicImage::ImageRgba8(rgba).into_luma8(); // grayscale
        }
    }

    pub fn push(&mut self, data: ObservationData) {
        self.observations.push(data);
    }

    // pub fn from_file(file_path: &Path) {
    //     //
    // }

    pub fn len(&self) -> usize {
        self.observations.len()
    }
}

fn get_pixel_buffer_checksum(pixel_buffer: &Vec<u8>) -> u32 {
    // shhh don't worry about why
    let mut ret: u32 = 0;
    for x in pixel_buffer {
        ret += (*x) as u32;
    }
    // get 2's compliment of buffer self sum
    1_u32.overflowing_add(ret ^ u32::MAX).0
}
