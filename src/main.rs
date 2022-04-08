use std::thread;
use std::time::Duration;
use ocl::ProQue;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::surface::Surface;

const SIZE: usize = 2000;
const TICK_DURATION: Duration = Duration::new(0, 1_000_000_000u32 / 60);

fn create_pro_que() -> ProQue {
    let src = r#"
        typedef struct {
            float real;
            float imag;
        } Complex;

        inline void complex_add(Complex* c1, Complex* c2) {
            c1->real += c2->real;
            c1->imag += c2->imag;
        }

        inline void complex_mul(Complex* c1, Complex* c2) {
            float new_real = c1->real * c2->real - c1->imag * c2->imag;
            float new_imag = c1->real * c2->imag + c1->imag * c2->real;
            c1->real = new_real;
            c1->imag = new_imag;
        }

        __kernel void mandelbrot(
                __global char* buffer,
                int size,
                float center_x,
                float center_y,
                float width,
                int iteration_num
        ) {
            int id = get_global_id(0);
            int index = id / 3;

            float pixel_x = index / size;
            float pixel_y = index % size;

            float x = center_x + width * (pixel_x / size - 0.5);
            float y = center_y + width * (pixel_y / size - 0.5);

            Complex z = { 0.0f, 0.0f };
            Complex c = { x, y };

            int r_color = 0x00;
            int g_color = 0x00;
            int b_color = 0x00;

            for (int i = 0; i < iteration_num; i++) {
                complex_mul(&z, &z);
                complex_add(&z, &c);

                if (z.real * z.real + z.imag * z.imag > 4.0) {
                    int color = i; //255/100 * (i * 10);
                    color = 255 - color;
                    r_color = color;
                    g_color = (color * 2 + 150) % 256;
                    b_color = (color * 3 + 60) % 256;
                    break;
                }
            }

            if (id % 3 == 0) {
                buffer[id] = r_color;
            } else if (id % 3 == 1) {
                buffer[id] = g_color;
            } else {
                buffer[id] = b_color;
            }
        }
    "#;
    ProQue::builder()
        .src(src)
        .dims(SIZE * SIZE * 3)
        .build()
        .expect("Error building the kernel")
}

fn calculate_values(pro_que: &ProQue, pos_x: f32, pos_y: f32, width: f32) -> Vec<u8> {
    let buffer = pro_que.create_buffer::<u8>().unwrap();

    let kernel = pro_que.kernel_builder("mandelbrot")
        .arg(&buffer)
        .arg(SIZE as u32)
        .arg(pos_x)
        .arg(pos_y)
        .arg(width)
        .arg(1000)
        .build()
        .unwrap();

    unsafe { kernel.enq().unwrap(); }

    let mut result = vec![0; buffer.len()];
    buffer.read(&mut result).enq().unwrap();
    result
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Mandelbrot", SIZE as u32, SIZE as u32)
        .position_centered()
        .build()
        .expect("could not initialize video subsystem");

    let mut canvas = window.into_canvas()
        .build()
        .expect("could not make a canvas");

    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut scale = 1.0;

    let pro_que = create_pro_que();

    let mut positions = vec![];
    for i in 0..(SIZE * SIZE) {
        positions.push(((i % SIZE) as i32, (i / SIZE) as i32));
    }

    let mut center_x = 0.0f32;
    let mut center_y = 0.0f32;
    let mut width = 4.0f32;

    'running: loop {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();

        let values = calculate_values(&pro_que, center_x, center_y, width);
        let mut values = values;
        let creator = canvas.texture_creator();
        let texture = Surface::from_data(
            values.as_mut(),
            SIZE as u32,
            SIZE as u32,
            (SIZE * 3) as u32,
            PixelFormatEnum::RGB24,
        ).unwrap()
            .as_texture(&creator)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();

        scale *= 1.01;

        let mut mouse_motion = None;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
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
                    mouse_motion = Some((xrel, yrel));
                }
                _ => {}
            }
        }

        let mouse_state = event_pump.mouse_state();

        if mouse_state.is_mouse_button_pressed(MouseButton::Left) {
            if let Some((dy, dx)) = mouse_motion {
                center_x -= 10.0 * width * (dx as f32) / (SIZE as f32);
                center_y -= 10.0 * width * (dy as f32) / (SIZE as f32);
            }
        }

        canvas.present();
        thread::sleep(TICK_DURATION);
    }
}