use image::RgbImage;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::thread;
use image::io::Reader;

use super::vision;

pub struct Agent {
    states: Vec<State>,
    previous_index: Option<usize>,
    previous_action: Option<u8>,
    previous_q: Option<f32>,
    number_of_revisited_states: usize,
    discount_factor: f32,
    learning_rate: f32,
    hist_threshold: u32,
    blur: f32,
    median_filter: u32,
    max_mse: f32,
    character1_histogram: Option<Vec<Vec<Vec<u32>>>>,
}

#[derive(Clone)]
struct State {
    frame_abstraction: RgbImage,
    q: [f32; 256],
}

impl State {
    fn new(frame_abstraction: RgbImage) -> Self {
        Self {
            frame_abstraction,
            q: [0.0; 256],
        }
    }
}

impl Agent {
    pub fn new() -> Self {
        let img = Reader::open("characters/yoshimitsu.png").unwrap().decode().unwrap();
        let img = img.to_rgb8();
        Self {
            states: Vec::<State>::new(),
            previous_index: None,
            previous_action: None,
            previous_q: None,
            number_of_revisited_states: 0,
            discount_factor: 0.9,
            learning_rate: 0.5,
            hist_threshold: 85,
            blur: 1.0,
            median_filter: 3,
            max_mse: 0.03,
            character1_histogram: Some(vision::get_histogram(&img)),
        }
    }

    pub fn visit_state(&mut self, frame: RgbImage, reward: f32) -> u8 {
        // We need a way to recognize equivalent states
        // This is one of the most important/challenging parts
        let frame_abstraction = vision::get_frame_abstraction(
            &frame,
            self.character1_histogram.clone().as_ref().unwrap(),
            self.hist_threshold,
            self.blur,
            self.median_filter,
        );

        // Search or Add
        let current_index: usize;
        let current_action: u8;
        let max_q: f32;
        //if let Some(index) = self.search_state(&frame_abstraction) {
        if let Some(index) =
            parallel_linear_search(self.states.clone(), frame_abstraction.clone(), self.max_mse)
        {
            // Existing state
            let current_state = &self.states[index];
            (current_action, max_q) = choose_best_action(current_state);
            self.number_of_revisited_states += 1;
            current_index = index;
        } else {
            // New state
            current_index = self.states.len();
            self.states.push(State::new(frame_abstraction));
            let mut rng = rand::thread_rng();
            current_action = rng.gen_range(0..=255);
            max_q = 0.0;
        }

        // Heart of Q-Learning
        if let Some(previous_index) = self.previous_index {
            //let reward = (reward + 1.0) / 2.0;
            //print!("Reward: {}\t", reward);
            let previous_state = &mut self.states[previous_index];
            let act = self.previous_action.unwrap() as usize;
            //print!("Max Q: {}\t", max_q);
            let temporal_difference = reward + self.discount_factor * max_q - previous_state.q[act];
            //print!("Previous: {}\t", previous_state.q[act]);
            previous_state.q[act] =
                previous_state.q[act] + self.learning_rate * temporal_difference;
            //println!("Next: {}", previous_state.q[act]);
        }

        self.previous_index = Some(current_index);
        self.previous_action = Some(current_action);
        self.previous_q = Some(max_q);

        current_action
    }

    #[allow(dead_code)]
    fn search_state(&self, target: &RgbImage) -> Option<usize> {
        let mut min_mse: f32 = (1 << 16) as f32;
        let mut best_index = 0;
        for (index, state) in &mut self.states.iter().enumerate() {
            let mse = vision::get_mse(&state.frame_abstraction, &target);
            // get_color_mse_foreground could be a good idea
            // Or be stricter with max_mse
            if mse < min_mse {
                min_mse = mse;
                best_index = index;
            }
        }
        if min_mse < self.max_mse {
            return Some(best_index);
        }
        None
    }

    pub fn get_last_state_abstraction(&self) -> RgbImage {
        if let (Some(index), Some(q)) = (self.previous_index, self.previous_q) {
            let mut frame = self.states[index].frame_abstraction.clone();
            vision::enclose_with_q(&mut frame, q);
            return frame;
        }
        RgbImage::default()
    }

    pub fn get_number_of_states(&self) -> usize {
        self.states.len()
    }

    pub fn get_number_of_revisited_states(&self) -> usize {
        self.number_of_revisited_states
    }

    pub fn set_hist_threshold(&mut self, val: u32) {
        self.hist_threshold = val;
    }

    pub fn set_blur(&mut self, val: f32) {
        self.blur = val;
    }

    pub fn set_median_filter(&mut self, val: u32) {
        self.median_filter = val;
    }

    pub fn set_max_mse(&mut self, val: f32) {
        self.max_mse = val;
    }
}

fn choose_best_action(state: &State) -> (u8, f32) {
    let mut max_q = 0.0;
    let mut best_action = None;
    for (action, &q) in state.q.iter().enumerate() {
        if q > max_q {
            best_action = Some(action as u8);
            max_q = q;
        }
    }
    if let Some(best_action) = best_action {
        println!("Chosen!: 0b{:08b} ({})", best_action, max_q);
        return (best_action, max_q);
    }
    let mut rng = rand::thread_rng();
    (rng.gen_range(0..=255), max_q)
}

fn parallel_linear_search(data: Vec<State>, target: RgbImage, max_mse: f32) -> Option<usize> {
    if data.len() < 8 {
        return None;
    }
    let data = Arc::new(data);
    let result = Arc::new(Mutex::new(None::<(usize, f32)>));
    let target = Arc::new(target);

    let chunk_size = data.len() / 8;
    let mut handles = vec![];

    for i in 0..8 {
        let data_clone = Arc::clone(&data);
        let result_clone = Arc::clone(&result);
        let target_clone = Arc::clone(&target);
        let handle = thread::spawn(move || {
            let chunk = data_clone.chunks(chunk_size).nth(i).unwrap();
            for (index, &ref state) in chunk.iter().enumerate() {
                let mse = vision::get_mse(&state.frame_abstraction, &target_clone);
                if mse < max_mse {
                    // Lock the mutex to check/update result
                    let mut result = result_clone.lock().unwrap();
                    if result.is_none() || mse < result.unwrap().1 {
                        *result = Some((i * chunk_size + index, mse));
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let result = result.lock().unwrap();
    if let Some(local_result) = *result {
        return Some(local_result.0);
    }

    None
}
