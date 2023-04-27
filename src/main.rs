use ocl::prm::Uchar4;
use ocl::ProQue;
use ocl::SpatialDims::Two;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::surface::Surface;
use std::thread;
use std::time::{Duration, Instant};

const SIZE: usize = 2000;
const TICK_DURATION: Duration = Duration::new(0, 1_000_000_000u32 / 60);

fn create_pro_que() -> ProQue {
    ProQue::builder()
        .src(include_str!("kernel.cl"))
        .dims(SIZE * SIZE * 4)
        .build()
        .expect("Error building the kernel.c")
}

fn calculate_values(pro_que: &mut ProQue, pos_x: f64, pos_y: f64, width: f64) -> Vec<u8> {
    let buffer = pro_que.create_buffer::<Uchar4>().unwrap();
    pro_que.set_dims(Two(SIZE, SIZE));

    let kernel = pro_que
        .kernel_builder("mandelbrot")
        .arg(&buffer)
        .arg(SIZE as u32)
        .arg(pos_x)
        .arg(pos_y)
        .arg(width)
        .arg(255)
        .build()
        .unwrap();

    unsafe {
        kernel.enq().unwrap();
    }

    let i = Instant::now();
    let mut result = vec![Uchar4::zero(); buffer.len()];
    buffer.read(&mut result).enq().unwrap();
    let res: Vec<u8> = result.iter().flat_map(|it| it.to_vec()).collect();
    println!("{:?}", i.elapsed());
    res
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Mandelbrot", SIZE as u32, SIZE as u32)
        .position_centered()
        .build()
        .expect("could not initialize video subsystem");

    let mut canvas = window
        .into_canvas()
        .build()
        .expect("could not make a canvas");

    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut pro_que = create_pro_que();

    let mut center_x = 0.0;
    let mut center_y = 0.0;
    let mut width = 4.0;

    'running: loop {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();

        let values = calculate_values(&mut pro_que, center_x, center_y, width);
        let mut values = values;
        let creator = canvas.texture_creator();
        let texture = Surface::from_data(
            values.as_mut(),
            SIZE as u32,
            SIZE as u32,
            (SIZE * 4) as u32,
            PixelFormatEnum::RGBA32,
        )
        .unwrap()
        .as_texture(&creator)
        .unwrap();
        canvas.copy(&texture, None, None).unwrap();

        let mut mouse_motion = (0, 0);
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::MouseWheel { y, .. } => {
                    if y == 1 {
                        width *= 1.05;
                    } else if y == -1 {
                        width *= 0.95;
                    }
                }
                Event::MouseMotion { xrel, yrel, .. } => {
                    mouse_motion.0 += xrel;
                    mouse_motion.1 += yrel;
                }
                _ => {}
            }
        }

        let mouse_state = event_pump.mouse_state();

        if mouse_state.is_mouse_button_pressed(MouseButton::Left) {
            center_x -= width * (mouse_motion.1 as f64) / (SIZE as f64);
            center_y -= width * (mouse_motion.0 as f64) / (SIZE as f64);
        }

        canvas.present();
        thread::sleep(TICK_DURATION);
    }
}
