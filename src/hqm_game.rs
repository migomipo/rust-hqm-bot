use nalgebra::{Matrix3, Point3, Vector2};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum HQMTeam {
    Red,
    Blue,
}

#[derive(Debug, Clone)]
pub struct HQMPlayerInput {
    pub stick_angle: f32,
    pub turn: f32,
    pub unknown: f32,
    pub fwbw: f32,
    pub stick: Vector2<f32>,
    pub head_rot: f32,
    pub body_rot: f32,
    pub shift_rotate: bool,
    pub crouch: bool,
    pub jump: bool,
    pub join_red: bool,
    pub join_blue: bool,
    pub spectate: bool
}

impl Default for HQMPlayerInput {
    fn default() -> Self {
        HQMPlayerInput {
            stick_angle: 0.0,
            turn: 0.0,
            unknown: 0.0,
            fwbw: 0.0,
            stick: Vector2::new(0.0, 0.0),
            head_rot: 0.0,
            body_rot: 0.0,
            shift_rotate: false,
            crouch: false,
            jump: false,
            join_red: false,
            join_blue: false,
            spectate: false
        }
    }
}

#[derive(Debug, Clone)]
pub struct HQMGameState {
    pub red_score: u32,
    pub blue_score: u32,
    pub time: u32,
    pub period: u32,
    pub goal_interruption: bool,
    pub game_over: bool,
    pub objects: Vec<HQMGameStateObject>,
    pub yourself: usize,
    pub players: HashMap<usize, HQMPlayer>,

    pub game_id: u32,
    pub step: u32
}

#[derive(Debug, Clone)]
pub enum HQMGameStateObject {
    None,
    Skater(HQMGameStateSkater),
    Puck(HQMGameStatePuck)
}

#[derive(Debug, Clone)]
pub struct HQMGameStateSkater {
    pub pos: Point3<f32>,
    pub rot: Matrix3<f32>,
    pub stick_pos: Point3<f32>,
    pub stick_rot: Matrix3<f32>,
    pub head_rot: f32,
    pub body_rot: f32,
}

#[derive(Debug, Clone)]
pub struct HQMGameStatePuck {
    pub pos: Point3<f32>,
    pub rot: Matrix3<f32>,
}

#[derive(Debug, Clone)]
pub struct HQMPlayer {
    pub name: String,
    pub index: usize,
    pub object_index: Option<(usize, HQMTeam)>,
}

#[derive(Debug, Clone)]
pub enum HQMMessage {
    PlayerUpdate {
        player_name: String,
        object: Option<(usize, HQMTeam)>,
        player_index: usize,
        in_server: bool,
    },
    Goal {
        team: HQMTeam,
        goal_player_index: Option<usize>,
        assist_player_index: Option<usize>,
    },
    Chat {
        player_index: Option<usize>,
        message: String,
    },
}