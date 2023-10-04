use image::RgbImage;
use rand::Rng;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

use super::vision;

pub struct Agent {
    states: Vec<State>,
    previous_index: Option<usize>,
    previous_action: Option<u8>,
    previous_q: Option<f32>,
    number_of_revisited_states: usize,
    discount_factor: f32,
    learning_rate: f32,
    red_thresholds: [u8; 2],
    green_thresholds: [u8; 2],
    blue_thresholds: [u8; 2],
    dilate_k: u8,
    max_mse: f32,
}

#[derive(Clone)]
struct State {
    frame_abstraction: RgbImage,
    char1_centroid: [u32; 2],
    char2_centroid: [u32; 2],
    blob_limits: ([u32; 2], [u32; 2]),
    q: [f32; 256],
    next_states: HashSet<usize>,
}

impl State {
    fn new(
        frame_abstraction: RgbImage,
        char1_centroid: [u32; 2],
        char2_centroid: [u32; 2],
        blob_limits: ([u32; 2], [u32; 2]),
    ) -> Self {
        Self {
            frame_abstraction,
            char1_centroid,
            char2_centroid,
            blob_limits,
            q: [0.0; 256],
            next_states: HashSet::default(),
        }
    }
}

impl Agent {
    pub fn new() -> Self {
        Self {
            states: Vec::<State>::new(),
            previous_index: None,
            previous_action: None,
            previous_q: None,
            number_of_revisited_states: 0,
            discount_factor: 0.9,
            learning_rate: 0.5,
            red_thresholds: [0, 173],
            green_thresholds: [15, 165],
            blue_thresholds: [15, 156],
            dilate_k: 6,
            max_mse: 0.012,
        }
    }

