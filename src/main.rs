use rand::prelude::*;
use error_iter::ErrorIter as _;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;
use std::sync::{Arc, RwLock};
use std::thread;
use std::sync::atomic::{AtomicI32, Ordering};
use rayon::prelude::*;


#[derive(PartialEq, Eq, Clone, Copy)]
enum CellState {
    Dead,
    Alive
}

struct ConwayState {
    cells: Vec<CellState>,
    width: usize,
    height: usize
}

impl ConwayState {
    pub fn new(width: usize, height: usize) -> Self {
        let mut cells = vec![CellState::Dead; width*height];
        for (i,c) in cells.iter_mut().enumerate() {
            if rand::random::<bool>() {
                *c = CellState::Alive;
            }
        }
        ConwayState {cells, width, height}
    }

    fn count_alive_neighbors(&self, x: usize, y:usize) -> usize {
        const NEIGHBORS: [(i32, i32); 8] = [ // y, x or row, column
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1)
        ];
        let mut count = 0;
        let x = x as i32;
        let y = y as i32;
        // Boundary conidition is dead cells
        for (j, i) in NEIGHBORS {
            if y + j >= 0 &&
               x + i >= 0 &&
               y + j < self.height as i32 &&
               x + i < self.width as i32 {
                let linear_id = ((y + j) as usize)*self.width + ((x + i) as usize);
                    if self.cells[linear_id] == CellState::Alive {
                        count += 1;
                    }
            }
        }
        count
    }
    
    fn next_cell_state(&self, x: usize, y:usize) -> CellState{
        let linear_id = (y as usize)*self.width + (x as usize);
        let cell_state = &self.cells[linear_id];
        let live_count = self.count_alive_neighbors(x, y);
        let ns = match (cell_state, live_count) {
            (CellState::Dead, 3) => CellState::Alive,
            (CellState::Alive, 2 | 3) => CellState::Alive,
            _ => CellState::Dead
        };
        ns
    }
    
    pub fn next_state(&self, scratch: &mut ConwayState) {
        const rows_in_chunk:usize = 2;
        let elements_in_chunk = rows_in_chunk * self.width;
        let num_chunks = self.cells.len() / elements_in_chunk;
        let rows_in_last_chunk = (self.cells.len() - num_chunks * elements_in_chunk) / self.width;

        scratch.cells.par_chunks_mut(elements_in_chunk).enumerate().map(|(chunk, cells)| {
            let row = chunk * rows_in_chunk;
            if chunk < num_chunks
            {
                for j in 0..rows_in_chunk  {
                    for i in 0..self.width {
                        cells[j*self.width + i] = self.next_cell_state(i, j + row);
                    }
                }
            }
            else {
                for j in 0..rows_in_last_chunk
                {
                    for i in 0..self.width{
                        cells[j*self.width + i] = self.next_cell_state(i, j + row);
                    }
                }
            }
        }).count();
    }

    pub fn swap_state(&mut self, scratch: &mut ConwayState) {
        std::mem::swap(&mut self.cells, &mut scratch.cells);
    }
}

fn draw(width: u32, height: u32, screen: &mut [u8], state: &ConwayState) {
    let width_f = (width) as f64;
    let height_f = (height) as f64;

    let state_width: f64 = state.width as f64;
    let state_height: f64 = state.height as f64;

    for (i, pix) in screen.chunks_exact_mut(4).enumerate() {
        let y = (i as u32 / width) as f64 / height_f;
        let x = (i as u32 % width) as f64 / width_f;
        let x_border = x * state_width;
        let y_border = y * state_height;

       {
            let x_id = x_border.floor() as usize;
            let y_id = y_border.floor() as usize;
            let linear_id = y_id * state.width + x_id;
            match state.cells[linear_id] {
                CellState::Alive => {
                    let color = [0xff, 0xff, 0xff, 0xff];
                    pix.copy_from_slice(&color);
                },
                CellState::Dead => {
                    let color = [0x0, 0x00, 0x00, 0xff];
                    pix.copy_from_slice(&color);
                }
            }

        }
    }
}

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;

const GAME_WIDTH: u32 = 2048;
const GAME_HEIGHT: u32 = 2048;

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Conway's Game of Life")
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut life = Arc::new(RwLock::new(ConwayState::new(GAME_WIDTH as usize, GAME_HEIGHT as usize)));
    let c_life = Arc::clone(&life);

    let mut paused = false;

    let mut draw_state: Option<bool> = None;
    let mut now = std::time::Instant::now();

    let frames = Arc::new(AtomicI32::new(0));
    let c_frames = Arc::clone(&frames);

    thread::spawn(move || {
        let mut scratch = ConwayState::new(GAME_WIDTH as usize, GAME_HEIGHT as usize);

        loop {
            if let Ok(l) = c_life.read() {
                l.next_state(&mut scratch);
            }
            if let Ok(mut l) = c_life.write() {
                l.swap_state(&mut scratch);
            }
            c_frames.fetch_add(1, Ordering::Relaxed);
        }
    });

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            //life.draw(pixels.frame_mut());
            if let Ok(life) = life.read()
            {
                draw(WIDTH, HEIGHT, pixels.frame_mut(), &life);
            }

            let duration = now.elapsed().as_micros() as f64;
            if(duration >= 1_000_000.0) {
                println!("FPS: {}", 1_000_000.0*((frames.load(Ordering::Relaxed) as f64)/duration) );
                frames.store(0, Ordering::SeqCst);
                now = std::time::Instant::now();
            }

            //panic!("ENd");
            if let Err(err) = pixels.render() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if input.key_pressed(VirtualKeyCode::P) {
                paused = !paused;
            }
            if input.key_pressed_os(VirtualKeyCode::Space) {
                // Space is frame-step, so ensure we're paused
                paused = true;
            }
            if input.key_pressed(VirtualKeyCode::R) {
                //life.randomize();
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            if !paused || input.key_pressed_os(VirtualKeyCode::Space) {
                //life.update();
            }
            window.request_redraw();
        }
    });
}
