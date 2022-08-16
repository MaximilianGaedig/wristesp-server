#[macro_use]
extern crate rocket;
extern crate font8x8;
use bitvec::{macros::internal::funty::Fundamental, prelude::*};
use chrono::{DateTime, Utc};
use font8x8::{UnicodeFonts, BASIC_FONTS};
use rocket::State;
use std::{fs, sync::atomic::AtomicUsize, time::Duration, time::Instant};

struct HitCount {
    count: AtomicUsize,
    last_frame_time: AtomicUsize,
}

const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 64;

fn string_to_matrix(text: &str) -> Vec<Vec<bool>> {
    let mut string_matrix = vec![vec![false; 8]; text.chars().count() * 8];
    for (offset, c) in text.chars().enumerate() {
        if let Some(glyph) = BASIC_FONTS.get(c) {
            let mut line_index = 0;
            for line in glyph {
                for pixel_index in 0..8 {
                    if line & 1 << pixel_index != 0 {
                        string_matrix[offset * 8 + pixel_index][line_index] = true;
                    }
                }
                line_index += 1;
            }
        }
    }
    string_matrix
}

/// * `(x, y)` - offset
fn write(
    &mut mut screen_buffer: &mut [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH],
    text: &str,
    (x, y): (usize, usize),
    color: bool,
) -> [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH] {
    // TODO: text wrapping
    // TODO: text alignment
    let text_string = text.to_string();

    let mut matrixes: Vec<Vec<Vec<bool>>> = Vec::new();

    for (_, chunk) in text_string
        .chars()
        .collect::<Vec<char>>()
        .chunks((SCREEN_WIDTH - x) / 8)
        .enumerate()
    {
        matrixes.push(string_to_matrix(&*chunk.iter().collect::<String>()));
    }

    for (line, matrix) in matrixes.iter().enumerate() {
        for (i, col) in matrix.iter().enumerate() {
            for (j, val) in col.iter().enumerate() {
                let screen_x = i + x;
                let screen_y = j + y + (line * 8);
                if screen_x < SCREEN_WIDTH && screen_y < SCREEN_HEIGHT {
                    if *val {
                        screen_buffer[screen_x][screen_y] = color;
                    }
                }
            }
        }
    }
    screen_buffer
}

fn write_img_in_weird_encoding(
    &mut mut screen_buffer: &mut [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH],
    file: Vec<u8>,
    (x, y): (usize, usize),
    color: bool,
    (resx, resy): (usize, usize),
) -> [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH] {
    for cy in 0..resy {
        let bit = cy % 8;
        for cx in 0..resx {
            let screen_x = x + cx;
            let screen_y = y + cy;
            if screen_x < SCREEN_WIDTH && screen_y < SCREEN_HEIGHT {
                if !((file[cx + (cy / 8) * resx] & (1 << bit)) > 0) {
                    screen_buffer[screen_x][screen_y] = color;
                }
            }
        }
    }
    screen_buffer
}

fn write_img(
    &mut mut screen_buffer: &mut [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH],
    file: Vec<u8>,
    (x, y): (usize, usize),
    invert: bool,
    (resx, resy): (usize, usize),
) -> [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH] {
    let bits: BitVec<u8, Msb0> = BitVec::from_vec(file);
    for cy in 0..resy {
        for cx in 0..resx {
            let screen_x = x + cx;
            let screen_y = y + cy;
            let val = bits[cx + cy * resx];
            if screen_x < SCREEN_WIDTH && screen_y < SCREEN_HEIGHT {
                if invert {
                    screen_buffer[screen_x][screen_y] = !val;
                } else {
                    screen_buffer[screen_x][screen_y] = val;
                }
            }
        }
    }
    screen_buffer
}

fn draw_rectangle(
    &mut mut screen_buffer: &mut [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH],
    (x, y): (usize, usize),
    (resx, resy): (usize, usize),
    color: bool,
) -> [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH] {
    for cy in 0..resy {
        for cx in 0..resx {
            let screen_x = x + cx;
            let screen_y = y + cy;
            if screen_x < SCREEN_WIDTH && screen_y < SCREEN_HEIGHT {
                screen_buffer[screen_x][screen_y] = color;
            }
        }
    }
    screen_buffer
}

