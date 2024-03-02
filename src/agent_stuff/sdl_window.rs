// I got this file and edited it from the embedded-graphics-simulator crates

use sdl2::{
    event::Event,
    keyboard::Keycode,
    render::{Canvas, Texture, TextureCreator},
    video::WindowContext,
    EventPump,
};

use super::observation::*;

pub struct SdlWindow {
    canvas: Canvas<sdl2::video::Window>,
    event_pump: EventPump,
    window_texture: SdlWindowTexture,
    // frame_buffer: &'a FrameBuffer,
    // frame_buffer: &'a ObservationData,
    pub observation_data: ObservationData,
    cur_key_code: u16,
}

impl SdlWindow {
    // pub fn new<C>(
    pub fn new(
        // display: &SimulatorDisplay<C>,
        title: &str,
        // output_settings: &OutputSettings,
        // width: u32,
        // height: u32,
        // bpp: u32,
        // frame_buffer: &FrameBuffer,

        // Copies the data over, or moves it in!
        observation_data: ObservationData,
    ) -> Self
// where
    //     C: PixelColor + Into<Rgb888>,
    {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window(
                title,
                observation_data.frame_buffer.width,
                observation_data.frame_buffer.height
            )
            .position_centered()
            .build()
            .unwrap();

        let canvas = window.into_canvas().build().unwrap();
        let event_pump = sdl_context.event_pump().unwrap();

        // let frame_buffer = FrameBuffer::new(width, height, bpp, pixel_format);

        let window_texture = SdlWindowTextureBuilder {
            texture_creator: canvas.texture_creator(),
            texture_builder: |creator: &TextureCreator<WindowContext>| {
                creator
                    .create_texture_streaming(
                        observation_data.frame_buffer.pixel_format,
                        observation_data.frame_buffer.width,
                        observation_data.frame_buffer.height
                    )
                    .unwrap()
            },
        }
        .build();
        Self {
            canvas,
            event_pump,
            window_texture,
            // frame_buffer: frame_buffer,
            observation_data:observation_data,
            cur_key_code: 0x0000,
        }
    }

    // pub fn update(&mut self, framebuffer: &OutputImage<Rgb888>) {
    //     self.window_texture.with_mut(|fields| {
    //         fields
    //             .texture
    //             .update(
    //                 None,
    //                 framebuffer.data.as_ref(),
    //                 self.size.width as usize * 3,
    //             )
    //             .unwrap();
    //     });

    //     self.canvas
    //         .copy(&self.window_texture.borrow_texture(), None, None)
    //         .unwrap();
    //     self.canvas.present();
    // }

    // pub fn update(&mut self, framebuffer: &[u8]) {
    pub fn update(&mut self) {
        // For now, we're mutating the framebuffer directly
        // let mut pixels = vec![0_u8; screen_buffer_size as usize];
        self.window_texture.with_mut(|fields| {
            fields
                .texture
                .with_lock(None, |wonly_tex_frame_buffer: &mut [u8], _pitch| {
                    wonly_tex_frame_buffer.copy_from_slice(self.observation_data.frame_buffer.get_buffer());
                })
                .unwrap();
        });

        self.canvas
            .copy(&self.window_texture.borrow_texture(), None, None)
            .unwrap();
        self.canvas.present();

        // store the framebuffer to our struct?
    }

    pub fn matt_events(&mut self) -> Option<u16> {
        // won't get an event every time we get here, so we need to keep track of what it was set to
        // on a previous frame
        for event in self.event_pump.poll_iter() {
            // println!("Event detected");
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    // running = false;
                    return None;
                }
                Event::KeyDown {
                    keycode: Some(_keycode),
                    // repeat: bool
                    ..
                } => {
                    // println!("Keycode down? {}",_keycode);
                    // |=
                    match _keycode {
                        Keycode::A => self.cur_key_code |= 0x200_u16, // 1<<9 , 512
                        Keycode::S => self.cur_key_code |= 0x100_u16, // 1<<8 , 256
                        Keycode::X => self.cur_key_code |= 0x1_u16,   // 1<<0 , 1
                        Keycode::Z => self.cur_key_code |= 0x2_u16,   // 1<<1 , 2
                        Keycode::Backspace => self.cur_key_code |= 0x4_u16, // 1<<2 , 4
                        Keycode::Return => self.cur_key_code |= 0x8_u16, // 1<<3 , 8
                        Keycode::Right => self.cur_key_code |= 0x10_u16, // 1<<4 , 16
                        Keycode::Left => self.cur_key_code |= 0x20_u16, // 1<<5 , 32
                        Keycode::Up => self.cur_key_code |= 0x40_u16, // 1<<6 , 64
                        Keycode::Down => self.cur_key_code |= 0x80_u16, // 1<<7 , 128
                        _ => (),
                    }
                }
                // &=
                Event::KeyUp {
                    keycode: Some(_keycode),
                    ..
                } => {
                    match _keycode {
                        Keycode::A => self.cur_key_code &= !0x200_u16, // 1<<9 , 512
                        Keycode::S => self.cur_key_code &= !0x100_u16, // 1<<8 , 256
                        Keycode::X => self.cur_key_code &= !0x1_u16,   // 1<<0 , 1
                        Keycode::Z => self.cur_key_code &= !0x2_u16,   // 1<<1 , 2
                        Keycode::Backspace => self.cur_key_code &= !0x4_u16, // 1<<2 , 4
                        Keycode::Return => self.cur_key_code &= !0x8_u16, // 1<<3 , 8
                        Keycode::Right => self.cur_key_code &= !0x10_u16, // 1<<4 , 16
                        Keycode::Left => self.cur_key_code &= !0x20_u16, // 1<<5 , 32
                        Keycode::Up => self.cur_key_code &= !0x40_u16, // 1<<6 , 64
                        Keycode::Down => self.cur_key_code &= !0x80_u16, // 1<<7 , 128
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        // filter out specific keys
        if (self.cur_key_code & 0x30) == (0x30) {
            Some(self.cur_key_code & (!0x30))
        } else if (self.cur_key_code & 0xc0) == (0xc0) {
            Some(self.cur_key_code & (!0xc0))
        } else {
            Some(self.cur_key_code)
        }
        // output_keycode
    }
}

#[ouroboros::self_referencing]
struct SdlWindowTexture {
    texture_creator: TextureCreator<WindowContext>,
    #[borrows(texture_creator)]
    #[covariant]
    texture: Texture<'this>,
}
