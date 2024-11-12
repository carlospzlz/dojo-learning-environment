use image::{Rgb, RgbImage};
use log::warn;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::vision;

pub struct Agent {
    states: Vec<State>,
    states_index: HashMap<(u32, u32), Vec<usize>>,
    number_of_states: usize,
    radius: u32,
    revisited: bool,
    previous_index: Option<usize>,
    previous_action: Option<u8>,
    previous_q: Option<f32>,
    number_of_revisited_states: usize,
    discount_factor: f32,
    learning_rate: f32,
    iteration_number: usize,
    states_per_iteration: Vec<[f64; 2]>,
    training_time: Duration,
}

struct State {
    frame_abstraction: vision::FrameAbstraction,
    q: [f32; 256],
    next_states: HashMap<(u32, u32), Vec<usize>>,
}

impl State {
    fn new(frame_abstraction: vision::FrameAbstraction) -> Self {
        Self {
            frame_abstraction,
            q: [0.0; 256],
            next_states: HashMap::<(u32, u32), Vec<usize>>::new(),
        }
    }
}

impl Agent {
    pub fn new() -> Self {
        Self {
            states: Vec::<State>::new(),
            number_of_states: 0,
            states_index: HashMap::<(u32, u32), Vec<usize>>::new(),
            radius: 30,
            revisited: false,
            previous_index: None,
            previous_action: None,
            previous_q: None,
            number_of_revisited_states: 0,
            discount_factor: 0.9,
            learning_rate: 0.5,
            iteration_number: 0,
            states_per_iteration: Vec::<[f64; 2]>::new(),
            training_time: Duration::ZERO,
        }
    }

