use std::thread;
use std::time::Duration;
use ocl::ProQue;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::surface::Surface;

const SIZE: usize = 2000;
const TICK_DURATION: Duration = Duration::new(0, 1_000_000_000u32 / 60);

fn create_pro_que() -> ProQue {
    let src = r#"
        #define VIEWPORT 4.0

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
                unsigned int size,
                unsigned int mouse_x,
                unsigned int mouse_y,
                float scale,
                int iteration_num
        ) {
            int id = get_global_id(0);
            if (id % 4 != 0) return;

            int index = id / 4;

            float pos_x = ((mouse_x - (float) size / 2) / (float) size) * 4.0 * scale;
            float pos_y = ((mouse_y - (float) size / 2) / (float) size) * 4.0 * scale;
            float min_x = ((float) pos_x - 2) / scale;
            float max_x = ((float) pos_x + 2) / scale;
            float min_y = ((float) pos_y - 2) / scale;
            float max_y = ((float) pos_y + 2) / scale;

            float dif_x = max_x - min_x;
            float dif_y = max_y - min_y;

            float m_x = dif_x / size;
            float m_y = dif_y / size;

            float x = m_x * (index % size) + min_x;
            float y = m_y * (index / size) + min_y;

            Complex z = { 0.0f, 0.0f };
            Complex c = { x, y };

            int r_color = 0x00;
            int g_color = 0x00;
            int b_color = 0x00;

            for (int i = 0; i < iteration_num; i++) {
                complex_mul(&z, &z);
                complex_add(&z, &c);

                if (z.real * z.real + z.imag * z.imag > 4.0) {
                    int color = 255/100 * (i * 10);
                    color = 255 - color;
                    r_color = color;
                    g_color = color;
                    b_color = color;
                    break;
                }
            }

            buffer[id] = r_color;
            buffer[id + 1] = g_color;
            buffer[id + 2] = b_color;
            buffer[id + 3] = 0xff;
        }
    "#;
    ProQue::builder()
        .src(src)
        .dims(SIZE * SIZE * 4)
        .build()
        .expect("Error building the kernel")
}

fn calculate_values(pro_que: &ProQue, mouse_x: i32, mouse_y: i32, scale: f32) -> Vec<u8> {
    let buffer = pro_que.create_buffer::<u8>().unwrap();

    let kernel = pro_que.kernel_builder("mandelbrot")
        .arg(&buffer)
        .arg(SIZE as u32)
        .arg(mouse_x)
        .arg(mouse_y)
        .arg(scale)
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

    let window = video_subsystem.window("rust-sdl2 demo", SIZE as u32, SIZE as u32)
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

    let mut offset_x = 0;
    let mut offset_y = 0;

    'running: loop {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();

        let mouse_state = event_pump.mouse_state();
        let values = calculate_values(&pro_que, mouse_state.x(), mouse_state.y(), scale);
        let mut values = values;
        let creator = canvas.texture_creator();
        let texture = Surface::from_data(
            values.as_mut(),
            SIZE as u32,
            SIZE as u32,
            (SIZE * 4) as u32,
            PixelFormatEnum::RGBA32,
        ).unwrap()
            .as_texture(&creator)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();

        scale *= 1.01;

        for event in event_pump.poll_iter().into_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                }
                _ => {}
            }
        }

        canvas.present();
        thread::sleep(TICK_DURATION);
    }
}