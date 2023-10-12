use image::RgbImage;
use log::warn;
use opencv::core::Mat;
use opencv::core::NORM_HAMMING;
use opencv::features2d;
use opencv::prelude::DescriptorMatcherTrait;
use opencv::types;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use super::vision;

pub struct Agent {
    states: Vec<State>,
    states_index: HashMap<(u32, u32), Vec<usize>>,
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
    x_limits: (u32, u32),
    descriptors: Mat,
    q: [f32; 256],
    next_states: HashMap<(u32, u32), Vec<usize>>,
}

impl State {
    fn new(frame_abstraction: RgbImage, x_limits: (u32, u32), descriptors: Mat) -> Self {
        Self {
            frame_abstraction,
            x_limits,
            descriptors,
            q: [0.0; 256],
            next_states: HashMap::<(u32, u32), Vec<usize>>::new(),
        }
    }
}

impl Agent {
    pub fn new() -> Self {
        Self {
            states: Vec::<State>::new(),
            states_index: HashMap::<(u32, u32), Vec<usize>>::new(),
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
        let (frame_abstraction, descriptors) = vision::get_frame_abstraction(
            &frame,
            self.red_thresholds,
            self.green_thresholds,
            self.blue_thresholds,
            self.dilate_k,
        );
        if frame_abstraction.is_none() {
            warn!("Frame abstraction not good enough");
            return 0;
        }

        let frame_abstraction = frame_abstraction.unwrap();
        let x_limits = vision::get_x_limits(&frame_abstraction);
        let descriptors = descriptors.unwrap();
        let state = State::new(frame_abstraction, x_limits, descriptors);

        // Search or Add
        let current_index: usize;
        let current_action: u8;
        let max_q: f32;
        if let Some(index) = self.search_state(&state) {
            // Existing state
            let current_state = &self.states[index];
            (current_action, max_q) = choose_best_action(current_state);
            self.number_of_revisited_states += 1;
            current_index = index;
        } else {
            // New state
            current_index = self.states.len();
            self.states.push(state);
            let mut rng = rand::thread_rng();
            current_action = rng.gen_range(0..=255);
            max_q = 0.0;
            // Index
            if let Some(vector) = self.states_index.get_mut(&x_limits) {
                vector.push(current_index);
            } else {
                self.states_index.insert(x_limits, vec![current_index]);
            }
            // Add to previous next states index
            if let Some(index) = self.previous_index {
                let next_states = &mut self.states[index].next_states;
                if let Some(vector) = next_states.get_mut(&x_limits) {
                    vector.push(current_index);
                } else {
                    next_states.insert(x_limits, vec![current_index]);
                }
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

    fn search_state(&self, state: &State) -> Option<usize> {
        // Brute force with descriptors just to try
        for (index, other_state) in self.states.iter().enumerate() {
            let orb = features2d::ORB::default().unwrap();
            let mut bf_matcher = features2d::BFMatcher::create(NORM_HAMMING, true).unwrap();
            let descriptors = &state.descriptors;
            bf_matcher.add(descriptors);
            bf_matcher.train();
            let other_descriptors = &other_state.descriptors;
            let mut matches = types::VectorOfDMatch::new();
            bf_matcher
                .match_(&other_descriptors, &mut matches, &Mat::default())
                .unwrap();

            let total_distance: f32 = matches.iter().map(|m| m.distance).sum();
            let average_distance = total_distance / matches.len() as f32;

            println!("{}", average_distance);
            if average_distance < self.max_mse {
                return Some(index);
            }
        }
        return None;

        // Search first in previous next states
        if let Some(index) = self.previous_index {
            let previous_state = &self.states[index];
            if let Some(indexes) = previous_state.next_states.get(&state.x_limits) {
                let result = self.search_state_in_index_vector(state, indexes);
                if result.is_some() {
                    return result;
                }
            }
        }
        // Search in global index
        if let Some(indexes) = self.states_index.get(&state.x_limits) {
            return self.search_state_in_index_vector(state, indexes);
        }
        None
    }

    fn search_state_in_index_vector(
        &self,
        state: &State,
        index_vector: &Vec<usize>,
    ) -> Option<usize> {
        let mut min_mse = 1.0;
        let mut best_index = 0;
        let frame_abstraction = &state.frame_abstraction;
        let x_limits = state.x_limits;
        for index in index_vector {
            let other_frame_abstraction = &self.states[*index].frame_abstraction;
            let other_x_limits = self.states[*index].x_limits;
            let mse = vision::get_mse_in_x_limits(
                &frame_abstraction,
                &other_frame_abstraction,
                x_limits,
                other_x_limits,
            );
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

    pub fn get_last_state_abstraction(&self) -> RgbImage {
        if let Some(index) = self.previous_index {
            let mut frame = self.states[index].frame_abstraction.clone();
            let x_limits = self.states[index].x_limits;
            vision::draw_x_limits(&mut frame, x_limits);
            if index < self.states.len() - 1 {
                vision::draw_border(&mut frame);
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

//#[allow(dead_code)]
//fn parallel_linear_search(data: Vec<State>, target: RgbImage, max_mse: f32) -> Option<usize> {
//    if data.len() < 8 {
//        return None;
//    }
//    let data = Arc::new(data);
//    let result = Arc::new(Mutex::new(None::<(usize, f32)>));
//    let target = Arc::new(target);
//
//    let chunk_size = data.len() / 8;
//    let mut handles = vec![];
//
//    for i in 0..8 {
//        let data_clone = Arc::clone(&data);
//        let result_clone = Arc::clone(&result);
//        let target_clone = Arc::clone(&target);
//        let handle = thread::spawn(move || {
//            let chunk = data_clone.chunks(chunk_size).nth(i).unwrap();
//            for (index, &ref state) in chunk.iter().enumerate() {
//                let mse = vision::get_mse(&state.frame_abstraction, &target_clone);
//                if mse < max_mse {
//                    // Lock the mutex to check/update result
//                    let mut result = result_clone.lock().unwrap();
//                    if result.is_none() || mse < result.unwrap().1 {
//                        *result = Some((i * chunk_size + index, mse));
//                    }
//                }
//            }
//        });
//        handles.push(handle);
//    }
//
//    for handle in handles {
//        handle.join().unwrap();
//    }
//
//    let result = result.lock().unwrap();
//    if let Some(local_result) = *result {
//        return Some(local_result.0);
//    }
//
//    None
//}
