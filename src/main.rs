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
        let cells = vec![CellState::Dead; width*height];
        let scratch_cells = vec![CellState::Dead; width*height];
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


fn main() {
    let game = &mut ConwayState::new(8, 8);
    game.next_state();
}
