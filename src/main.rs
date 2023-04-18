use rayon::prelude::*;
use rug::ops::CompleteRound;
use rug::{Complex, Float};
use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::surface::Surface;
use std::error::Error;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const TICK_DURATION: Duration = Duration::new(0, 1_000_000_000u32 / 60);

fn mandelbrot(x: Float, y: Float, precision: u32) -> [u8; 3] {
    let mut z = Complex::new(precision);
    let c = Complex::with_val(precision, (x, y));

    let mut color = [0, 0, 0];
    for i in 0..255 {
        z *= z.clone();
        z += c.clone();

        if (z.real() * z.real() + z.imag() * z.imag()).complete(precision) > 4.0 {
            let co = 255 - i;
            color[0] = co as u8;
            color[1] = (co * 2 + 150) as u8;
            color[2] = (co * 3 + 60) as u8;
            break;
        }
    }
    color
}

fn create_image(
    size: u32,
    width: &Float,
    x: &Float,
    y: &Float,
    precision: u32,
) -> (Option<Receiver<Option<u32>>>, Receiver<Vec<u8>>) {
    let width = Float::with_val(precision, width);
    let center = (Float::with_val(precision, x), Float::with_val(precision, y));
    let (progress_tx, progress_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();

    thread::spawn(move || {
        let progress_tx = Arc::new(Mutex::new(progress_tx));

        let image = (0..size)
            .collect::<Vec<_>>()
            .par_iter()
            .map(|&y| {
                let res = (0..size)
                    .map(|x| {
                        let x = center.0.clone() + width.clone() * x / size;
                        let y = center.1.clone() + width.clone() * y / size;
                        mandelbrot(x, y, precision)
                    })
                    .collect::<Vec<_>>();
                let p = progress_tx.lock().unwrap();
                p.send(Some(y)).unwrap();
                res
            })
            .flatten()
            .flatten()
            .collect::<Vec<_>>();
        res_tx.send(image).unwrap();
        let p = progress_tx.lock().unwrap();
        p.send(None).unwrap()
    });

    (Some(progress_rx), res_rx)
}

fn main() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let size = 1000;

    let window = video_subsystem
        .window("Mandelbrot Viewer", size, size + 20)
        .position_centered()
        .build()?;

    let mut canvas = window.into_canvas().build()?;

    canvas.set_draw_color((255, 255, 255));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut positions = vec![];
    for i in 0..(size * size) {
        positions.push(((i % size) as i32, (i / size) as i32));
    }

    let mut selection_start: Option<(i32, i32)> = None;
    let mut selection_end: Option<(i32, i32)> = None;

    let mut precision = 32;
    let mut view_width = Float::with_val(precision, 4.0);
    let mut top_left_x = Float::with_val(precision, -2.0);
    let mut top_left_y = Float::with_val(precision, -2.0);

    let mut current_progress = 0;
    let (mut progress_rx, mut image_rx) =
        create_image(size, &view_width, &top_left_x, &top_left_y, precision);

    let mut image = vec![0; (size * size * 3) as usize];

    let creator = canvas.texture_creator();

    'running: loop {
        canvas.set_draw_color((0, 0, 0));
        canvas.clear();

        let texture = Surface::from_data(
            &mut image,
            size,
            size,
            (size * 3) as u32,
            PixelFormatEnum::RGB24,
        )?
        .as_texture(&creator)?;
        canvas.copy(&texture, None, None)?;

        canvas.set_draw_color((0, 255, 0));
        canvas.fill_rect(Rect::new(0, size as i32, current_progress, 20))?;

        let mouse = event_pump.mouse_state();
        if let Some(start) = selection_start {
            let width = (mouse.x() - start.0).max(mouse.y() - start.1);
            let end = selection_end.unwrap_or((start.0 + width, start.1 + width));

            let r = Rect::new(
                start.0.min(end.0),
                start.1.min(end.1),
                (end.0 - start.0).abs() as u32,
                (end.1 - start.1).abs() as u32,
            );
            canvas.set_draw_color((255, 255, 255));
            canvas.draw_rect(r)?;
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                }
                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    if mouse_btn == MouseButton::Left && progress_rx.is_none() {
                        if selection_start.is_none() {
                            selection_start = Some((x, y))
                        } else if selection_end.is_none() {
                            if let Some(start) = selection_start {
                                let width = (mouse.x() - start.0).max(mouse.y() - start.1);
                                selection_end = Some((start.0 + width, start.1 + width));
                            }
                        } else {
                            if let Some(start) = selection_start {
                                if let Some(end) = selection_end {
                                    selection_start = None;
                                    selection_end = None;

                                    precision += 1;
                                    top_left_x.set_prec(precision);
                                    top_left_y.set_prec(precision);
                                    view_width.set_prec(precision);

                                    top_left_x = top_left_x + view_width.clone() * start.0 / size;
                                    top_left_y = top_left_y + view_width.clone() * start.1 / size;
                                    view_width = view_width * (end.0 - start.0) / size;
                                    (progress_rx, image_rx) = create_image(
                                        size,
                                        &view_width,
                                        &top_left_x,
                                        &top_left_y,
                                        precision,
                                    );
                                }
                            }
                        }
                    } else if mouse_btn == MouseButton::Right {
                        selection_start = None;
                        selection_end = None;
                    }
                }
                _ => {}
            }
        }

        if let Some(rx) = &progress_rx {
            while let Ok(_) = rx.try_recv() {
                current_progress += 1;
            }
            if let Ok(im) = image_rx.try_recv() {
                image = im;
                progress_rx = None;
                current_progress = 0;
            }
        }

        canvas.present();
        thread::sleep(TICK_DURATION);
    }
    Ok(())
}
