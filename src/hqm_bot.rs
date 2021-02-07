use crate::hqm_parse::{HQMMessageWriter, HQMObjectPacket, HQMMessageReader, HQMSkaterPacket, HQMPuckPacket};
use crate::hqm_game::{HQMMessage, HQMPlayerInput, HQMTeam, HQMGameStateObject, HQMGameState, HQMPlayer, HQMGameStatePuck, HQMGameStateSkater};
use std::collections::HashMap;
use tokio::net::UdpSocket;
use std::sync::Arc;
use nalgebra::Point3;
use crate::hqm_parse;
use std::net::SocketAddr;
use bytes::BytesMut;

const GAME_HEADER: &[u8] = b"Hock";

pub trait HQMBotLogic {
    fn new_game(& mut self);
    fn tick(& mut self, state: &HQMGameState, messages: &[HQMMessage]) -> (HQMPlayerInput, Option<String>);
}

pub struct HQMBotSession<T: HQMBotLogic> {
    name: String,
    current_game: u32,
    known_packet: u32,
    chat_rep: u32,
    known_msgpos: u16,
    players: HashMap<usize, HQMPlayer>,
    saved_packets: HashMap<u32, Vec<HQMObjectPacket>>,
    logic: T
}

impl<T: HQMBotLogic> HQMBotSession<T> {

    pub fn new (name: String, logic: T) -> Self {
        HQMBotSession {
            name,
            current_game: u32::MAX,
            known_packet: u32::MAX,
            chat_rep: 0,
            known_msgpos: 0,
            players: HashMap::new(),
            saved_packets: HashMap::new(),
            logic
        }
    }

