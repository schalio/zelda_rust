// src/sdl_context.rs

pub struct SdlBundle {
    pub canvas:      sdl2::render::Canvas<sdl2::video::Window>,
    pub event_pump:  sdl2::EventPump,
    pub ttf_context: sdl2::ttf::Sdl2TtfContext,
}

impl SdlBundle {
    pub fn init(title: &str, width: u32, height: u32) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;

        let ttf_context = sdl2::ttf::init()
            .map_err(|e| format!("Erreur init TTF: {e}"))?;

        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(title, width, height)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_context.event_pump()?;

        Ok(Self { canvas, event_pump, ttf_context })
    }
}