    pub fn visit_state(
        &mut self,
        frame_abstraction: vision::FrameAbstraction,
        reward: f32,
        max_mse: f64,
    ) -> u8 {
        // We need a way to recognize equivalent states
        // This is one of the most important/challenging parts
        //if frame_abstraction.is_none() {
        //    warn!("Frame abstraction not good enough");
        //    return 0;
        //}

        let state = State::new(frame_abstraction);

        // Search or Add
        let current_index: usize;
        let current_action: u8;
        let max_q: f32;
        if let Some(index) = self.search_state(&state, max_mse) {
            // Return we are still in the same state
            if index == self.states.len() - 1 {
                return 0;
            }
            // Existing state
            let current_state = &self.states[index];
            (current_action, max_q) = choose_best_action(current_state);
            self.number_of_revisited_states += 1;
            current_index = index;
            self.revisited = true;
        } else {
            // New state
            current_index = self.states.len();
            self.states.push(state);
            let mut rng = rand::thread_rng();
            current_action = rng.gen_range(0..=255);
            max_q = 0.0;
            self.number_of_states = self.states.len();
            self.revisited = false;
            // Index
            //if let Some(vector) = self.states_index.get_mut(&x_limits) {
            //    vector.push(current_index);
            //} else {
            //    self.states_index.insert(x_limits, vec![current_index]);
            //}
            // Add to previous next states index
            //if let Some(index) = self.previous_index {
            //    let next_states = &mut self.states[index].next_states;
            //    if let Some(vector) = next_states.get_mut(&x_limits) {
            //        vector.push(current_index);
            //    } else {
            //        next_states.insert(x_limits, vec![current_index]);
            //    }
            //}
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

        let iteration_number = self.iteration_number as f64;
        let number_of_states = self.states.len() as f64;
        self.states_per_iteration
            .push([iteration_number, number_of_states]);
        self.iteration_number += 1;

        self.previous_index = Some(current_index);
        self.previous_action = Some(current_action);
        self.previous_q = Some(max_q);

        current_action
    }

    fn search_state(&self, state: &State, max_mse: f64) -> Option<usize> {
        // Search first in previous next states
        //if let Some(index) = self.previous_index {
        //    let previous_state = &self.states[index];
        //    if let Some(indexes) = previous_state.next_states.get(&state.x_limits) {
        //        let result = self.search_state_in_index_vector(state, indexes, max_mse);
        //        if result.is_some() {
        //            return result;
        //        }
        //    }
        //}
        //// Search in global index
        //if let Some(indexes) = self.states_index.get(&state.x_limits) {
        //    return self.search_state_in_index_vector(state, indexes, max_mse);
        //}
        //None

        let centroid1 = state.frame_abstraction.char1_centroid;
        let centroid2 = state.frame_abstraction.char2_centroid;
        let mut best_index = 0;
        let mut min_mse = 255.0 * 255.0;
        for (i, candidate) in self.states.iter().enumerate() {
            let candidate1 = candidate.frame_abstraction.char1_centroid;
            let candidate2 = candidate.frame_abstraction.char2_centroid;
            let distance1 = ((candidate1.0 as i32 - centroid1.0 as i32).abs()
                + (candidate1.1 as i32 - centroid1.1 as i32).abs())
                as u32;
            let distance2 = ((candidate2.0 as i32 - centroid2.0 as i32).abs()
                + (candidate2.1 as i32 - centroid2.1 as i32).abs())
                as u32;
            if distance1 < self.radius && distance2 < self.radius {
                let frame = &state.frame_abstraction.frame;
                let other_frame = &candidate.frame_abstraction.frame;
                let mse = vision::compute_mse(frame, other_frame);
                //println!("MSE {}", mse);
                if mse < min_mse {
                    best_index = i;
                    min_mse = mse;
                }
            }
        }

        if min_mse < max_mse {
            Some(best_index)
        } else {
            None
        }
    }

    //fn search_state_in_index_vector(
    //    &self,
    //    state: &State,
    //    index_vector: &Vec<usize>,
    //    max_mse: f32,
    //) -> Option<usize> {
    //    let mut min_mse = 1.0;
    //    let mut best_index = 0;
    //    let frame_abstraction = &state.frame_abstraction;
    //    let x_limits = state.x_limits;
    //    for index in index_vector {
    //        let other_frame_abstraction = &self.states[*index].frame_abstraction;
    //        let other_x_limits = self.states[*index].x_limits;
    //        let mse = vision::get_mse_in_x_limits(
    //            &frame_abstraction,
    //            &other_frame_abstraction,
    //            x_limits,
    //            other_x_limits,
    //        );
    //        if mse < min_mse {
    //            min_mse = mse;
    //            best_index = *index;
    //        }
    //    }
    //    if min_mse < max_mse {
    //        return Some(best_index);
    //    }
    //    None
    //}

    pub fn get_last_state_abstraction(&self) -> RgbImage {
        if let Some(index) = self.previous_index {
            let mut frame = self.states[index].frame_abstraction.frame.clone();
            let char1_centroid = self.states[index].frame_abstraction.char1_centroid;
            let char2_centroid = self.states[index].frame_abstraction.char2_centroid;
            vision::draw_centroid(&mut frame, char1_centroid, self.radius);
            vision::draw_centroid(&mut frame, char2_centroid, self.radius);
            if self.revisited {
                //println!("{} / {}", index, self.states.len());
                if index == (self.states.len() - 1) {
                    vision::draw_border(&mut frame, Rgb([128, 0, 0]));
                } else {
                    vision::draw_border(&mut frame, Rgb([128, 128, 0]));
                }
            }
            return frame;
        }
        RgbImage::default()
    }

    pub fn get_iteration_number(&self) -> usize {
        self.iteration_number
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

    pub fn set_radius(&mut self, radius: u32) {
        self.radius = radius;
    }

    pub fn get_states_per_iteration(&self) -> Vec<[f64; 2]> {
        return self.states_per_iteration.clone();
    }

    pub fn add_training_time(&mut self, training_time: Duration) {
        self.training_time += training_time;
    }

    pub fn get_training_time(&self) -> Duration {
        // Don't we need clone here?
        self.training_time
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

#[derive(Serialize, Deserialize)]
struct SerDesAgent {
    number_of_states: usize,
    iteration_number: usize,
    training_time: Duration,
}

impl SerDesAgent {
    pub fn new(agent: &Agent) -> Self {
        Self {
            number_of_states: agent.number_of_states,
            iteration_number: agent.iteration_number,
            training_time: agent.training_time,
        }
    }
}

pub fn save_agent(agent: &Agent, path: &str) {
    println!("Saving agent to {}...", path);

    let agent_path = Path::new(path);

    if !agent_path.exists() {
        let _ = fs::create_dir_all(agent_path);
    } else {
        println!("Path already exists: {}", path);
        return;
    }

    // Serializable data from agent
    let agent_file = fs::File::create(agent_path.join("agent.json")).unwrap();
    let ser_des_agent = SerDesAgent::new(agent);
    let _ = serde_json::to_writer_pretty(agent_file, &ser_des_agent);

    // States
    let states_path = agent_path.join("states");
    let _ = fs::create_dir_all(states_path.clone());
    let mut data = fs::File::create(states_path.join("data.csv")).unwrap();
    for (i, state) in agent.states.iter().enumerate() {
        // Frame
        let frame_path = states_path.join(format!("{:06}.png", i));
        state
            .frame_abstraction
            .frame
            .save(frame_path.clone())
            .expect("Failed to save frame");

        // Q
        let q_path = states_path.join(format!("{:06}_q.csv", i));
        let mut q_file = fs::File::create(q_path.clone()).unwrap();
        for q in state.q.iter() {
            writeln!(q_file, "{}", q);
        }

        // Data
        writeln!(
            data,
            "{},{},{},{},{},{}",
            frame_path.file_name().unwrap().to_string_lossy(),
            state.frame_abstraction.char1_centroid.0,
            state.frame_abstraction.char1_centroid.1,
            state.frame_abstraction.char2_centroid.0,
            state.frame_abstraction.char2_centroid.1,
            q_path.file_name().unwrap().to_string_lossy(),
        );
    }

    // States per iteration
    let mut states_per_iteration_file =
        fs::File::create(agent_path.join("states_per_iteration.csv")).unwrap();
    for values in agent.states_per_iteration.iter() {
        writeln!(states_per_iteration_file, "{}, {}", values[0], values[1]);
    }
}

pub fn load_agent(path: &str) -> Agent {
    println!("Loading agent from {}...", path);

    let agent_path = Path::new(path);

    if !agent_path.exists() {
        println!("Path doesn't exist: {}", path);
        return Agent::new();
    }

    // Deserializable data to agent
    let agent_file = fs::File::open(agent_path.join("agent.json")).unwrap();
    let reader = BufReader::new(agent_file);
    let ser_des_agent: SerDesAgent = serde_json::from_reader(reader).unwrap();

    // Read states
    let mut states = Vec::<State>::new();
    let states_path = agent_path.join("states");
    let mut data = fs::File::open(states_path.join("data.csv")).unwrap();
    let reader = BufReader::new(data);
    for line in reader.lines() {
        let line = line.unwrap();
        let tokens: Vec<&str> = line.split(',').collect();

        // Frame abstraction
        let frame_path = states_path.join(tokens[0].to_string());
        let frame = image::open(&frame_path).unwrap().to_rgb8();
        let char1_centroid: (u32, u32) = (
            tokens[1].trim().parse().unwrap(),
            tokens[2].trim().parse().unwrap(),
        );
        let char2_centroid: (u32, u32) = (
            tokens[3].trim().parse().unwrap(),
            tokens[4].trim().parse().unwrap(),
        );
        let frame_abstraction =
            vision::FrameAbstraction::new(frame, char1_centroid, char2_centroid);

        let mut state = State::new(frame_abstraction);

        // Q
        let q_path = states_path.join(tokens[5].to_string());
        let q_file = fs::File::open(q_path).unwrap();
        let reader = BufReader::new(q_file);
        for (i, line) in reader.lines().enumerate() {
            let line = line.unwrap();
            let value: f32 = line.trim().parse().unwrap();
            state.q[i] = value;
        }

        states.push(state);
    }

    // States per iteration
    let mut states_per_iteration = Vec::<[f64; 2]>::new();
    let states_per_iteration_file =
        fs::File::open(agent_path.join("states_per_iteration.csv")).unwrap();
    let reader = BufReader::new(states_per_iteration_file);
    for line in reader.lines() {
        let line = line.unwrap();
        let tokens: Vec<&str> = line.split(',').collect();
        let iteration_number: f64 = tokens[0].trim().parse().unwrap();
        let number_of_states: f64 = tokens[1].trim().parse().unwrap();
        states_per_iteration.push([iteration_number, number_of_states]);
    }

    // Build agent
    let mut agent = Agent::new();
    agent.number_of_states = ser_des_agent.number_of_states;
    agent.iteration_number = ser_des_agent.iteration_number;
    agent.training_time = ser_des_agent.training_time;
    agent.states = states;
    agent.states_per_iteration = states_per_iteration;

    agent
}
