use image::{DynamicImage, GrayImage, RgbImage};
use std::sync::{Arc, Mutex};
use std::thread;

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
        if self.states.len() < 100
        {
            for state in &mut self.states {
                if vision::are_the_same(&state.frame, &frame) {
                    state.times_visited += 1;
                    self.number_of_revisited_states += 1;
                    self.last_visited_state = state.clone();
                    return;
                }
            }
        }
        else {
            let result = parallel_linear_search(self.states.clone(), frame.clone());
            if let Some(index) = result {
                let index = index;
                self.states[index].times_visited += 1;
                self.number_of_revisited_states += 1;
                self.last_visited_state = self.states[index].clone();
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

fn parallel_linear_search(data: Vec<State>, target: GrayImage) -> Option<usize> {
    println!("Parallel!");
    let data = Arc::new(data);
    let result = Arc::new(Mutex::new(None));
    let target = Arc::new(target);

    let chunk_size = data.len() / 4;
    let mut handles = vec![];

    for i in 0..4 {
        let data_clone = Arc::clone(&data);
        let result_clone = Arc::clone(&result);
        let target_clone = Arc::clone(&target);
        let handle = thread::spawn(move || {
            let mut local_result = None;
            let chunk = data_clone.chunks(chunk_size).nth(i).unwrap();
            for (index, &ref state) in chunk.iter().enumerate() {
                if vision::are_the_same(&state.frame, &target_clone) {
                    local_result = Some(i * chunk_size + index);
                    break;
                }
            }
            // Lock the mutex to update result
            let mut result = result_clone.lock().unwrap();
            if result.is_none() {
                *result = local_result;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let result = result.lock().unwrap();
    *result
}
