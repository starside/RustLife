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


#[derive(PartialEq, Eq, Clone, Copy)]
enum CellState {
    Dead,
    Alive
}

struct ConwayState {
    cells: Vec<CellState>,
    scratch_cells: Vec<CellState>,
    width: usize,
    height: usize
}

impl ConwayState {
    pub fn new(width: usize, height: usize) -> Self {
        let mut cells = vec![CellState::Dead; width*height];
        let scratch_cells = vec![CellState::Dead; width*height];
        for (i,c) in cells.iter_mut().enumerate() {
            if rand::random::<bool>() {
                *c = CellState::Alive;
            }
        }
        ConwayState {cells, scratch_cells, width, height}
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
    
    fn next_cell_state(&mut self,  x: usize, y:usize) {
        let linear_id = (y as usize)*self.width + (x as usize);
        let cell_state = &self.cells[linear_id];
        let live_count = self.count_alive_neighbors(x, y);
        let ns = match (cell_state, live_count) {
            (CellState::Dead, 3) => CellState::Alive,
            (CellState::Alive, 2 | 3) => CellState::Alive,
            _ => CellState::Dead
        };
        self.scratch_cells[linear_id] = ns;
    }
    
    pub fn next_state(&mut self) {
        for i in 0..self.width {
            for j in 0..self.height {
                self.next_cell_state(i, j);
            }
        }
        std::mem::swap(&mut self.cells, &mut self.scratch_cells);
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

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1268;

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 3.0, HEIGHT as f64 * 3.0);
        WindowBuilder::new()
            .with_title("Conway's Game of Life")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut life = ConwayState::new((WIDTH/2) as usize, (HEIGHT/2) as usize);
    let mut paused = false;

    let mut draw_state: Option<bool> = None;
    let mut now = std::time::Instant::now();
    let mut frames: f64 = 0.0;

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            //life.draw(pixels.frame_mut());
            draw(WIDTH, HEIGHT, pixels.frame_mut(), &life);
            life.next_state();
            frames += 1.0;
            let duration = now.elapsed().as_micros() as f64;
            if(duration >= 1_000_000.0) {
                println!("FPS: {}", 1_000_000.0*(frames/duration) );
                frames = 0.0;
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