    pub async fn start (& mut self, server_address: SocketAddr) -> std::io::Result<()> {
        let local_addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], 0));

        let socket = Arc::new(UdpSocket::bind(local_addr).await?);
        socket.connect(server_address).await;



        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::channel(64);
        {
            let socket = socket.clone();
            tokio::spawn(async move {
                loop {
                    let mut buf = BytesMut::new();
                    buf.resize(2048, 0u8);

                    match socket.recv(&mut buf).await {
                        Ok(size) => {
                            buf.truncate(size);
                            let _ = msg_sender.send(buf.freeze()).await;
                        }
                        Err(_) => {}
                    }
                }
            });
        };


        self.send_join_message(&socket).await;
        loop {
            if let Some(x) = msg_receiver.recv().await {
                self.handle_message (x.as_ref(), &socket).await;
            }
        }
    }


    async fn handle_message (& mut self, msg: &[u8], socket: &UdpSocket) {
        let mut parser = HQMMessageReader::new(&msg);

        let header = parser.read_bytes_aligned(4);
        if header != GAME_HEADER {
            return;
        }

        let command = parser.read_byte_aligned();
        let mut input = Default::default();
        let mut chat = None;
        if command == 5 {

            let game_id = parser.read_u32_aligned();
            let step = parser.read_u32_aligned();
            let game_over = parser.read_bits(1) == 1;
            let red_score = parser.read_bits(8);
            let blue_score = parser.read_bits(8);

            let time = parser.read_bits(16);
            let goal_time = parser.read_bits(16);

            let period = parser.read_bits(8);

            let own_player_id = parser.read_bits (8) as usize;

            let packet = parser.read_u32_aligned();
            let known_packet = parser.read_u32_aligned();

            let old_packet = self.saved_packets.get(&(known_packet & 0xff));
            let mut new_packet:Vec<HQMObjectPacket> = Vec::new ();
            for i in 0..32 {

                let exists = parser.read_bits(1) == 1;
                let new_object = if exists {
                    let obj_type = parser.read_bits(2);

                    if obj_type == 0 {
                        let old_packet_object = old_packet.and_then(|x| match &x[i] {
                            HQMObjectPacket::Skater(skater) => Some(skater),
                            _ => None
                        });
                        let old_pos = old_packet_object.map (|x| x.pos);
                        let old_rot = old_packet_object.map (|x| x.rot);

                        let x = parser.read_pos(17, old_pos.map(|x| x.0));
                        let y = parser.read_pos(17, old_pos.map(|x| x.1));
                        let z = parser.read_pos(17, old_pos.map(|x| x.2));
                        let r1 = parser.read_pos(31, old_rot.map(|x| x.0));
                        let r2 = parser.read_pos(31, old_rot.map(|x| x.1));

                        let stick_x = parser.read_pos(13, old_packet_object.map(|x| x.stick_pos.0));
                        let stick_y = parser.read_pos(13, old_packet_object.map(|x| x.stick_pos.1));
                        let stick_z = parser.read_pos(13, old_packet_object.map(|x| x.stick_pos.2));

                        let stick_r1 = parser.read_pos(25, old_packet_object.map(|x| x.stick_rot.0));
                        let stick_r2 = parser.read_pos(25, old_packet_object.map(|x| x.stick_rot.1));

                        let head_rot = parser.read_pos(16, old_packet_object.map(|x| x.head_rot));
                        let body_rot = parser.read_pos(16, old_packet_object.map(|x| x.body_rot));

                        HQMObjectPacket::Skater(HQMSkaterPacket {
                            pos: (x, y, z),
                            rot: (r1, r2),
                            stick_pos: (stick_x, stick_y, stick_z),
                            stick_rot: (stick_r1, stick_r2),
                            head_rot,
                            body_rot,
                        })
                    } else if obj_type == 1 {
                        let old_packet_object = old_packet.and_then(|x| match &x[i] {
                            HQMObjectPacket::Puck(puck) => Some(puck),
                            _ => None
                        });

                        let old_pos = old_packet_object.map (|x| x.pos);
                        let old_rot = old_packet_object.map (|x| x.rot);

                        let x = parser.read_pos(17, old_pos.map(|x| x.0));
                        let y = parser.read_pos(17, old_pos.map(|x| x.1));
                        let z = parser.read_pos(17, old_pos.map(|x| x.2));
                        let r1 = parser.read_pos(31, old_rot.map(|x| x.0));
                        let r2 = parser.read_pos(31, old_rot.map(|x| x.1));

                        HQMObjectPacket::Puck(HQMPuckPacket {
                            pos: (x, y, z),
                            rot: (r1, r2),
                        })
                    } else {
                        panic!()
                    }
                } else {
                    HQMObjectPacket::None
                };
                new_packet.push(new_object);
            }

            let game_state_objects: Vec<HQMGameStateObject> = new_packet.iter().map(|x| match x {
                HQMObjectPacket::None => HQMGameStateObject::None,
                HQMObjectPacket::Puck(packet) => {
                    let pos = Point3::new(packet.pos.0 as f32 / 1024.0,
                                          packet.pos.1 as f32 / 1024.0,
                                          packet.pos.2 as f32 / 1024.0);
                    let rot = hqm_parse::convert_matrix_from_network(31, packet.rot.0, packet.rot.1);
                    HQMGameStateObject::Puck(HQMGameStatePuck {
                        pos,
                        rot
                    })
                }
                HQMObjectPacket::Skater(packet) => {
                    let pos = Point3::new(packet.pos.0 as f32 / 1024.0,
                                          packet.pos.1 as f32 / 1024.0,
                                          packet.pos.2 as f32 / 1024.0);
                    let rot = hqm_parse::convert_matrix_from_network(31, packet.rot.0, packet.rot.1);
                    let stick_pos = Point3::new(
                        (packet.stick_pos.0 as f32 / 1024.0) + pos.x - 4.0,
                        (packet.stick_pos.1 as f32 / 1024.0) + pos.y - 4.0,
                        (packet.stick_pos.2 as f32 / 1024.0) + pos.z - 4.0
                    );
                    let stick_rot = hqm_parse::convert_matrix_from_network(25, packet.stick_rot.0, packet.stick_rot.1);
                    HQMGameStateObject::Skater(HQMGameStateSkater {
                        pos,
                        rot,
                        stick_pos,
                        stick_rot,
                        head_rot: packet.head_rot as f32 / 8192.0 + 2.0,
                        body_rot: packet.body_rot as f32 / 8192.0 + 2.0
                    })
                }
            }).collect();

            let message_num = parser.read_bits (4);
            let known_msg_pos = parser.read_bits(16);
            let mut messages = vec![];
            for i in known_msg_pos..known_msg_pos+message_num {
                let message_type = parser.read_bits(6);
                if message_type == 0 {
                    // Player update
                    let player_index = parser.read_bits(6) as usize;
                    let is_online = parser.read_bits(1) == 1;
                    let team = match parser.read_bits(2) {
                        0 => Some(HQMTeam::Red),
                        1 => Some(HQMTeam::Blue),
                        _ => None
                    };
                    let object_index = match parser.read_bits(6) {
                        u32::MAX => None,
                        x => Some(x as usize)
                    };
                    let object_index = object_index.zip(team);
                    let mut bytes = vec![];
                    for _ in 0..31 {
                        bytes.push(parser.read_bits(7) as u8);
                    }
                    if let Ok(s) = String::from_utf8(bytes) {
                        let s = s.trim_matches(char::from(0)).to_string();

                        if i >= self.known_msgpos as u32 {
                            if is_online {
                                self.players.insert(player_index, HQMPlayer {
                                    name: s.clone (),
                                    index: player_index,
                                    object_index
                                });
                            } else {
                                self.players.remove(&player_index);
                            }

                            messages.push(HQMMessage::PlayerUpdate {
                                player_name: s,
                                object: object_index,
                                player_index,
                                in_server: is_online
                            });
                        }

                    }
                } else if message_type == 1 {
                    // Goal
                    let team = match parser.read_bits(2) {
                        0 => HQMTeam::Red,
                        _ => HQMTeam::Blue,
                    };
                    let goal_player_index = match parser.read_bits(6) {
                        u32::MAX => None,
                        x => Some(x as usize)
                    };
                    let assist_player_index = match parser.read_bits(6) {
                        u32::MAX => None,
                        x => Some(x as usize)
                    };
                    if i >= self.known_msgpos as u32 {
                        messages.push(HQMMessage::Goal {
                            team,
                            goal_player_index,
                            assist_player_index
                        });
                    }
                } else if message_type == 2 {
                    let player_index = match parser.read_bits(6) {
                        u32::MAX => None,
                        x => Some(x as usize)
                    };
                    let size = parser.read_bits(6);
                    let mut bytes = vec![];
                    for _ in 0..size {
                        bytes.push(parser.read_bits(7) as u8);
                    }
                    if let Ok(s) = String::from_utf8(bytes) {
                        let s = s.trim_matches(char::from(0)).to_string();
                        if i >= self.known_msgpos as u32 {
                            messages.push(HQMMessage::Chat {
                                player_index,
                                message: s
                            });
                        }
                    }
                }
            }
            let game_state = HQMGameState {
                game_id,
                step,
                red_score,
                blue_score,
                time,
                period,
                game_over,
                goal_interruption: goal_time > 0,
                objects: game_state_objects,
                yourself: own_player_id,
                players: self.players.clone(),
            };

            if self.known_msgpos < (known_msg_pos+message_num) as u16{
                self.known_msgpos = (known_msg_pos+message_num) as u16;
            }

            self.saved_packets.insert(packet & 0xff, new_packet);

            if self.known_packet == u32::MAX || self.known_packet < packet {
                self.known_packet = packet;
            }

            let (new_input, new_chat) = self.logic.tick (&game_state, &messages);
            input = new_input;
            chat = new_chat;
        } else if command == 6 {
            let game = parser.read_u32_aligned();
            if self.current_game != game {
                self.current_game = game;
                self.known_packet = u32::MAX;
                self.known_msgpos = 0;
                self.chat_rep = 0;
                self.saved_packets.clear();
                self.players.clear();
                self.logic.new_game();
            }

        }

        self.send_update (input, chat, socket).await;
    }

    async fn send_update (& mut self, input: HQMPlayerInput, chat: Option<String>, socket: &UdpSocket) -> std::io::Result<()> {
        let mut buf = [0u8;128];
        let mut writer = HQMMessageWriter::new(& mut buf);
        writer.write_bytes_aligned(GAME_HEADER);
        writer.write_byte_aligned(4);
        writer.write_u32_aligned(self.current_game);
        writer.write_f32_aligned(input.stick_angle);
        writer.write_f32_aligned(input.turn);
        writer.write_f32_aligned(input.unknown);
        writer.write_f32_aligned(input.fwbw);
        writer.write_f32_aligned(input.stick[0]);
        writer.write_f32_aligned(input.stick[1]);
        writer.write_f32_aligned(input.head_rot);
        writer.write_f32_aligned(input.body_rot);

        writer.write_u32_aligned({
            let mut x = 0;
            if input.jump {
                x |= 1;
            }
            if input.crouch {
                x |= 2;
            }
            if input.join_red {
                x |= 4;
            }
            if input.join_blue {
                x |= 8;
            }
            if input.shift_rotate {
                x |= 0x10;
            }
            if input.spectate {
                x |= 0x20;
            }
            x
        });


        writer.write_u32_aligned(self.known_packet);
        writer.write_u16_aligned (self.known_msgpos);
        if let Some(chat) = chat {
            let chat_rep = self.chat_rep;
            self.chat_rep += 1;
            self.chat_rep &= 7;
            writer.write_bits(1, 1);
            writer.write_bits(3, chat_rep);
            let bytes = chat.into_bytes();
            let size = bytes.len().min(255);
            writer.write_bits(8, size as u32);
            writer.write_bytes_aligned(&bytes[0..size])
        } else {
            writer.write_bits(1, 0);
        }


        let bytes_written = writer.get_bytes_written();

        let slice = &buf[0..bytes_written];
        let x = socket.send(slice).await?;
        Ok(())

    }

    async fn send_join_message (& self, socket: &UdpSocket) -> std::io::Result<()> {
        let mut buf = [0u8;64];
        let mut writer = HQMMessageWriter::new(& mut buf);
        writer.write_bytes_aligned(GAME_HEADER);
        writer.write_byte_aligned(2);
        writer.write_byte_aligned(55);
        let mut name_bytes = Vec::from(self.name.as_bytes());
        name_bytes.truncate(32);
        while name_bytes.len() < 32 {
            name_bytes.push(0);
        }
        writer.write_bytes_aligned(&name_bytes);
        let bytes_written = writer.get_bytes_written();

        let slice = &buf[0..bytes_written];
        let x = socket.send(slice).await?;
        Ok(())

    }
}