fn draw_pixel(
    &mut mut screen_buffer: &mut [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH],
    (x, y): (usize, usize),
    color: bool,
) -> [[bool; SCREEN_HEIGHT]; SCREEN_WIDTH] {
    screen_buffer[x][y] = color;
    screen_buffer
}

fn trim_bytes(x: BitVec<u8, Msb0>) -> BitVec<u8, Msb0> {
    let to = x.iter().rposition(|x| *x).unwrap();
    x[0..=to].into()
}

#[get("/update-screen?<b>")]
fn update(b: Option<&str>, hit_count: &State<HitCount>) -> Vec<u8> {
    // let current_millis = Instant::now().elapsed().as_millis();
    // println!("{}", current_millis);
    // let time_passed = current_millis
    //     - hit_count
    //         .last_frame_time
    //         .load(std::sync::atomic::Ordering::Relaxed) as u128;

    // if time_passed as usize > 5000 {
    //     hit_count
    //         .last_frame_time
    //         .store(0, std::sync::atomic::Ordering::Relaxed);
    // };

    hit_count
        .count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let current_count = hit_count.count.load(std::sync::atomic::Ordering::Relaxed);
    let mut screen_content: BitVec<u8, Msb0> = BitVec::new();
    let mut screen_buffer = [[false; SCREEN_HEIGHT]; SCREEN_WIDTH];
    // screen_buffer = write(&mut screen_buffer, "Request count:", (0, 0), true);
    let frame = (current_count as f32 * 3.5).round() as usize;
    // let frame = current_count;
    screen_buffer = write(&mut screen_buffer, frame.to_string().as_str(), (0, 0), true);

    if let Some(b) = b {
        let b_int = b.parse::<u8>();
        if let Ok(b_int) = b_int {
            let b_bits: BitVec<u8, Lsb0> = BitVec::from_slice(&[b_int]);
            for (i, bit) in b_bits.iter().enumerate() {
                screen_buffer = draw_pixel(&mut screen_buffer, (i, 8), !bit.as_bool());
            }
        }
    }

    let width = 3;
    let file_name = format!(
        "video_processing/monochrome/out-{:0width$}.jpg.gray",
        frame,
        width = width
    );
    println!("{}", file_name);
    let file = fs::read(&file_name);
    if let Ok(file) = file {
        screen_buffer = write_img(&mut screen_buffer, file, (40, 0), false, (88, 64));
    } else {
        // hit_count
        //     .count
        //     .store(0, std::sync::atomic::Ordering::Relaxed);
    }
    // let now: DateTime<Utc> = Utc::now();
    // let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    // screen_buffer = write(&mut screen_buffer, now_str.as_str(), (0, 22), true);

    // screen_buffer = draw_rectangle(&mut screen_buffer, (85 + 22, 0), (4, 64), false);
    for (_, col) in screen_buffer.iter().enumerate() {
        for (_, val) in col.iter().enumerate() {
            screen_content.push(*val);
        }
    }

    let mut flipped_screen_buffer = [[false; SCREEN_WIDTH]; SCREEN_HEIGHT];

    for (i, col) in screen_buffer.iter().enumerate() {
        for (j, val) in col.iter().enumerate() {
            flipped_screen_buffer[j][i] = *val;
        }
    }

    for (_, col) in flipped_screen_buffer.iter().enumerate() {
        for (_, val) in col.iter().enumerate() {
            if *val {
                print!("█");
            } else {
                print!(" ");
            }
        }
        print!("\n");
    }
    screen_content = trim_bytes(screen_content);
    screen_content.into_vec()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(HitCount {
            count: AtomicUsize::new(0),
            last_frame_time: AtomicUsize::new(
                0, // Instant::now().elapsed().as_millis().try_into().unwrap(),
            ),
        })
        .mount("/", routes![update])
}
