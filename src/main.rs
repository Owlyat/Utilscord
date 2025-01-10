use open_dmx::DMXSerial;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use std::env;
use std::time::Duration;

#[path = "applib/interact.rs"]
mod interact_mod;
use interact_mod::{component::Content, TabManager};
//mod ratatui_elements;
struct Utilscord {
    should_quit: bool,
    quit_key_code: Vec<KeyCode>,
    move_key_code: Vec<KeyCode>,
    _interact_key_code: KeyCode,
    tab_manager: TabManager,
}

impl Default for Utilscord {
    fn default() -> Self {
        Self {
            should_quit: false,
            quit_key_code: vec![KeyCode::Char('q'), KeyCode::Esc],
            move_key_code: vec![KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down],
            _interact_key_code: KeyCode::Enter,
            tab_manager: TabManager::default(),
        }
    }
}

impl Utilscord {
    pub fn run(&mut self) {
        let mut terminal = ratatui::init();

        while !self.should_quit {
            terminal
                .draw(|frame: &mut Frame| {
                    frame.render_stateful_widget(
                        self.tab_manager.tabs[self.tab_manager.selected_tab].clone(),
                        frame.area(),
                        &mut self.tab_manager.tabs[self.tab_manager.selected_tab].content,
                    );
                })
                .unwrap();
            self.handle_events();
            self.handle_osc();
            self.handle_dmx();
        }

        ratatui::restore();
    }

    fn handle_dmx(&mut self) {
        if let Content::DMX(v, r, g, b, adr) = &mut self.tab_manager.tabs[2].content {
            let mut dmx_value: Vec<&mut interact_mod::component::DMXInput> = vec![v, r, g, b];
            let dmx_answer = match dmx_value.iter().find(|e| e.value != 0) {
                Some(dmx_chan) => Some(dmx_chan),
                None => None,
            };
            if dmx_answer.is_none() {
                return;
            } else {
                match DMXSerial::open(match env::consts::OS {
                    "Window" => "COM3",
                    "linux" => "/dev/ttyUSB0",
                    _ => "",
                }) {
                    Ok(mut dmx) => {
                        dmx_value
                            .iter_mut()
                            .enumerate()
                            .map(|e| {
                                let dmx_channel_adress: usize = e.0.wrapping_add(**adr as usize);
                                if !e
                                    .1
                                    .title
                                    .clone()
                                    .ends_with(&format!("{}", dmx_channel_adress))
                                {
                                    let mut title = String::new();
                                    for char in e.1.title.chars() {
                                        if !matches!(
                                            char,
                                            '0' | '1'
                                                | '2'
                                                | '3'
                                                | '4'
                                                | '5'
                                                | '6'
                                                | '7'
                                                | '8'
                                                | '9'
                                        ) {
                                            title.push(char);
                                        }
                                    }
                                    e.1.title = format!("{}{}", title, dmx_channel_adress);
                                }
                                if e.1.value == 0 {
                                } else {
                                    dmx.set_channel(dmx_channel_adress, e.1.value).ok();
                                }
                            })
                            .count();
                    }
                    Err(_e) => {}
                }
            }
        }
    }

    fn handle_osc(&mut self) {
        match &mut self.tab_manager.osc_receiver {
            Some(rcvr) => match rcvr.recv_timeout(Duration::from_millis(50)) {
                Ok(packet) => match packet {
                    rosc::OscPacket::Message(osc_message) => {
                        self.tab_manager.osc_message_interaction(osc_message);
                    }
                    rosc::OscPacket::Bundle(osc_bundle) => {
                        self.tab_manager.osc_bundle_interaction(osc_bundle);
                    }
                },
                Err(_e) => {}
            },
            None => {}
        }
    }

    fn handle_events(&mut self) {
        let timeout = Duration::from_millis(50);

        if !event::poll(timeout).unwrap() {
            return;
        }

        if let Event::Key(key) = event::read().unwrap() {
            if key.kind == KeyEventKind::Press {
                // QUIT EVENT
                for keycode in self.quit_key_code.clone() {
                    if (key.code, key.modifiers) == (keycode, KeyModifiers::CONTROL) {
                        self.should_quit = true;
                    }
                }
                if (key.modifiers, key.code) == (KeyModifiers::CONTROL, KeyCode::Char('c')) {
                    self.should_quit = true;
                }

                // MOVE EVENT
                for move_key_code in self.move_key_code.clone() {
                    if key.code == move_key_code {
                        match &self.tab_manager.tabs[self.tab_manager.selected_tab].content {
                            Content::MainMenu(_sound_list, _input) => {
                                if key.modifiers == KeyModifiers::SHIFT {
                                    match key.code {
                                        KeyCode::Up => {
                                            if !self.tab_manager.tabs[0].is_used() {
                                                self.tab_manager.tabs[0].previous_content_element();
                                            }
                                        }
                                        KeyCode::Down => {
                                            if !self.tab_manager.tabs[0].is_used() {
                                                self.tab_manager.tabs[0].next_content_element();
                                            }
                                        }
                                        KeyCode::Left => {
                                            if !self.tab_manager.tabs[0].is_used() {
                                                self.tab_manager.previous()
                                            }
                                        }
                                        KeyCode::Right => {
                                            if !self.tab_manager.tabs[0].is_used() {
                                                self.tab_manager.next()
                                            }
                                        }
                                        _ => panic!("You should not be here!"),
                                    }
                                }
                            }
                            Content::OSC(_listening_ip_input, _remote_ip_input) => {
                                if key.modifiers == KeyModifiers::SHIFT {
                                    match key.code {
                                        KeyCode::Up => {
                                            if !self.tab_manager.tabs[1].is_used() {
                                                self.tab_manager.tabs[1].previous_content_element()
                                            }
                                        }
                                        KeyCode::Down => {
                                            if !self.tab_manager.tabs[1].is_used() {
                                                self.tab_manager.tabs[1].next_content_element()
                                            }
                                        }
                                        KeyCode::Left => self.tab_manager.previous(),
                                        KeyCode::Right => self.tab_manager.next(),
                                        _ => panic!("You should not be here!"),
                                    }
                                }
                            }
                            Content::DMX(..) => match key.code {
                                KeyCode::Right => {
                                    if key.modifiers == KeyModifiers::SHIFT {
                                        self.tab_manager.next()
                                    } else {
                                        self.tab_manager.tabs[2].next_content_element()
                                    }
                                }
                                KeyCode::Left => {
                                    if key.modifiers == KeyModifiers::SHIFT {
                                        self.tab_manager.previous()
                                    } else {
                                        self.tab_manager.tabs[2].previous_content_element()
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                }

                // INTERACT EVENT
                self.tab_manager.interact(key.code, key.modifiers);
            }
        }
    }
}

fn main() {
    let mut utilschord = Utilscord::default();
    utilschord.run();
}