    pub fn visit_state(&mut self, frame: RgbImage, reward: f32) -> u8 {
        // We need a way to recognize equivalent states
        // This is one of the most important/challenging parts
        let frame_abstraction = vision::get_frame_abstraction(
            &frame,
            self.red_thresholds,
            self.green_thresholds,
            self.blue_thresholds,
            self.dilate_k,
        );
        if frame_abstraction.is_none() {
            return 0;
        }

        let frame_abstraction = frame_abstraction.unwrap();

        // Compute characters centroids
        let width = frame_abstraction.width();
        let half_width = (frame_abstraction.width() as f32 / 2.0) as u32;
        let char1_centroid = vision::get_centroid(&frame_abstraction, [0, half_width]);
        let char2_centroid = vision::get_centroid(&frame_abstraction, [half_width, width]);
        let blob_limits = vision::get_blob_limits(&frame_abstraction);
        let state = State::new(
            frame_abstraction,
            char1_centroid,
            char2_centroid,
            blob_limits,
        );

        // Search or Add
        let current_index: usize;
        let current_action: u8;
        let max_q: f32;
        //let mut result = self.search_on_previous_next_states(&frame_abstraction);
        //if result.is_none() {
        //    //result = parallel_linear_search(
        //        self.states.clone(),
        //        frame_abstraction.clone(),
        //        self.max_mse,
        //    );
        //}
        let result = self.search_state(&state);
        //if let Some(index) =
        //    parallel_linear_search(self.states.clone(), frame_abstraction.clone(), self.max_mse)
        //{
        if let Some(index) = result {
            // Existing state
            let current_state = &self.states[index];
            (current_action, max_q) = choose_best_action(current_state);
            self.number_of_revisited_states += 1;
            current_index = index;
        } else {
            // New state
            current_index = self.states.len();
            //self.states.push(State::new(frame_abstraction, char1_centroid, char2_centroid, blob_limits));
            self.states.push(state);
            let mut rng = rand::thread_rng();
            current_action = rng.gen_range(0..=255);
            max_q = 0.0;
            // Add to previous next states
            if let Some(index) = self.previous_index {
                self.states[index].next_states.insert(current_index);
            }
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

    fn search_on_previous_next_states(&self, target: &RgbImage) -> Option<usize> {
        if self.previous_index.is_none() {
            return None;
        }
        let mut min_mse: f32 = (1 << 16) as f32;
        let mut best_index = 0;
        let prev_index = self.previous_index.unwrap();
        for index in &self.states[prev_index].next_states {
            let frame_abstraction = &self.states[*index].frame_abstraction;
            let mse = vision::get_mse(&frame_abstraction, &target);
            // get_color_mse_foreground could be a good idea
            // Or be stricter with max_mse
            if mse < min_mse {
                min_mse = mse;
                best_index = *index;
            }
        }
        if min_mse < self.max_mse {
            return Some(best_index);
        }
        None
    }

    #[allow(dead_code)]
    fn search_state(&self, target_state: &State) -> Option<usize> {
        let mut min_mse = 1.0;
        let mut best_index = 0;
        for (index, state) in &mut self.states.iter().enumerate() {
            let (are_equal, mse) = self.are_states_equal(&state, target_state);
            if are_equal && mse < min_mse {
                min_mse = mse;
                best_index = index;
            }
            //let mse = vision::get_mse(&state.frame_abstraction, &target);
            //// get_color_mse_foreground could be a good idea
            //// Or be stricter with max_mse
            //if mse < min_mse {
            //    min_mse = mse;
            //    best_index = index;
            //}
        }
        if min_mse < self.max_mse {
            return Some(best_index);
        }
        None
    }

    fn are_states_equal(&self, state1: &State, state2: &State) -> (bool, f32) {
        let width1 = state1.blob_limits.0[1] - state1.blob_limits.0[0];
        let width2 = state2.blob_limits.0[1] - state2.blob_limits.0[0];
        let height1 = state1.blob_limits.1[1] - state1.blob_limits.1[0];
        let height2 = state2.blob_limits.1[1] - state2.blob_limits.1[0];
        if width1 == width2 && height1 == height2 {
            let mse = vision::get_mse_in_roi(
                &state1.frame_abstraction,
                &state2.frame_abstraction,
                state1.blob_limits,
                state2.blob_limits,
            );
            if mse < self.max_mse {
                return (true, mse);
            }
        }
        (false, 0.0)
    }

    pub fn get_last_state_abstraction(&self) -> RgbImage {
        //if let (Some(index), Some(q)) = (self.previous_index, self.previous_q) {
        //    let mut frame = self.states[index].frame_abstraction.clone();
        //    vision::enclose_with_q(&mut frame, q);
        //    return frame;
        //}
        if let Some(index) = self.previous_index {
            let mut frame = self.states[index].frame_abstraction.clone();
            //if index < self.states.len() - 1 {
            let char1_centroid = self.states[index].char1_centroid;
            let char2_centroid = self.states[index].char2_centroid;
            //vision::decorate_frame(&mut frame, char1_centroid, char2_centroid);
            //let distance = char2_centroid[0] - char1_centroid[0];
            //let middle = char1_centroid[0] + (distance as f32 / 2.0) as u32;
            //vision::add_sections(&mut frame, middle);
            vision::enclose_blobs(&mut frame);
            //}
            if index < self.states.len() - 1 {
                vision::enclose_with_q(&mut frame, 1.0);

            }
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

    pub fn get_number_of_previous_next_states(&self) -> usize {
        if let Some(index) = self.previous_index {
            return self.states[index].next_states.len();
        }
        0
    }

    pub fn set_red_thresholds(&mut self, val: [u8; 2]) {
        self.red_thresholds = val;
    }

    pub fn set_green_thresholds(&mut self, val: [u8; 2]) {
        self.green_thresholds = val;
    }

    pub fn set_blue_thresholds(&mut self, val: [u8; 2]) {
        self.blue_thresholds = val;
    }

    pub fn set_dilate_k(&mut self, val: u8) {
        self.dilate_k = val;
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
