use image::{DynamicImage, GrayImage, RgbImage};

use super::vision;

pub struct Agent {
    states: Vec<State>,
    last_visited_state: State,
    number_of_revisited_states: usize,
}

#[derive(Clone)]
struct State {
    frame: GrayImage,
    times_visited: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            frame: GrayImage::default(),
            times_visited: 0,
        }
    }
}

impl Agent {
    pub fn new() -> Self {
        Self {
            states: Vec::<State>::new(),
            last_visited_state: State::default(),
            number_of_revisited_states: 0,
        }
    }

    pub fn visit_state(&mut self, frame: RgbImage) {
        let frame = DynamicImage::ImageRgb8(frame).to_luma8();
        for state in &mut self.states {
            if vision::are_the_same(&state.frame, &frame) {
                state.times_visited += 1;
                self.number_of_revisited_states += 1;
                self.last_visited_state = state.clone();
                return;
            }
        }
        let state = State {
            frame,
            times_visited: 0,
        };
        self.last_visited_state = state.clone();
        self.states.push(state);
    }

    pub fn get_last_state_frame(&self) -> RgbImage {
        let img = self.last_visited_state.frame.clone();
        DynamicImage::ImageLuma8(img).to_rgb8()
    }

    pub fn get_number_of_states(&self) -> usize {
        self.states.len()
    }

    pub fn get_number_of_revisited_states(&self) -> usize {
        self.number_of_revisited_states
    }
}

fn parallel_linear_search(data: &[State], target: GrayImage) -> {

}
